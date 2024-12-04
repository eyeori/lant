use std::fs::File;
use std::os::unix::fs::FileExt;
use std::path::Path;

use bytes::Bytes;
use chrono::Local;
use quinn::VarInt;

use crate::client::{send_and_receive, unwrap_message, Stage};
use crate::message::put::{PutRequestMeta, PutRequestPayload, PutResponsePayload};
use crate::message::{FromMessagePayloadRef, MessagePayloadRef, MessageType};
use crate::utils::error::{MsgErr, Result};
use crate::utils::file::{buffer_size, index_offset, FileChunkSize};

pub async fn put(connecting: quinn::Connecting, file_path: &Path, remote_dir: &Path) {
    if let Err(e) = process_request(connecting, file_path, remote_dir).await {
        println!("[ERR][Client] Process request error, error={e}");
    }
}

async fn process_request(
    mut connecting: quinn::Connecting,
    file_path: &Path,
    remote_dir: &Path,
) -> Result<()> {
    println!(
        "put file: {file_path:?}, to remote dir: {remote_dir:?}, time:{}",
        Local::now().timestamp_millis()
    );
    let local_file = File::open(file_path)?;
    let local_file_len = local_file.metadata()?.len();
    let local_file_chunk_size = FileChunkSize::from(local_file_len as usize);

    let file_name = file_path
        .file_name()
        .ok_or(MsgErr::new("got file name error"))?;
    let mut req_meta = PutRequestMeta::new(file_name, remote_dir);
    let mut req_payload = PutRequestPayload::new(req_meta.clone(), None);

    let conn = (&mut connecting).await?;
    loop {
        // do request
        println!(">>>: {:?}", req_payload.meta);
        let response = send_and_receive(&conn, MessageType::PutRequest, req_payload).await?;

        // process response
        if let Some(res_payload) = unwrap_message(&response, MessageType::PutResponse)? {
            let stage = process_response(
                &local_file,
                local_file_len as usize,
                &local_file_chunk_size,
                res_payload,
            )?;
            match stage {
                Stage::Processing((is_done, curr_tarns_trunk_index, data)) => {
                    req_meta.curr_trans_trunk_index = curr_tarns_trunk_index;
                    req_meta.is_done = is_done;
                    req_payload = PutRequestPayload::new(req_meta.clone(), data);
                }
                Stage::Finish => break,
            }
        } else {
            break;
        }
    }
    conn.close(VarInt::from(200u32), "OK".as_bytes());

    println!(
        "put file: {file_path:?}, to remote dir: {remote_dir:?} finish, time:{:?}",
        Local::now().timestamp_millis()
    );

    Ok(())
}

fn process_response(
    local_file: &File,
    local_file_len: usize,
    local_file_chunk_size: &FileChunkSize,
    payload: MessagePayloadRef,
) -> Result<Stage<(bool, u64, Option<Bytes>)>> {
    // get payload
    let payload = PutResponsePayload::from_payload(payload)?;
    println!("<<<: {payload:?}");

    if payload.is_done {
        return Ok(Stage::Finish);
    }
    let stage_data = if *local_file_chunk_size != payload.remote_file_chunk_size {
        let mut curr_tarns_trunk_index = payload.remote_file_chunk_size.total_chunks();
        if payload.remote_file_chunk_size.rest_size() != 0 {
            curr_tarns_trunk_index -= 1;
        }
        let offset = index_offset(curr_tarns_trunk_index);
        let buffer_size = buffer_size(local_file_len - offset as usize);
        let mut buffer = vec![0; buffer_size];
        let _ = local_file.read_at(&mut buffer, offset)?;
        (false, curr_tarns_trunk_index, Some(Bytes::from(buffer)))
    } else {
        (true, 0, None)
    };

    Ok(Stage::Processing(stage_data))
}
