use std::fs::File;
use std::os::unix::fs::FileExt;

use anyhow::{anyhow, Result};

use crate::message::put::{PutRequestPayloadRef, PutResponsePayload};
use crate::message::{
    build_message, FromMessagePayloadRef, MessagePayloadRef, MessageTypeEnum, SendMessage,
};
use crate::server::get_server_abs_root_dir;
use crate::utils::file::{index_offset, FileChunkSize};

pub async fn request(req_payload: MessagePayloadRef<'_>) -> Result<SendMessage> {
    // deserialize request payload
    let req_payload = PutRequestPayloadRef::from_payload(req_payload)?;

    // get abs root dir
    let abs_root_dir = get_server_abs_root_dir()?;

    if req_payload.meta.is_done {
        return Ok(build_message(
            MessageTypeEnum::PutResponse,
            PutResponsePayload::finish(),
        ));
    }

    // build file path
    let mut remote_file_path = abs_root_dir.clone();
    remote_file_path.push(req_payload.meta.remote_dir.clone());
    remote_file_path = remote_file_path.canonicalize()?;
    if !remote_file_path
        .to_str()
        .ok_or(anyhow!(""))?
        .starts_with(abs_root_dir.to_str().ok_or(anyhow!(""))?)
    {
        remote_file_path = abs_root_dir;
    }
    remote_file_path.push(req_payload.meta.file_name.clone());

    // open & create file
    let remote_file = File::options()
        .create(true)
        .append(true)
        .open(remote_file_path)?;

    // store data
    if !req_payload.data.is_empty() {
        // append data
        let offset = index_offset(req_payload.meta.curr_trans_trunk_index);
        remote_file.write_all_at(req_payload.data, offset)?;
    }

    // build response payload
    let remote_file_chunk_size = FileChunkSize::from(remote_file.metadata()?.len() as usize);
    let res_payload = PutResponsePayload::new(remote_file_chunk_size);

    // build response message
    Ok(build_message(MessageTypeEnum::PutResponse, res_payload))
}
