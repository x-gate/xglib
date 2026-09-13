use crate::RleError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildError {
    BufferTooShort {
        context: &'static str,
        needed: usize,
        actual: usize,
    },
    InvalidMagic {
        context: &'static str,
        expected: Vec<u8>,
        actual: Vec<u8>,
    },
    InvalidValue {
        context: &'static str,
        message: &'static str,
    },
    Unsupported {
        context: &'static str,
        message: &'static str,
    },
    TrailingBytes {
        context: &'static str,
        remaining: usize,
    },
    Rle(RleError),
}

impl From<RleError> for BuildError {
    fn from(value: RleError) -> Self {
        Self::Rle(value)
    }
}
