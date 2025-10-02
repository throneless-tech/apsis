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

use std::net::SocketAddrV4;

use base32;
use blake2b_simd::Params;
use eris_rs::types::Reference;
use mainline::{Dht, Id, errors::DecodeIdError};
use reqwest;

use crate::error::{ApsisErrorKind, Result};

const MAX_PEER_RETRIES: usize = 3;

pub fn try_ref_to_id(reference: &Reference) -> Result<Id> {
    let id = Id::from_bytes(&reference[..20]).map_err(|err| DecodeIdError::InvalidIdSize(err))?;
    Ok(id)
}

pub fn urn_to_ref(urn: String) -> Option<Reference> {
    let base32_alphabet = base32::Alphabet::Rfc4648 { padding: false };
    match urn.split_once("urn:") {
        Some((_, reference_base32)) => match base32::decode(base32_alphabet, reference_base32) {
            Some(bytes) => bytes.try_into().ok(),
            None => None,
        },
        None => None,
    }
}

fn ref_to_urn(reference: &Reference) -> String {
    let base32_alphabet = base32::Alphabet::Rfc4648 { padding: false };
    let block_ref = base32::encode(base32_alphabet, reference);
    "urn:".to_owned() + &block_ref
}

fn peer_to_url(peer: SocketAddrV4, block: &Reference) -> String {
    format!(
        "http://{}:{}/uri-res/N2R?{}",
        peer.ip(),
        peer.port(),
        ref_to_urn(block)
    )
}

fn blake2b256_hash(input: &[u8], key: Option<&[u8]>) -> Reference {
    let mut hasher = match key {
        Some(k) => Params::new().hash_length(32).key(k).to_state(),
        None => Params::new().hash_length(32).to_state(),
    };
    hasher.update(input);
    let mut result: Reference = Default::default();
    result.copy_from_slice(hasher.finalize().as_bytes());
    result
}

pub fn fetch_block(reference: [u8; 32], dht: &Dht, check: bool) -> Result<Vec<u8>> {
    if !dht.bootstrapped() {
        return Err(ApsisErrorKind::BlockNotFound("DHT failed to bootstrap.".to_owned()).into());
    }

    let id = try_ref_to_id(&reference)?;
    let client = reqwest::blocking::Client::new();

    let mut tries = 0;
    while tries < MAX_PEER_RETRIES {
        let subset = dht.get_peers(id);
        for peers in subset {
            for peer in peers {
                let candidate = client.get(peer_to_url(peer, &reference)).send()?.bytes()?;
                if check {
                    let hash = blake2b256_hash(candidate.as_ref(), None);
                    if hash != reference {
                        continue;
                    }
                }
                return Ok(candidate.into());
            }
        }
        tries += 1;
    }

    Err(ApsisErrorKind::BlockNotFound("Failed to fetch valid block.".to_owned()).into())
}
