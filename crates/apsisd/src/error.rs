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

use figment;
use mainline::errors::DecodeIdError;
use opentelemetry_otlp;
use reqwest::Error as ReqwestError;
use rocksdb::Error as RocksDBError;
use std::array::TryFromSliceError;
use std::io;
use thiserror::Error;
use thiserror_ext::Box;

#[derive(Debug, Error, Box)]
#[thiserror_ext(newtype(name = ApsisError))]
pub enum ApsisErrorKind {
    #[error("Block not found: `{0}`")]
    BlockNotFound(String),
    #[error("Directory error: `{0}`")]
    Directory(String),
    #[error("Figment error: `{0}`")]
    Figment(#[from] figment::Error),
    #[error("Mainline ID error: `{0}`")]
    MainlineId(#[from] DecodeIdError),
    #[error("I/O error: `{0}`")]
    Io(#[from] io::Error),
    #[error("OpenTelemtry build error: `{0}`")]
    OpenTelemetry(#[from] opentelemetry_otlp::ExporterBuildError),
    #[error("Reqwest error: `{0}`")]
    Reqwest(#[from] ReqwestError),
    #[error("RocksDB error: `{0}`")]
    RocksDB(#[from] RocksDBError),
    #[error("TryFromSliceError: `{0}`")]
    TryFromSliceError(#[from] TryFromSliceError),
}

pub type Result<T> = std::result::Result<T, ApsisError>;
