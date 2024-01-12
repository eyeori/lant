use std::fs::FileType;

use serde::{Deserialize, Serialize};

use crate::utils::dir::DirItemType::{Dir, File};

#[derive(Serialize, Deserialize, PartialEq)]
pub enum DirItemType {
    Dir,
    File,
}

impl From<FileType> for DirItemType {
    fn from(item_type: FileType) -> Self {
        if item_type.is_file() {
            File
        } else {
            Dir
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct DirItem(String, DirItemType);

impl DirItem {
    pub fn new(name: impl Into<String>, item_type: DirItemType) -> Self {
        Self(name.into(), item_type)
    }

    pub fn name(&self) -> String {
        self.0.clone()
    }

    pub fn is_file(&self) -> bool {
        self.1 == File
    }
}
