// Apsis
// Copyright (C) 2025 Throneless Tech

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.

// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use axum::{
    RequestExt,
    body::Bytes,
    debug_handler,
    extract::{FromRequest, Json, Multipart, Request, State},
    http::{
        HeaderMap, StatusCode,
        header::{ACCEPT, CONTENT_TYPE},
    },
    response::{IntoResponse, Response},
};
use bytes::{Buf, BufMut, BytesMut};
use eris_rs::{
    decode::decode,
    encode::encode,
    types::{BlockSize, BlockStorageError, BlockWithReference, ReadCapability, Reference},
};
use mainline::Dht;
use rand::prelude::*;
use rand_chacha::ChaCha20Rng;
use serde_json::Value;
use std::io;
use std::sync::Arc;
use tokio_util::task::TaskTracker;

use crate::db::Db;
use crate::utils;

#[derive(Clone)]
pub struct ApiState {
    pub auth: String,
    pub dht: Arc<Dht>,
    pub rng: ChaCha20Rng,
    pub store: Db,
    pub tracker: TaskTracker,
}

pub enum Content {
    Json(Value),
    File(Multipart),
}

impl<S> FromRequest<S> for Content
where
    Bytes: FromRequest<S>,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        let headers = req.headers();
        let content_type = headers
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok());
        match content_type {
            Some(content_type) if content_type.starts_with("application/json") => {
                let Json(body) = req
                    .extract::<Json<Value>, _>()
                    .await
                    .map_err(|err| err.into_response())?;
                Ok(Self::Json(body))
            }
            Some(content_type) if content_type.starts_with("multipart/form-data") => {
                let body = req
                    .extract::<Multipart, _>()
                    .await
                    .map_err(|err| err.into_response())?;
                Ok(Self::File(body))
            }
            _ => Err((StatusCode::UNSUPPORTED_MEDIA_TYPE).into_response()),
        }
    }
}

pub struct DynamicQuery(String);

impl<S> FromRequest<S> for DynamicQuery
where
    Bytes: FromRequest<S>,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        if let Some(query) = req.uri().query() {
            Ok(Self(query.to_owned()))
        } else {
            Err(StatusCode::NOT_FOUND.into_response())
        }
    }
}

#[debug_handler]
pub async fn resource_to_name(
    State(mut state): State<ApiState>,
    body: Content,
) -> impl IntoResponse {
    match body {
        Content::Json(json) => {
            let mut key = [0u8; 32];
            state.rng.fill_bytes(&mut key);
            let write_block = move |block: BlockWithReference| -> Result<usize, BlockStorageError> {
                let res = state
                    .store
                    .write_block(block.reference, block.block)
                    .map_err(|_err| io::Error::other("Failed to write block to database."));
                let id = utils::try_ref_to_id(&block.reference)
                    .map_err(|err| io::Error::other(err.to_string()))?;
                let dht = state.dht.clone();
                state.tracker.spawn(async move {
                    let _ = dht
                        .announce_peer(id, None)
                        .map_err(|_err| io::Error::other("Failed to announce block peer."));
                });

                res
            };
            let bytes = json.to_string();
            let block_size = if bytes.as_bytes().len() < 1000 {
                BlockSize::Size1KiB
            } else {
                BlockSize::Size32KiB
            };
            match encode(&mut bytes.as_bytes(), &key, block_size, &write_block) {
                Ok(capability) => (StatusCode::CREATED, capability.to_urn()),
                Err(err) => (StatusCode::UNPROCESSABLE_ENTITY, err.to_string()),
            }
        }
        Content::File(mut multipart) => {
            let mut key = [0u8; 32];
            state.rng.fill_bytes(&mut key);
            let write_block = move |block: BlockWithReference| -> Result<usize, BlockStorageError> {
                let res = state
                    .store
                    .write_block(block.reference, block.block)
                    .map_err(|_err| io::Error::other("Failed to write block to database."));
                let id = utils::try_ref_to_id(&block.reference)
                    .map_err(|err| io::Error::other(err.to_string()))?;
                let dht = state.dht.clone();
                state.tracker.spawn(async move {
                    let _ = dht
                        .announce_peer(id, None)
                        .map_err(|_err| io::Error::other("Failed to announce block peer."));
                });
                res
            };

            if let Ok(Some(field)) = multipart.next_field().await {
                if let Ok(bytes) = field.bytes().await {
                    if let Ok(capability) =
                        encode(&mut bytes.reader(), &key, BlockSize::Size1KiB, &write_block)
                    {
                        (StatusCode::CREATED, capability.to_urn())
                    } else {
                        (
                            StatusCode::UNPROCESSABLE_ENTITY,
                            "Failed to create capability.".to_owned(),
                        )
                    }
                } else {
                    (
                        StatusCode::UNPROCESSABLE_ENTITY,
                        "Failed to extract bytes from multipart files.".to_owned(),
                    )
                }
            } else {
                (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "Failed to read file.".to_owned(),
                )
            }
        }
    }
}

#[debug_handler]
pub async fn name_to_resource(
    State(state): State<ApiState>,
    headers: HeaderMap,
    DynamicQuery(query): DynamicQuery,
) -> impl IntoResponse {
    let read_block = move |reference: Reference| -> Result<Vec<u8>, BlockStorageError> {
        if let Some(block) = state
            .store
            .read_block(reference)
            .map_err(|_err| io::Error::other("Failed to read block from database."))?
        {
            Ok(block)
        } else {
            utils::fetch_block(reference, &state.dht, true)
                .map_err(|_err| io::Error::other("Failed to fetch block."))
        }
    };
    if let Some(capability) = ReadCapability::from_urn(query.clone()) {
        let mut buf = BytesMut::new().writer();
        if let Ok(_size) = decode(capability, &mut buf, &read_block) {
            let buf = buf.into_inner();
            match headers.get(ACCEPT) {
                Some(accept) if accept == "application/json" => {
                    if let Ok(json) = serde_json::from_slice::<Value>(&buf) {
                        Json(json).into_response()
                    } else {
                        (
                            StatusCode::UNPROCESSABLE_ENTITY,
                            "Entity is not JSON".to_owned(),
                        )
                            .into_response()
                    }
                }
                Some(accept) if accept == "application/octet-stream" => buf.into_response(),
                _ => (
                    StatusCode::NOT_FOUND,
                    "Failed to fetch capability.".to_owned(),
                )
                    .into_response(),
            }
        } else {
            (
                StatusCode::NOT_FOUND,
                "Failed to dereference capability.".to_owned(),
            )
                .into_response()
        }
    } else if let Some(reference) = utils::urn_to_ref(query) {
        if let Ok(block) = read_block(reference) {
            block.into_response()
        } else {
            (StatusCode::NOT_FOUND, "Failed to fetch block.".to_owned()).into_response()
        }
    } else {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            "Invalid capability.".to_owned(),
        )
            .into_response()
    }
}
