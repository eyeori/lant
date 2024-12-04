use std::fmt::{Debug, Display, Formatter};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug)]
pub struct MsgErr(String);

impl MsgErr {
    pub fn new(msg: impl Into<String>) -> Self {
        Self(msg.into())
    }

    pub fn res<T>(msg: impl Into<String>) -> Result<T> {
        Err(Self::new(msg).into())
    }
}

impl Display for MsgErr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for MsgErr {}
