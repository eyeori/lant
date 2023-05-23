use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};

pub use get::get;
pub use ls::ls;
pub use put::put;

use crate::message::{
    build_message, deconstruct_message, MessagePayloadRef, MessageTypeEnum, RecvMessage,
    ToMessagePayload,
};
use crate::quic::quic_client::{build_client_config, build_connecting, build_endpoint};

mod get;
mod ls;
mod put;

pub(crate) fn init(connect_to: &str) -> Result<(Arc<quinn::Endpoint>, quinn::Connecting)> {
    // init
    let remote = connect_to.parse()?;
    let client_config = build_client_config()?;
    let endpoint = Arc::new(build_endpoint(client_config)?);

    // connect server
    Ok((endpoint.clone(), build_connecting(endpoint, remote)?))
}

async fn send_and_receive(
    conn: &quinn::Connection,
    msg_type: MessageTypeEnum,
    payload: impl ToMessagePayload,
) -> Result<RecvMessage> {
    // build request message
    let mut msg = build_message(msg_type, payload);

    // connect & send request
    let (mut ss, mut rs) = conn.open_bi().await?;
    ss.write_all_chunks(msg.as_mut_slice()).await?;
    ss.finish().await?;

    // receive response
    let response = rs.read_to_end(usize::MAX).await?;
    Ok(response.into())
}

fn unwrap_message(
    msg: &RecvMessage,
    msg_type_expect: MessageTypeEnum,
) -> Result<Option<MessagePayloadRef>> {
    let (msg_type, msg_payload) = deconstruct_message(msg)?;
    match MessageTypeEnum::from(msg_type) {
        msg_type if msg_type == msg_type_expect => Ok(msg_payload),
        MessageTypeEnum::Error => {
            if let Some(payload) = msg_payload {
                println!("[ERR]{}", std::str::from_utf8(payload)?);
            }
            Ok(None)
        }
        msg_type => {
            println!("[ERR]{msg_type:?} not fit");
            Ok(None)
        }
    }
}

#[derive(Serialize, Deserialize)]
pub enum Stage<T> {
    Processing(T),
    Finish,
}

impl<T> Default for Stage<T> {
    fn default() -> Self {
        Self::Finish
    }
}
