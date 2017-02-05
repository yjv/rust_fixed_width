use spec::PaddingDirection;
use std::fmt::{Display, Formatter, Error as FmtError};

#[derive(Debug)]
pub struct Error {
    repr: Box<::std::error::Error + Send + Sync>
}

impl Clone for Error {
    fn clone(&self) -> Self {
        Error::new("")
    }
}

impl Error {
    pub fn new<E>(error: E) -> Self
        where E: Into<Box<::std::error::Error + Send + Sync>>
    {
        Error { repr: error.into() }
    }

    pub fn downcast<E: ::std::error::Error + Send + Sync + 'static>(self) -> ::std::result::Result<E, Self> {
        Ok(*(self.repr.downcast::<E>().map_err(|e| Error { repr: e })?))
    }

    pub fn downcast_ref<E: ::std::error::Error + Send + Sync + 'static>(&self) -> Option<&E> {
        self.repr.downcast_ref::<E>()
    }
}

impl ::std::error::Error for Error {
    fn description(&self) -> &str {
        self.repr.description()
    }

    fn cause(&self) -> Option<&::std::error::Error> {
        self.repr.cause()
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> ::std::result::Result<(), FmtError> {
        Display::fmt(&*self.repr, f)
    }
}

type Result<T> = ::std::result::Result<T, Error>;

pub trait Padder {
    fn pad(&self, data: &[u8], length: usize, padding: &[u8], direction: PaddingDirection) -> Result<Vec<u8>>;
}

impl<'a, T> Padder for &'a T where T: 'a + Padder {
    fn pad(&self, data: &[u8], length: usize, padding: &[u8], direction: PaddingDirection) -> Result<Vec<u8>> {
        (**self).pad(data, length, padding, direction)
    }
}

pub trait UnPadder {
    fn unpad(&self, data: &[u8], padding: &[u8], direction: PaddingDirection) -> Result<Vec<u8>>;
}

impl<'a, T> UnPadder for &'a T where T: 'a + UnPadder {
    fn unpad(&self, data: &[u8], padding: &[u8], direction: PaddingDirection) -> Result<Vec<u8>> {
        (**self).unpad(data, padding, direction)
    }
}

pub struct DefaultPadder;

#[derive(Debug)]
pub enum PaddingError {
    DataSplitNotOnCharBoundary(usize),
    PaddingSplitNotOnCharBoundary(usize)
}

impl ::std::error::Error for PaddingError {
    fn description(&self) -> &str {
        match *self {
            PaddingError::DataSplitNotOnCharBoundary(_) => "The index needed for splitting the data is not on a char boundary",
            PaddingError::PaddingSplitNotOnCharBoundary(_) => "The index needed for splitting the padding is not on a char boundary"
        }
    }
}

impl Display for PaddingError {
    fn fmt(&self, f: &mut Formatter) -> ::std::result::Result<(), FmtError> {
        match *self {
            PaddingError::DataSplitNotOnCharBoundary(index) => write!(
                f,
                "The index {} needed for splitting the data is not on a char boundary",
                index
            ),
            PaddingError::PaddingSplitNotOnCharBoundary(index) => write!(
                f,
                "The index {} needed for splitting the padding is not on a char boundary",
                index
            )
        }
    }
}

impl From<PaddingError> for Error {
    fn from(e: PaddingError) -> Self {
        Error::new(e)
    }
}

impl Padder for DefaultPadder {
    fn pad(&self, data: &[u8], length: usize, padding: &[u8], direction: PaddingDirection) -> Result<Vec<u8>> {
        if data.len() >= length {
            return Ok(data[..length].to_owned());
        }

        let padding_iter = padding.iter().cycle().take(length - data.len());

        Ok(if direction == PaddingDirection::Left {
            padding_iter.chain(data.iter()).cloned().collect()
        } else {
            data.iter().chain(padding_iter).cloned().collect()
        })
    }
}

