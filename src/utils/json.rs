use anyhow::Result;
use serde::{Deserialize, Serialize};

pub trait FromJson<'a>
where
    Self: Sized,
{
    fn from_json(data: &'a [u8]) -> Result<Self>;
}

impl<'a, T> FromJson<'a> for T
where
    T: 'a + Deserialize<'a>,
{
    fn from_json(data: &'a [u8]) -> Result<Self> {
        Ok(serde_json::from_slice::<T>(data)?)
    }
}

pub trait ToJsonString {
    fn to_json(self) -> String;
}

impl<T> ToJsonString for T
where
    T: Serialize,
{
    fn to_json(self) -> String {
        serde_json::json!(self).to_string()
    }
}
