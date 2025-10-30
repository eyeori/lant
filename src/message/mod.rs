use crate::utils::bytes_num::NumberFromBytes;
use crate::utils::cursor::Cursor;
use crate::utils::json::{FromJson, ToJsonString};
use anyhow::{anyhow, Result};
use bytes::Bytes;
use num_enum_derive::{IntoPrimitive, TryFromPrimitive};
use std::mem::size_of;
use std::vec;

pub mod get;
pub mod ls;
pub mod put;

pub type SendMessage = Vec<Bytes>;
pub type RecvMessage = Bytes;
pub type MessagePayload = Vec<Bytes>;

pub type MessageMagic = u8;
pub type MessagePayloadSize = u64;
pub type MessagePayloadRef<'a> = &'a [u8];

#[repr(u16)]
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug, TryFromPrimitive, IntoPrimitive)]
pub enum MessageType {
    LsRequest = 0b00000001,
    LsResponse = 0b00000010,
    PutRequest = 0b00000100,
    PutResponse = 0b00001000,
    GetRequest = 0b00010000,
    GetResponse = 0b00100000,
    Error = 0b11110000,
    #[default]
    Invalid = 0b11111111,
}

impl MessageType {
    pub fn to_num(self) -> u16 {
        self as u16
    }
}

impl NumberFromBytes for MessageType {
    fn fle(bytes: &[u8]) -> Self
    where
        Self: Sized,
    {
        MessageType::try_from(u16::fle(bytes)).unwrap_or_default()
    }

    fn fbe(bytes: &[u8]) -> Self
    where
        Self: Sized,
    {
        MessageType::try_from(u16::fbe(bytes)).unwrap_or_default()
    }
}

const MESSAGE_MAGIC: MessageMagic = b'l' ^ b'a' ^ b'n' ^ b't';
const SIZE_OF_MESSAGE_MAGIC: usize = size_of::<MessageMagic>();
const SIZE_OF_MESSAGE_TYPE: usize = size_of::<MessageType>();
const SIZE_OF_MESSAGE_PAYLOAD_SIZE: usize = size_of::<MessagePayloadSize>();
const SIZE_OF_HEADER: usize =
    SIZE_OF_MESSAGE_MAGIC + SIZE_OF_MESSAGE_TYPE + SIZE_OF_MESSAGE_PAYLOAD_SIZE;

/// ```
/// ┌───────┬───────┬───┬───────┬───┬───┬───┬───┬───┬───┬───┬───────┬──~~~──┐
/// │ MAGIC │  MSG TYPE │ PAYLOAD SIZE                      │ PAYLOAD       │
/// └───────┴───────┴───┴───────┴───┴───┴───┴───┴───┴───┴───┴───────┴──~~~──┘
/// ```
pub fn deconstruct_message(
    msg: &RecvMessage,
) -> Result<(MessageType, Option<MessagePayloadRef<'_>>)> {
    let mut cursor = Cursor::new(msg);

    // header size valid
    if cursor.total_size() < SIZE_OF_HEADER {
        return Err(anyhow!("message header size invalid"));
    }

    // magic valid
    let magic = cursor.read_num_fle::<MessageMagic>()?;
    if magic != MESSAGE_MAGIC {
        return Err(anyhow!("message magic invalid"));
    }

    // type valid
    let msg_type = cursor.read_num_fle::<MessageType>()?;
    if msg_type == MessageType::Invalid {
        return Err(anyhow!("message type invalid"));
    }

    // total size valid
    let payload_size = cursor.read_num_fle::<MessagePayloadSize>()?;
    if cursor.total_size() != SIZE_OF_HEADER + payload_size as usize {
        return Err(anyhow!("message size invalid"));
    }

    let mut payload = None;
    if payload_size > 0 {
        let payload_bytes = cursor.rest()?;
        payload = Some(payload_bytes as MessagePayloadRef);
    }

    Ok((msg_type, payload))
}

pub fn build_message(msg_type: MessageType, payload: impl ToMessagePayload) -> SendMessage {
    let magic = Bytes::copy_from_slice(&MESSAGE_MAGIC.to_le_bytes());
    let msg_type = Bytes::copy_from_slice(&msg_type.to_num().to_le_bytes());
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
    build_message(MessageType::Error, Bytes::from(error_msg))
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
