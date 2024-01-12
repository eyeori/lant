use std::fs::File;
use std::os::unix::fs::FileExt;

use anyhow::{anyhow, Result};
use bytes::Bytes;

use crate::message::get::{GetRequestPayload, GetResponseMeta, GetResponsePayload};
use crate::message::{
    build_message, FromMessagePayloadRef, MessagePayloadRef, MessageType, SendMessage,
};
use crate::server::get_server_abs_root_dir;
use crate::utils::file::{buffer_size, index_offset, FileChunkSize};
use crate::utils::res::ExtResult;

pub async fn request(req_payload: MessagePayloadRef<'_>) -> Result<SendMessage> {
    // deserialize request payload
    let req_payload = GetRequestPayload::from_payload(req_payload)?;

    // get abs root dir
    let abs_root_dir = get_server_abs_root_dir()?;

    // check file path valid
    let mut file_path_valid = true;
    let mut abs_file_path = abs_root_dir.clone();
    abs_file_path.push(req_payload.remote_file_path.clone());
    abs_file_path = abs_file_path.canonicalize()?;
    if !abs_file_path
        .to_str()
        .ok()?
        .starts_with(abs_root_dir.to_str().ok()?)
    {
        file_path_valid = false;
    }
    if !abs_file_path.exists() || !abs_file_path.is_file() {
        file_path_valid = false;
    }

    // build response payload
    let res_payload = if file_path_valid {
        let file = File::open(&abs_file_path)?;
        let file_len = file.metadata()?.len();
        let file_chunked_size = FileChunkSize::from(file_len as usize);
        let local_file_chunked_size = req_payload.local_file_chunk_size;
        if local_file_chunked_size != file_chunked_size {
            let mut curr_tarns_trunk_index = local_file_chunked_size.total_chunks();
            if local_file_chunked_size.rest_size() != 0 {
                curr_tarns_trunk_index -= 1;
            }
            let meta = GetResponseMeta::new(file_chunked_size, curr_tarns_trunk_index);
            let offset = index_offset(curr_tarns_trunk_index);
            let buffer_size = buffer_size((file_len - offset) as usize);
            let mut buffer = vec![0; buffer_size];
            let _ = file.read_at(&mut buffer, offset)?;
            GetResponsePayload::new(meta, Bytes::from(buffer))
        } else {
            return Err(anyhow!(
                "the file transfer is already completed, path={:?}",
                req_payload.remote_file_path
            ));
        }
    } else {
        return Err(anyhow!(
            "file not exists, path={:?}",
            req_payload.remote_file_path
        ));
    };

    // build response message
    Ok(build_message(MessageType::GetResponse, res_payload))
}
