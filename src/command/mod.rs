use crate::message::{MessagePayloadRef, SendMessage};
use crate::utils::error::Result;

pub mod get;
pub mod ls;
pub mod put;

pub trait CommandClient {
    async fn request(&self);
}

pub trait CommandServer {
    async fn handle(&self, req_payload: MessagePayloadRef<'_>) -> Result<SendMessage>;
}
