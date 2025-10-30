use crate::message::JsonPayload;
use crate::utils::dir::DirItem;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
pub struct LsRequestPayload {
    pub remote_path: PathBuf,
}

impl LsRequestPayload {
    pub fn new(remote_path: impl Into<PathBuf>) -> Self {
        Self {
            remote_path: remote_path.into(),
        }
    }
}

impl JsonPayload for LsRequestPayload {}

#[derive(Serialize, Deserialize, Default)]
pub struct LsResponsePayload {
    pub dir: PathBuf,
    pub items: Vec<DirItem>,
}

impl LsResponsePayload {
    pub fn new(dir: impl Into<PathBuf>, items: Vec<DirItem>) -> Self {
        Self {
            dir: dir.into(),
            items,
        }
    }
}

impl JsonPayload for LsResponsePayload {}
