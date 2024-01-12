use std::fs;

use anyhow::{anyhow, Result};

use crate::message::ls::{LsRequestPayload, LsResponsePayload};
use crate::message::{
    build_message, FromMessagePayloadRef, MessagePayloadRef, MessageType, SendMessage,
};
use crate::server::get_server_abs_root_dir;
use crate::utils::dir::{DirItem, DirItemType};
use crate::utils::res::ExtResult;

pub async fn request(req_payload: MessagePayloadRef<'_>) -> Result<SendMessage> {
    // deserialize request payload
    let req_payload = LsRequestPayload::from_payload(req_payload)?;

    // get abs root dir
    let abs_root_dir = get_server_abs_root_dir()?;

    // check abs_root_dir and ls_dir relation
    let mut abs_ls_path = abs_root_dir.clone();
    abs_ls_path.push(req_payload.path_on_remote.clone());
    abs_ls_path = abs_ls_path.canonicalize()?;
    if !abs_ls_path
        .to_str()
        .ok()?
        .starts_with(abs_root_dir.to_str().ok()?)
    {
        abs_ls_path = abs_root_dir;
    }

    // build response payload
    let res_payload = if abs_ls_path.is_dir() {
        let mut items = Vec::new();
        for entry in fs::read_dir(abs_ls_path)? {
            let entry = entry?;
            let entry_name = entry.file_name().into_string().map_err(|_| anyhow!(""))?;
            let entry_type = DirItemType::from(entry.file_type()?);
            items.push(DirItem::new(entry_name, entry_type));
        }
        LsResponsePayload::new(req_payload.path_on_remote, items)
    } else if abs_ls_path.is_file() {
        let abs_ls_path = abs_ls_path.as_path();
        let ls_item = abs_ls_path.file_name().unwrap().to_str().unwrap();
        let item = DirItem::new(ls_item.to_string(), DirItemType::File);
        LsResponsePayload::new(req_payload.path_on_remote, vec![item])
    } else {
        return Err(anyhow!(
            "ls path resource not exists, path={:?}",
            req_payload.path_on_remote
        ));
    };

    // build payload message
    Ok(build_message(MessageType::LsResponse, res_payload))
}
