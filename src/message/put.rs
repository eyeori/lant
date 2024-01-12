use std::fmt::Debug;
use std::mem::size_of;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use bytes::Bytes;
use serde::{Deserialize, Serialize};

use crate::message::{
    FromMessagePayloadRef, JsonPayload, MessagePayload, MessagePayloadRef, ToMessagePayload,
};
use crate::utils::file::FileChunkSize;
use crate::utils::json::ToJsonString;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PutRequestMeta {
    pub file_name: PathBuf,
    pub remote_dir: PathBuf,
    pub curr_trans_trunk_index: u64,
    pub is_done: bool,
}

impl PutRequestMeta {
    pub fn new(file_name: impl Into<PathBuf>, remote_dir: impl Into<PathBuf>) -> Self {
        Self {
            file_name: file_name.into(),
            remote_dir: remote_dir.into(),
            curr_trans_trunk_index: 0,
            is_done: false,
        }
    }
}

pub struct PutRequestPayload {
    pub meta: PutRequestMeta,
    pub data: Option<Bytes>,
}

impl PutRequestPayload {
    pub fn new(meta: PutRequestMeta, data: Option<Bytes>) -> Self {
        Self { meta, data }
    }
}

impl ToMessagePayload for PutRequestPayload {
    fn to_payload(self) -> MessagePayload {
        let mut chunked_payload = Vec::new();
        let meta_json = self.meta.to_json();
        let mut buffer = Vec::from(meta_json.len().to_le_bytes());
        buffer.extend(meta_json.into_bytes());
        chunked_payload.push(Bytes::from(buffer));
        if let Some(data) = self.data {
            chunked_payload.push(data);
        }
        chunked_payload
    }
}

pub struct PutRequestPayloadRef<'a> {
    pub meta: PutRequestMeta,
    pub data: &'a [u8],
}

impl<'a> PutRequestPayloadRef<'a> {
    pub fn new(meta: PutRequestMeta, data: &'a [u8]) -> Self {
        Self { meta, data }
    }
}

impl<'a> FromMessagePayloadRef<'a> for PutRequestPayloadRef<'a> {
    fn from_payload(payload: MessagePayloadRef<'a>) -> Result<Self> {
        // meta size
        let size_of_meta_size = size_of::<usize>();
        if payload.len() < size_of_meta_size {
            return Err(anyhow!("payload size error"));
        }
        let meta_size_bytes = &payload[..size_of_meta_size];
        let meta_size = usize::from_le_bytes(meta_size_bytes.try_into().unwrap());

        // meta
        if payload.len() < size_of_meta_size + meta_size {
            return Err(anyhow!("payload size error"));
        }
        let meta_offset = size_of_meta_size;
        let meta_bytes = &payload[meta_offset..meta_offset + meta_size];
        let meta = serde_json::from_slice::<PutRequestMeta>(meta_bytes)?;

        // data
        let data_offset = meta_offset + meta_size;
        let data = &payload[data_offset..];

        Ok(Self::new(meta, data))
    }
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct PutResponsePayload {
    pub remote_file_chunk_size: FileChunkSize,
    pub is_done: bool,
}

impl PutResponsePayload {
    pub fn new(remote_file_chunk_size: FileChunkSize) -> Self {
        Self {
            remote_file_chunk_size,
            is_done: false,
        }
    }

    pub fn finish() -> Self {
        Self {
            remote_file_chunk_size: Default::default(),
            is_done: true,
        }
    }
}

impl JsonPayload for PutResponsePayload {}
