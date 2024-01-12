use std::path::Path;

use anyhow::Result;
use quinn::VarInt;

use crate::client::{send_and_receive, unwrap_message};
use crate::message::ls::{LsRequestPayload, LsResponsePayload};
use crate::message::{FromMessagePayloadRef, MessagePayloadRef, MessageType};

pub async fn ls(connecting: quinn::Connecting, path_on_remote: &Path) {
    if let Err(e) = process_request(connecting, path_on_remote).await {
        println!("[ERR][Client] Process request error, error={e}");
    }
}

async fn process_request(connecting: quinn::Connecting, path_on_remote: &Path) -> Result<()> {
    // build request payload
    let req_payload = LsRequestPayload::new(path_on_remote.to_path_buf());

    // do request
    let conn = connecting.await?;
    let response = send_and_receive(&conn, MessageType::LsRequest, req_payload).await?;
    conn.close(VarInt::from(200u32), "OK".as_bytes());

    // process response
    if let Some(res_payload) = unwrap_message(&response, MessageType::LsResponse)? {
        process_response(res_payload)?;
    }
    println!("done");

    Ok(())
}

fn process_response(payload: MessagePayloadRef) -> Result<()> {
    let payload = LsResponsePayload::from_payload(payload)?;
    println!("ls dir: {:?}", payload.dir);
    for entry in payload.items {
        let entry_type = if entry.is_file() { "file" } else { "dir " };
        println!("{}: \"{}\"", entry_type, entry.name())
    }
    Ok(())
}
