use std::fs::File;
use std::os::unix::fs::FileExt;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use chrono::Local;
use quinn::VarInt;

use crate::client::{send_and_receive, unwrap_message, Stage};
use crate::message::get::{GetRequestPayload, GetResponsePayloadRef};
use crate::message::{FromMessagePayloadRef, MessagePayloadRef, MessageTypeEnum};
use crate::utils::file::{get_file_chunk_size, index_offset, FileChunkSize};

pub async fn get(connecting: quinn::Connecting, file_path: &PathBuf, local_dir: &PathBuf) {
    if let Err(e) = process_request(connecting, file_path, local_dir).await {
        println!("[ERR][Client] Process request error, error={e}");
    }
}

async fn process_request(
    mut connecting: quinn::Connecting,
    file_path: &PathBuf,
    local_dir: &PathBuf,
) -> Result<()> {
    println!(
        "get file: {file_path:?}, to local dir: {local_dir:?}, time:{}",
        Local::now().timestamp_millis()
    );
    let file_name = file_path
        .as_path()
        .file_name()
        .ok_or(anyhow!("got file name error"))?;
    let file_name = PathBuf::from(file_name);
    let mut local_file_path = local_dir.clone();
    local_file_path.push(&file_name);
    let local_file_chunk_size = get_file_chunk_size(&local_file_path);
    let mut req_payload = GetRequestPayload::new(file_path.clone(), local_file_chunk_size);

    let conn = (&mut connecting).await?;
    loop {
        // do request
        println!(">>>: {req_payload:?}");
        let response = send_and_receive(&conn, MessageTypeEnum::GetRequest, req_payload).await?;

        // process response
        if let Some(res_payload) = unwrap_message(&response, MessageTypeEnum::GetResponse)? {
            let stage = process_response(local_dir, &file_name, res_payload)?;
            match stage {
                Stage::Processing(local_file_chunk_size) => {
                    req_payload = GetRequestPayload::new(file_path.clone(), local_file_chunk_size);
                }
                Stage::Finish => break,
            }
        } else {
            break;
        }
    }
    conn.close(VarInt::from(200u32), "OK".as_bytes());

    println!(
        "get file: {file_path:?}, to local dir: {local_dir:?} finish, time:{}",
        Local::now().timestamp_millis()
    );

    Ok(())
}

fn process_response(
    local_dir: &PathBuf,
    file_name: &PathBuf,
    payload: MessagePayloadRef,
) -> Result<Stage<FileChunkSize>> {
    // get payload
    let payload = GetResponsePayloadRef::from_payload(payload)?;
    println!("<<<: {:?}", payload.meta);

    // build local file path
    let mut local_file_path = local_dir.clone();
    local_file_path.push(file_name);

    // open local file
    let local_file = File::options()
        .create(true)
        .append(true)
        .open(local_file_path)?;

    // append data
    if payload.data.len() > 0 {
        let offset = index_offset(payload.meta.curr_trans_trunk_index);
        local_file.write_all_at(payload.data, offset)?;
    }

    // write analyse
    let stage = if payload.meta.remote_file_chunk_size.total_chunks()
        == payload.meta.curr_trans_trunk_index + 1
    {
        Stage::Finish
    } else {
        Stage::Processing(FileChunkSize::from(local_file.metadata()?.len() as usize))
    };

    Ok(stage)
}
