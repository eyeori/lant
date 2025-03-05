use crate::command::{CommandClient, CommandServer};
use crate::message::ls::{LsRequestPayload, LsResponsePayload};
use crate::message::{
    build_message, FromMessagePayloadRef, MessagePayloadRef, MessageType, SendMessage,
};
use crate::quic::client::Client;
use crate::utils::dir::{DirItem, DirItemType};
use crate::utils::error::{MsgErr, Result};
use quinn::VarInt;
use std::fs;
use std::path::{Path, PathBuf};

pub struct LsCommandClient<'a> {
    client: &'a Client,
    path_on_remote: PathBuf,
}

impl<'a> LsCommandClient<'a> {
    pub fn new(client: &'a Client, path_on_remote: PathBuf) -> Self {
        Self {
            client,
            path_on_remote,
        }
    }

    async fn do_request(&self, client: &Client, path_on_remote: &Path) -> Result<()> {
        // build request payload
        let req_payload = LsRequestPayload::new(path_on_remote);

        // do request
        let conn = client.connecting()?.await?;
        let response = client
            .request(&conn, MessageType::LsRequest, req_payload)
            .await?;
        conn.close(VarInt::from(200u32), "OK".as_bytes());

        // process response
        if let Some(res_payload) = client.unwrap_message(&response, MessageType::LsResponse)? {
            self.process_response(res_payload)?;
        }
        println!("done");

        Ok(())
    }

    fn process_response(&self, payload: MessagePayloadRef) -> Result<()> {
        let payload = LsResponsePayload::from_payload(payload)?;
        println!("ls dir: {:?}", payload.dir);
        for entry in payload.items {
            let entry_type = if entry.is_file() { "file" } else { "dir " };
            println!("{}: \"{}\"", entry_type, entry.name())
        }
        Ok(())
    }
}

impl<'a> CommandClient for LsCommandClient<'a> {
    async fn request(&self) {
        if let Err(e) = self.do_request(self.client, &self.path_on_remote).await {
            println!("[ERR][Client] Process request error, error={e}");
        }
    }
}

pub struct LsCommandServer(PathBuf);

impl LsCommandServer {
    pub fn new(abs_root_dir: PathBuf) -> Self {
        Self(abs_root_dir)
    }
}

impl CommandServer for LsCommandServer {
    async fn handle(&self, req_payload: MessagePayloadRef<'_>) -> Result<SendMessage> {
        // deserialize request payload
        let req_payload = LsRequestPayload::from_payload(req_payload)?;

        // check abs_root_dir and ls_dir relation
        let mut abs_ls_path = self.0.clone();
        abs_ls_path.push(req_payload.path_on_remote.clone());
        abs_ls_path = abs_ls_path.canonicalize()?;
        if !abs_ls_path.starts_with(&self.0) {
            abs_ls_path = self.0.clone();
        }

        // build response payload
        let res_payload = if abs_ls_path.is_dir() {
            let mut items = Vec::new();
            for entry in fs::read_dir(abs_ls_path)? {
                let entry = entry?;
                let entry_name = entry
                    .file_name()
                    .into_string()
                    .map_err(|e| MsgErr::new(e.to_str().unwrap_or("")))?;
                let entry_type = DirItemType::from(entry.file_type()?);
                items.push(DirItem::new(entry_name, entry_type));
            }
            LsResponsePayload::new(req_payload.path_on_remote, items)
        } else if abs_ls_path.is_file() {
            let abs_ls_path = abs_ls_path.to_path_buf();
            let ls_item = abs_ls_path.file_name().unwrap().to_str().unwrap();
            let item = DirItem::new(ls_item.to_string(), DirItemType::File);
            LsResponsePayload::new(req_payload.path_on_remote, vec![item])
        } else {
            return Err(format!(
                "ls path resource not exists, path={:?}",
                req_payload.path_on_remote
            )
            .into());
        };

        // build payload message
        Ok(build_message(MessageType::LsResponse, res_payload))
    }
}