impl UnPadder for DefaultPadder {
    fn unpad(&self, data: &[u8], padding: &[u8], direction: PaddingDirection) -> Result<Vec<u8>> {
        let mut index = 0;
        let mut iter = data.chunks(padding.len());

        while let Some(chunk) = match direction {
            PaddingDirection::Left => iter.next(),
            PaddingDirection::Right => iter.next_back(),
        } {
            if chunk != padding {
                break;
            }

            index += chunk.len();
        }

        Ok(match direction {
            PaddingDirection::Left => &data[index..],
            PaddingDirection::Right => &data[..data.len() - index],
        }.to_owned())
    }
}

pub struct IdentityPadder;

impl Padder for IdentityPadder {
    fn pad(&self, data: &[u8], _: usize, _: &[u8], _: PaddingDirection) -> Result<Vec<u8>> {
        Ok(data.to_owned())
    }
}

impl UnPadder for IdentityPadder {
    fn unpad(&self, data: &[u8], _: &[u8], _: PaddingDirection) -> Result<Vec<u8>> {
        Ok(data.to_owned())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use spec::*;

    #[test]
    fn default_padder() {
        let padder = DefaultPadder;
        let data = "qwer".as_bytes();
        assert_result!(Ok("qwer333333".as_bytes().to_owned()), padder.pad(data, 10, "33".as_bytes(), PaddingDirection::Right));
        let data = "qwer".as_bytes();
        assert_result!(Ok("333333qwer".as_bytes().to_owned()), padder.pad(data, 10, "33".as_bytes(), PaddingDirection::Left));
        let data = "qwer333333".as_bytes();
        assert_result!(Ok("qwer".as_bytes().to_owned()), padder.unpad(data, "33".as_bytes(), PaddingDirection::Right));
        let data = "333333qwer".as_bytes();
        assert_result!(Ok("qwer".as_bytes().to_owned()), padder.unpad(data, "33".as_bytes(), PaddingDirection::Left));
        let data = "qwer333333".as_bytes();
        assert_result!(Ok(data.to_owned()), padder.unpad(data, "33".as_bytes(), PaddingDirection::Left));
        let data = "333333qwer".as_bytes();
        assert_result!(Ok(data.to_owned()), padder.unpad(data, "33".as_bytes(), PaddingDirection::Right));
    }

    #[test]
    fn identity_padder() {
        let padder = IdentityPadder;
        let data = "qwer".as_bytes();
        assert_result!(Ok(data.to_owned()), padder.pad(data, 10, "3".as_bytes(), PaddingDirection::Right));
        assert_result!(Ok(data.to_owned()), padder.pad(data, 10, "3".as_bytes(), PaddingDirection::Left));
        assert_result!(Ok(data.to_owned()), padder.unpad(data, "3".as_bytes(), PaddingDirection::Right));
        assert_result!(Ok(data.to_owned()), padder.unpad(data, "3".as_bytes(), PaddingDirection::Left));
    }

    #[test]
    fn padder_reference() {
        let padder = IdentityPadder;
        let data = "qwer".as_bytes();
        assert_result!(Ok(data.to_owned()), Padder::pad(&&padder, data, 10, "3".as_bytes(), PaddingDirection::Right));
        assert_result!(Ok(data.to_owned()), UnPadder::unpad(&&padder, data, "3".as_bytes(), PaddingDirection::Right));
    }

    #[test]
    fn error() {
        let error = Error::new(PaddingError::PaddingSplitNotOnCharBoundary(23));
        assert_option!(Some(&PaddingError::PaddingSplitNotOnCharBoundary(23)), error.downcast_ref::<PaddingError>());
        assert_option!(Some(&PaddingError::PaddingSplitNotOnCharBoundary(23)), error.downcast_ref::<PaddingError>());
        match error.downcast::<PaddingError>() {
            Ok(PaddingError::PaddingSplitNotOnCharBoundary(23)) => (),
            e => panic!("bad result returned {:?}", e)
        }
    }
}