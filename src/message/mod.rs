use std::mem::size_of;
use std::vec;

use anyhow::{anyhow, Result};
use bytes::Bytes;

use crate::message::MessageTypeEnum::{
    Error, GetRequest, GetResponse, Invalid, LsRequest, LsResponse, PutRequest, PutResponse,
};
use crate::utils::bytes_as_t;
use crate::utils::json::{FromJson, ToJsonString};

pub mod get;
pub mod ls;
pub mod put;

pub type SendMessage = Vec<Bytes>;
pub type RecvMessage = Bytes;
pub type MessagePayload = Vec<Bytes>;

pub type MessageMagic = u8;
pub type MessageType = u16;
pub type MessagePayloadSize = u64;
pub type MessagePayloadRef<'a> = &'a [u8];

const MESSAGE_MAGIC: MessageMagic = b'l' ^ b'a' ^ b'n' ^ b't';
const SIZE_OF_MESSAGE_MAGIC: usize = size_of::<MessageMagic>();
const SIZE_OF_MESSAGE_TYPE: usize = size_of::<MessageType>();
const SIZE_OF_MESSAGE_PAYLOAD_SIZE: usize = size_of::<MessagePayloadSize>();
const SIZE_OF_HEADER: usize =
    SIZE_OF_MESSAGE_MAGIC + SIZE_OF_MESSAGE_TYPE + SIZE_OF_MESSAGE_PAYLOAD_SIZE;
const OFFSET_OF_MESSAGE_MAGIC: usize = 0;
const OFFSET_OF_MESSAGE_TYPE: usize = OFFSET_OF_MESSAGE_MAGIC + SIZE_OF_MESSAGE_MAGIC;
const OFFSET_OF_MESSAGE_PAYLOAD_SIZE: usize = OFFSET_OF_MESSAGE_TYPE + SIZE_OF_MESSAGE_TYPE;
const OFFSET_OF_MESSAGE_PAYLOAD: usize =
    OFFSET_OF_MESSAGE_PAYLOAD_SIZE + SIZE_OF_MESSAGE_PAYLOAD_SIZE;

/// ```
/// ┌───────┬───────┬───┬───────┬───┬───┬───┬───┬───┬───┬───┬───────┬──~~~──┐
/// │ MAGIC │  MSG TYPE │ PAYLOAD SIZE                      │ PAYLOAD       │
/// └───────┴───────┴───┴───────┴───┴───┴───┴───┴───┴───┴───┴───────┴──~~~──┘
/// ```
pub fn deconstruct_message(msg: &RecvMessage) -> Result<(MessageType, Option<MessagePayloadRef>)> {
    // header size valid
    if msg.len() < SIZE_OF_HEADER {
        return Err(anyhow!("message header size invalid"));
    }

    // magic valid
    let magic_bytes = &msg[OFFSET_OF_MESSAGE_MAGIC..OFFSET_OF_MESSAGE_TYPE];
    let magic = bytes_as_t::<MessageMagic>(magic_bytes);
    if magic != MESSAGE_MAGIC {
        return Err(anyhow!("message magic invalid"));
    }

    // type valid
    let type_bytes = &msg[OFFSET_OF_MESSAGE_TYPE..OFFSET_OF_MESSAGE_PAYLOAD_SIZE];
    let msg_type = bytes_as_t::<MessageType>(type_bytes);
    if MessageTypeEnum::from(msg_type) == Invalid {
        return Err(anyhow!("message type invalid"));
    }

    // total size valid
    let payload_size_bytes = &msg[OFFSET_OF_MESSAGE_PAYLOAD_SIZE..OFFSET_OF_MESSAGE_PAYLOAD];
    let payload_size = bytes_as_t::<MessagePayloadSize>(payload_size_bytes);
    if msg.len() != SIZE_OF_HEADER + payload_size as usize {
        return Err(anyhow!("message size invalid"));
    }

    let mut payload = None;
    if payload_size > 0 {
        let payload_bytes = &msg[OFFSET_OF_MESSAGE_PAYLOAD..];
        payload = Some(payload_bytes as MessagePayloadRef);
    }

    Ok((msg_type, payload))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MessageTypeEnum {
    Invalid,
    LsRequest,
    LsResponse,
    PutRequest,
    PutResponse,
    GetRequest,
    GetResponse,
    Error,
}

impl From<MessageType> for MessageTypeEnum {
    fn from(msg_type: MessageType) -> Self {
        match msg_type {
            0b00000001 => LsRequest,
            0b00000010 => LsResponse,
            0b00000100 => PutRequest,
            0b00001000 => PutResponse,
            0b00010000 => GetRequest,
            0b00100000 => GetResponse,
            0b11110000 => Error,
            _ => Invalid,
        }
    }
}

impl From<MessageTypeEnum> for MessageType {
    fn from(msg_type: MessageTypeEnum) -> Self {
        match msg_type {
            LsRequest => 0b00000001,
            LsResponse => 0b00000010,
            PutRequest => 0b00000100,
            PutResponse => 0b00001000,
            GetRequest => 0b00010000,
            GetResponse => 0b00100000,
            Error => 0b11110000,
            Invalid => 0b11111111,
        }
    }
}

pub fn build_message(msg_type: MessageTypeEnum, payload: impl ToMessagePayload) -> SendMessage {
    let magic = Bytes::copy_from_slice(&MESSAGE_MAGIC.to_le_bytes());
    let msg_type = Bytes::copy_from_slice(&MessageType::from(msg_type).to_le_bytes());
    let mut total_payload_size = 0;
    let mut payload = payload.to_payload();
    for chunked in &payload {
        total_payload_size += chunked.len();
    }
    let payload_size =
        Bytes::copy_from_slice(&(total_payload_size as MessagePayloadSize).to_le_bytes());
    let mut msg = vec![magic, msg_type, payload_size];
    msg.append(&mut payload);
    msg
}

pub fn build_error_message(error_msg: String) -> SendMessage {
    build_message(Error, Bytes::from(error_msg))
}

pub trait FromMessagePayloadRef<'a>
where
    Self: 'a + Sized,
{
    fn from_payload(payload: MessagePayloadRef<'a>) -> Result<Self>;
}

pub trait ToMessagePayload {
    fn to_payload(self) -> MessagePayload;
}

impl ToMessagePayload for Bytes {
    fn to_payload(self) -> MessagePayload {
        vec![self]
    }
}

pub trait JsonPayload {}

impl<'a, T> FromMessagePayloadRef<'a> for T
where
    T: JsonPayload + 'a + FromJson<'a>,
{
    fn from_payload(payload: MessagePayloadRef<'a>) -> Result<Self> {
        T::from_json(payload)
    }
}

impl<T> ToMessagePayload for T
where
    T: JsonPayload + ToJsonString,
{
    fn to_payload(self) -> MessagePayload {
        Bytes::from(self.to_json()).to_payload()
    }
}
