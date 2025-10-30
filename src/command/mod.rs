use crate::message::{MessagePayloadRef, SendMessage};
use anyhow::Result;

pub mod get;
pub mod ls;
pub mod put;

pub trait CommandClient {
    async fn request(&self);
}

pub trait CommandServer {
    async fn handle(&self, payload: MessagePayloadRef<'_>) -> Result<SendMessage>;
}
