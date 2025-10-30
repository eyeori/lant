pub trait NumberFromBytes {
    fn fle(bytes: &[u8]) -> Self
    where
        Self: Sized;

    fn fbe(bytes: &[u8]) -> Self
    where
        Self: Sized;
}

#[macro_export(local_inner_macros)]
macro_rules! num_from_bytes {
    ($($t:ty),+) => {
        $(
            impl NumberFromBytes for $t {
                fn fle(bytes: &[u8]) -> Self {
                    let mut buffer = [0u8; std::mem::size_of::<Self>()];
                    buffer.copy_from_slice(bytes);
                    Self::from_le_bytes(buffer)
                }

                fn fbe(bytes: &[u8]) -> Self {
                    let mut buffer = [0u8; std::mem::size_of::<Self>()];
                    buffer.copy_from_slice(bytes);
                    Self::from_be_bytes(buffer)
                }
            }
        )+
    };
}

num_from_bytes!(u8, u16, u32, u64, usize);
