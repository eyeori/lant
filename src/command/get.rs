use crate::command::{CommandClient, CommandServer};
use crate::message::get::{
    GetRequestPayload, GetResponseMeta, GetResponsePayload, GetResponsePayloadRef,
};
use crate::message::{
    build_message, FromMessagePayloadRef, MessagePayloadRef, MessageType, SendMessage,
};
use crate::quic::client::{Client, Stage};
use crate::utils::error::{MsgErr, Result};
use crate::utils::file::{buffer_size, get_file_chunk_size, index_offset, FileChunkSize};
use bytes::Bytes;
use chrono::Local;
use quinn::VarInt;
use std::fs::File;
use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};

pub struct GetCommandClient<'a> {
    client: &'a Client,
    file_path: PathBuf,
    local_dir: PathBuf,
}

impl<'a> GetCommandClient<'a> {
    pub fn new(client: &'a Client, file_path: PathBuf, local_dir: PathBuf) -> Self {
        Self {
            client,
            file_path,
            local_dir,
        }
    }

    async fn do_request(&self) -> Result<()> {
        println!(
            "get file: {:?}, to local dir: {:?}, time:{}",
            self.file_path,
            self.local_dir,
            Local::now().timestamp_millis()
        );
        let file_name = self
            .file_path
            .file_name()
            .ok_or(MsgErr::new("got file name error"))?;
        let file_name = PathBuf::from(file_name);
        let mut local_file_path = self.local_dir.to_path_buf();
        local_file_path.push(&file_name);
        let local_file_chunk_size = get_file_chunk_size(&local_file_path);
        let mut req_payload = GetRequestPayload::new(&self.file_path, local_file_chunk_size);

        let conn = self.client.connecting()?.await?;
        loop {
            // do request
            println!(">>>: {req_payload:?}");
            let response = self
                .client
                .request(&conn, MessageType::GetRequest, req_payload)
                .await?;

            // process response
            if let Some(res_payload) = self
                .client
                .unwrap_message(&response, MessageType::GetResponse)?
            {
                let stage = self.process_response(&file_name, res_payload)?;
                match stage {
                    Stage::Processing(local_file_chunk_size) => {
                        req_payload =
                            GetRequestPayload::new(&self.file_path, local_file_chunk_size);
                    }
                    Stage::Finish => break,
                }
            } else {
                break;
            }
        }
        conn.close(VarInt::from(200u32), "OK".as_bytes());

        println!(
            "get file: {:?}, to local dir: {:?} finish, time:{}",
            self.file_path,
            self.local_dir,
            Local::now().timestamp_millis()
        );

        Ok(())
    }

    fn process_response(
        &self,
        file_name: &Path,
        payload: MessagePayloadRef,
    ) -> Result<Stage<FileChunkSize>> {
        // get payload
        let payload = GetResponsePayloadRef::from_payload(payload)?;
        println!("<<<: {:?}", payload.meta);

        // build local file path
        let mut local_file_path = self.local_dir.to_path_buf();
        local_file_path.push(file_name);

        // open local file
        let local_file = File::options()
            .create(true)
            .append(true)
            .open(local_file_path)?;

        // append data
        if !payload.data.is_empty() {
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
}

impl<'a> CommandClient for GetCommandClient<'a> {
    async fn request(&self) {
        if let Err(e) = self.do_request().await {
            println!("[ERR][Client] Process request error, error={e}");
        }
    }
}

pub struct GetCommandServer(PathBuf);

impl GetCommandServer {
    pub fn new(abs_root_dir: PathBuf) -> Self {
        Self(abs_root_dir)
    }
}

impl CommandServer for GetCommandServer {
    async fn handle(&self, req_payload: MessagePayloadRef<'_>) -> Result<SendMessage> {
        // deserialize request payload
        let req_payload = GetRequestPayload::from_payload(req_payload)?;

        // check file path valid
        let mut file_path_valid = true;
        let mut abs_file_path = self.0.clone();
        abs_file_path.push(req_payload.remote_file_path.clone());
        abs_file_path = abs_file_path.canonicalize()?;
        if !abs_file_path.starts_with(&self.0) {
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
                return Err(format!(
                    "the file transfer is already completed, path={:?}",
                    req_payload.remote_file_path
                )
                .into());
            }
        } else {
            return Err(format!("file not exists, path={:?}", req_payload.remote_file_path).into());
        };

        // build response message
        Ok(build_message(MessageType::GetResponse, res_payload))
    }
}
