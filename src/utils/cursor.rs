use crate::utils::bytes_num::NumberFromBytes;
use anyhow::Result;
use std::mem::size_of;
use thiserror::Error;

#[allow(dead_code)]
pub struct Cursor<'a>(std::io::Cursor<&'a [u8]>);

#[allow(dead_code)]
#[derive(Error, Debug)]
pub(crate) enum CursorError {
    #[error("out of bounds")]
    OutOfBounds,
}

#[allow(dead_code)]
impl<'a> Cursor<'a> {
    pub fn new(bytes: &'a [u8]) -> Cursor<'a> {
        Self(std::io::Cursor::new(bytes))
    }

    pub fn position(&self) -> u64 {
        self.0.position()
    }

    pub fn seek(&mut self, pos: u64) -> Result<()> {
        if pos > self.raw().len() as u64 {
            return Err(CursorError::OutOfBounds.into());
        }
        self.0.set_position(pos);
        Ok(())
    }

    pub fn raw(&self) -> &'a [u8] {
        self.0.get_ref()
    }

    pub fn total_size(&self) -> usize {
        self.raw().len()
    }

    pub fn rest(&self) -> Result<&'a [u8]> {
        let start = self.position() as usize;
        if start > self.raw().len() {
            return Err(CursorError::OutOfBounds.into());
        }
        Ok(&self.raw()[start..])
    }

    pub fn rest_size(&self) -> usize {
        self.raw().len().saturating_sub(self.position() as usize)
    }

    pub fn rest_is_empty(&self) -> bool {
        let start = self.position() as usize;
        start >= self.raw().len()
    }

    pub fn read_num_fle<T: NumberFromBytes>(&mut self) -> Result<T> {
        self.read_as(T::fle)
    }

    pub fn read_num_fbe<T: NumberFromBytes>(&mut self) -> Result<T> {
        self.read_as(T::fbe)
    }

    pub fn read_as<T, Op>(&mut self, convert: Op) -> Result<T>
    where
        for<'b> Op: FnOnce(&'b [u8]) -> T,
    {
        Ok(convert(self.read(size_of::<T>())?))
    }

    pub fn read(&mut self, count: usize) -> Result<&'a [u8]> {
        let (start, end) = self.range(count)?;
        let bytes = &self.raw()[start..end];
        self.0.set_position(end as u64);
        Ok(bytes)
    }

    pub fn peek_u8(&self) -> Result<u8> {
        Ok(u8::fle(self.peek(size_of::<u8>())?))
    }

    pub fn peek_u16(&self) -> Result<u16> {
        Ok(u16::fle(self.peek(size_of::<u16>())?))
    }

    pub fn peek_u32(&self) -> Result<u32> {
        Ok(u32::fle(self.peek(size_of::<u32>())?))
    }

    pub fn peek_u64(&self) -> Result<u64> {
        Ok(u64::fle(self.peek(size_of::<u64>())?))
    }

    pub fn peek_usize(&self) -> Result<usize> {
        Ok(usize::fle(self.peek(size_of::<usize>())?))
    }

    pub fn peek(&self, count: usize) -> Result<&'a [u8]> {
        let (start, end) = self.range(count)?;
        Ok(&self.raw()[start..end])
    }

    fn range(&self, count: usize) -> Result<(usize, usize)> {
        let start = self.position() as usize;
        let end = start + count;
        let len = self.raw().len();
        if start > len || end > len {
            return Err(CursorError::OutOfBounds.into());
        }
        Ok((start, end))
    }
}
