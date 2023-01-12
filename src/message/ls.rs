use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::message::JsonPayload;
use crate::utils::dir::DirItem;

#[derive(Serialize, Deserialize)]
pub struct LsRequestPayload {
    pub path_on_remote: PathBuf,
}

impl LsRequestPayload {
    pub fn new(path_on_remote: PathBuf) -> Self {
        Self { path_on_remote }
    }
}

impl JsonPayload for LsRequestPayload {}

#[derive(Serialize, Deserialize, Default)]
pub struct LsResponsePayload {
    pub dir: PathBuf,
    pub items: Vec<DirItem>,
}

impl LsResponsePayload {
    pub fn new(dir: PathBuf, items: Vec<DirItem>) -> Self {
        Self { dir, items }
    }
}

impl JsonPayload for LsResponsePayload {}
