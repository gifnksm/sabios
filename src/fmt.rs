use core::{
    ascii,
    fmt::{self, LowerHex, Write},
};

pub(crate) struct LowerHexDebug<T>(pub(crate) T);

impl<T> fmt::Debug for LowerHexDebug<T>
where
    T: LowerHex,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:x}", self.0)
    }
}

pub(crate) struct ByteArray<'a>(pub(crate) &'a [u8]);

impl fmt::Debug for ByteArray<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(self.0.iter().map(LowerHexDebug))
            .finish()
    }
}

pub(crate) struct ByteString<'a>(pub(crate) &'a [u8]);

impl fmt::Debug for ByteString<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"")?;
        for byte in self.0.iter().flat_map(|&b| ascii::escape_default(b)) {
            f.write_char(byte as char)?;
        }
        write!(f, "\"")?;
        Ok(())
    }
}

impl fmt::Display for ByteString<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0.iter().flat_map(|&b| ascii::escape_default(b)) {
            f.write_char(byte as char)?;
        }
        Ok(())
    }
}
