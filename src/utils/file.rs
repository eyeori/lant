use std::cmp::min;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

const FILE_CHUNK_UNIT_SIZE: usize = 4096 * 4096;

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Default, Debug)]
pub struct FileChunkSize(u64, usize);

impl FileChunkSize {
    pub fn new(total_chunk: u64, rest_size: usize) -> Self {
        Self(total_chunk, rest_size)
    }

    pub fn total_size(&self) -> usize {
        self.integer_size() + self.rest_size()
    }

    pub fn integer_size(&self) -> usize {
        self.integer_chunks() as usize * FILE_CHUNK_UNIT_SIZE
    }

    pub fn rest_size(&self) -> usize {
        self.1
    }

    pub fn total_chunks(&self) -> u64 {
        self.0
    }

    pub fn integer_chunks(&self) -> u64 {
        self.0 - if self.0 > 0 && self.1 > 0 { 1 } else { 0 }
    }

    pub fn last_chunk_index(&self) -> u64 {
        self.0 - if self.0 > 0 { 1 } else { 0 }
    }

    pub fn last_chunk_size(&self) -> usize {
        let last_chunk_index = self.last_chunk_index();
        if last_chunk_index == 0 {
            self.rest_size()
        } else {
            self.total_size() - (last_chunk_index - 1) as usize * FILE_CHUNK_UNIT_SIZE
        }
    }
}

impl From<usize> for FileChunkSize {
    fn from(len: usize) -> Self {
        let mut total_chunks = (len / FILE_CHUNK_UNIT_SIZE) as u64;
        let rest_size = len % FILE_CHUNK_UNIT_SIZE;
        if rest_size != 0 {
            total_chunks += 1;
        }
        Self(total_chunks, rest_size)
    }
}

pub fn get_file_chunk_size<P: AsRef<Path>>(file_path: P) -> FileChunkSize {
    fs::metadata(&file_path).map_or(Default::default(), |meta| {
        FileChunkSize::from(meta.len() as usize)
    })
}

pub fn buffer_size(buffer_size: usize) -> usize {
    min(buffer_size, FILE_CHUNK_UNIT_SIZE)
}

pub fn index_offset(index: u64) -> u64 {
    index * FILE_CHUNK_UNIT_SIZE as u64
}
