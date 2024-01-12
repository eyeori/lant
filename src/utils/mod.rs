pub mod dir;
pub mod file;
pub mod json;
pub mod res;

#[inline]
pub fn bytes_as_t<T: Copy>(bytes: &[u8]) -> T {
    unsafe { *(bytes.as_ptr() as *const T) }
}
