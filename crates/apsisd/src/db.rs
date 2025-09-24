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

use rocksdb::DB;
use std::path::PathBuf;
use std::sync::Arc;

use crate::error::Result;

#[derive(Clone)]
pub(crate) struct Db {
    inner: Arc<DB>,
}

impl Db {
    pub fn try_open(path: &PathBuf) -> Result<Self> {
        Ok(Self {
            inner: Arc::new(DB::open_default(path)?),
        })
    }

    pub fn write_block(&self, reference: [u8; 32], block: Vec<u8>) -> Result<usize> {
        let length = block.len();
        self.inner.put(reference, block)?;
        Ok(length)
    }

    pub fn read_block(&self, reference: [u8; 32]) -> Result<Option<Vec<u8>>> {
        self.inner.get(reference).map_err(|err| err.into())
    }
}
