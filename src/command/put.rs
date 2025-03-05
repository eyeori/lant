use std::fs::File;
use std::os::unix::fs::FileExt;
use std::path::PathBuf;

use crate::command::{CommandClient, CommandServer};
use crate::message::put::{
    PutRequestMeta, PutRequestPayload, PutRequestPayloadRef, PutResponsePayload,
};
use crate::message::{
    build_message, FromMessagePayloadRef, MessagePayloadRef, MessageType, SendMessage,
};
use crate::quic::client::{Client, Stage};
use crate::utils::error::{MsgErr, Result};
use crate::utils::file::{buffer_size, index_offset, FileChunkSize};
use bytes::Bytes;
use chrono::Local;
use quinn::VarInt;

pub struct PutCommandClient<'a> {
    client: &'a Client,
    file_path: PathBuf,
    remote_dir: PathBuf,
}

impl<'a> PutCommandClient<'a> {
    pub fn new(client: &'a Client, file_path: PathBuf, remote_dir: PathBuf) -> Self {
        Self {
            client,
            file_path,
            remote_dir,
        }
    }

    async fn do_request(&self) -> Result<()> {
        println!(
            "put file: {:?}, to remote dir: {:?}, time:{}",
            self.file_path,
            self.remote_dir,
            Local::now().timestamp_millis()
        );

        let file_name = self
            .file_path
            .file_name()
            .ok_or(MsgErr::new("got file name error"))?;
        let mut req_meta = PutRequestMeta::new(file_name, &self.remote_dir);
        let mut req_payload = PutRequestPayload::new(req_meta.clone(), None);

        let conn = self.client.connecting()?.await?;
        loop {
            // do request
            println!(">>>: {:?}", req_payload.meta);
            let response = self
                .client
                .request(&conn, MessageType::PutRequest, req_payload)
                .await?;

            // process response
            if let Some(res_payload) = self
                .client
                .unwrap_message(&response, MessageType::PutResponse)?
            {
                let stage = self.process_response(res_payload)?;
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
            "put file: {:?}, to remote dir: {:?} finish, time:{:?}",
            self.file_path,
            self.remote_dir,
            Local::now().timestamp_millis()
        );

        Ok(())
    }

    fn process_response(
        &self,
        payload: MessagePayloadRef,
    ) -> Result<Stage<(bool, u64, Option<Bytes>)>> {
        let local_file = File::open(&self.file_path)?;
        let local_file_len = local_file.metadata()?.len() as usize;
        let local_file_chunk_size = FileChunkSize::from(local_file_len);

        // get payload
        let payload = PutResponsePayload::from_payload(payload)?;
        println!("<<<: {payload:?}");

        if payload.is_done {
            return Ok(Stage::Finish);
        }
        let stage_data = if local_file_chunk_size != payload.remote_file_chunk_size {
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
}

impl<'a> CommandClient for PutCommandClient<'a> {
    async fn request(&self) {
        if let Err(e) = self.do_request().await {
            println!("[ERR][Client] Process request error, error={e}");
        }
    }
}

pub struct PutCommandServer(PathBuf);

impl PutCommandServer {
    pub fn new(abs_root_dir: PathBuf) -> Self {
        Self(abs_root_dir)
    }
}

impl CommandServer for PutCommandServer {
    async fn handle(&self, req_payload: MessagePayloadRef<'_>) -> Result<SendMessage> {
        // deserialize request payload
        let req_payload = PutRequestPayloadRef::from_payload(req_payload)?;

        if req_payload.meta.is_done {
            return Ok(build_message(
                MessageType::PutResponse,
                PutResponsePayload::finish(),
            ));
        }

        // build file path
        let mut remote_file_path = self.0.clone();
        remote_file_path.push(req_payload.meta.remote_dir.clone());
        remote_file_path = remote_file_path.canonicalize()?;
        if !remote_file_path.starts_with(&self.0) {
            remote_file_path = self.0.clone();
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
        Ok(build_message(MessageType::PutResponse, res_payload))
    }
}
