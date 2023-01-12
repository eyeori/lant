use std::fmt::Debug;
use std::mem::size_of;
use std::path::PathBuf;
use std::usize;

use anyhow::{anyhow, Result};
use bytes::Bytes;
use serde::{Deserialize, Serialize};

use crate::message::{
    FromMessagePayloadRef, JsonPayload, MessagePayload, MessagePayloadRef, ToMessagePayload,
};
use crate::utils::file::FileChunkSize;

#[derive(Serialize, Deserialize, Debug)]
pub struct GetRequestPayload {
    pub remote_file_path: PathBuf,
    pub local_file_chunk_size: FileChunkSize,
}

impl GetRequestPayload {
    pub fn new(remote_file_path: PathBuf, local_file_chunk_size: FileChunkSize) -> Self {
        Self {
            remote_file_path,
            local_file_chunk_size,
        }
    }
}

impl JsonPayload for GetRequestPayload {}

#[derive(Default, Debug)]
pub struct GetResponseMeta {
    pub remote_file_chunk_size: FileChunkSize,
    pub curr_trans_trunk_index: u64,
}

impl GetResponseMeta {
    pub fn new(remote_file_chunk_size: FileChunkSize, curr_trans_trunk_index: u64) -> Self {
        Self {
            remote_file_chunk_size,
            curr_trans_trunk_index,
        }
    }
}

impl From<GetResponseMeta> for Bytes {
    fn from(value: GetResponseMeta) -> Self {
        let mut buffer = Vec::new();
        buffer.extend(value.remote_file_chunk_size.total_chunks().to_le_bytes());
        buffer.extend(value.remote_file_chunk_size.rest_size().to_le_bytes());
        buffer.extend(value.curr_trans_trunk_index.to_le_bytes());
        Bytes::from(buffer)
    }
}

impl From<&[u8]> for GetResponseMeta {
    fn from(value: &[u8]) -> Self {
        let (total_chunk_bytes, rest) = value.split_at(size_of::<u64>());
        let total_chunk = u64::from_le_bytes(total_chunk_bytes.try_into().unwrap());

        let (rest_size_bytes, curr_trans_trunk_index_bytes) = rest.split_at(size_of::<usize>());
        let rest_size = usize::from_le_bytes(rest_size_bytes.try_into().unwrap());
        let curr_trans_trunk_index =
            u64::from_le_bytes(curr_trans_trunk_index_bytes.try_into().unwrap());

        let remote_file_chunk_size = FileChunkSize::new(total_chunk, rest_size);
        Self::new(remote_file_chunk_size, curr_trans_trunk_index)
    }
}

pub struct GetResponsePayload {
    pub meta: GetResponseMeta,
    pub data: Bytes,
}

impl GetResponsePayload {
    pub fn new(meta: GetResponseMeta, data: Bytes) -> Self {
        Self { meta, data }
    }
}

impl ToMessagePayload for GetResponsePayload {
    fn to_payload(self) -> MessagePayload {
        vec![self.meta.into(), self.data]
    }
}

pub struct GetResponsePayloadRef<'a> {
    pub meta: GetResponseMeta,
    pub data: &'a [u8],
}

impl<'a> GetResponsePayloadRef<'a> {
    pub fn new(meta: GetResponseMeta, data: &'a [u8]) -> Self {
        Self { meta, data }
    }
}

impl<'a> FromMessagePayloadRef<'a> for GetResponsePayloadRef<'a> {
    fn from_payload(payload: MessagePayloadRef<'a>) -> Result<Self> {
        // meta
        let size_of_meta = size_of::<GetResponseMeta>();
        if payload.len() < size_of_meta {
            return Err(anyhow!("payload size error"));
        }
        let meta_bytes = &payload[..size_of_meta];
        let meta = GetResponseMeta::from(meta_bytes);

        // data
        let data = &payload[size_of_meta..];

        Ok(Self::new(meta, data))
    }
}
