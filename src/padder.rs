use spec::PaddingDirection;
use std::fmt::{Display, Formatter, Error as FmtError};
use record::{ReadType, WriteType, BinaryType};

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

pub trait Padder<T: WriteType> {
    fn pad<'a>(&self, data: &[u8], length: usize, padding: &[u8], direction: PaddingDirection, destination: &'a mut Vec<u8>, data_type: &'a T) -> Result<()>;
}

impl<'a, T, U: WriteType> Padder<U> for &'a T where T: 'a + Padder<U> {
    fn pad<'b>(&self, data: &[u8], length: usize, padding: &[u8], direction: PaddingDirection, destination: &'b mut Vec<u8>, data_type: &'b U) -> Result<()> {
        (**self).pad(data, length, padding, direction, destination, data_type)
    }
}

pub trait UnPadder<T: ReadType> {
    fn unpad<'a>(&self, data: &[u8], padding: &[u8], direction: PaddingDirection, destination: &'a mut Vec<u8>, data_type: &'a T) -> Result<()>;
}

impl<'a, T, U: ReadType> UnPadder<U> for &'a T where T: 'a + UnPadder<U> {
    fn unpad<'b>(&self, data: &[u8], padding: &[u8], direction: PaddingDirection, destination: &'b mut Vec<u8>, data_type: &'b U) -> Result<()> {
        (**self).unpad(data, padding, direction, destination, data_type)
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

impl Padder<BinaryType> for DefaultPadder {
    fn pad<'a>(&self, data: &[u8], length: usize, padding: &[u8], direction: PaddingDirection, destination: &'a mut Vec<u8>, _: &'a BinaryType) -> Result<()> {
        if data.len() >= length {
            destination.extend_from_slice(&data[..length]);
            return Ok(());
        }

        let padding_iter = padding.iter().cycle().take(length - data.len());

        if direction == PaddingDirection::Left {
            destination.extend(padding_iter.chain(data.iter()));
        } else {
            destination.extend(data.iter().chain(padding_iter));
        }
        Ok(())
    }
}

impl UnPadder<BinaryType> for DefaultPadder {
    fn unpad<'a>(&self, data: &[u8], padding: &[u8], direction: PaddingDirection, destination: &'a mut Vec<u8>, _: &'a BinaryType) -> Result<()> {
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

        destination.extend_from_slice(match direction {
            PaddingDirection::Left => &data[index..],
            PaddingDirection::Right => &data[..data.len() - index],
        });
        Ok(())
    }
}

pub struct IdentityPadder;

impl<T: WriteType> Padder<T> for IdentityPadder {
    fn pad<'a>(&self, data: &[u8], _: usize, _: &[u8], _: PaddingDirection, destination: &'a mut Vec<u8>, _: &'a T) -> Result<()> {
        destination.extend_from_slice(data);
        Ok(())
    }
}

impl<T: ReadType> UnPadder<T> for IdentityPadder {
    fn unpad<'a>(&self, data: &[u8], _: &[u8], _: PaddingDirection, destination: &'a mut Vec<u8>, _: &'a T) -> Result<()> {
        destination.extend_from_slice(data);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use spec::*;
    use record::{BinaryType, StringType};

    #[test]
    fn default_padder() {
        let padder = DefaultPadder;
        let data = "qwer".as_bytes();
        let mut destination = Vec::new();
        let data_type = BinaryType;
        assert_result!(Ok(()), padder.pad(data, 10, "33".as_bytes(), PaddingDirection::Right, &mut destination, &data_type));
        assert_eq!("qwer333333".as_bytes().to_owned(), destination);
        destination.clear();
        let data = "qwer".as_bytes();
        assert_result!(Ok(()), padder.pad(data, 10, "33".as_bytes(), PaddingDirection::Left, &mut destination, &data_type));
        assert_eq!("333333qwer".as_bytes().to_owned(), destination);
        destination.clear();
        let data = "qwer333333".as_bytes();
        assert_result!(Ok(()), padder.unpad(data, "33".as_bytes(), PaddingDirection::Right, &mut destination, &data_type));
        assert_eq!("qwer".as_bytes().to_owned(), destination);
        destination.clear();
        let data = "333333qwer".as_bytes();
        assert_result!(Ok(()), padder.unpad(data, "33".as_bytes(), PaddingDirection::Left, &mut destination, &data_type));
        assert_eq!("qwer".as_bytes().to_owned(), destination);
        destination.clear();
        let data = "qwer333333".as_bytes();
        assert_result!(Ok(()), padder.unpad(data, "33".as_bytes(), PaddingDirection::Left, &mut destination, &data_type));
        assert_eq!(data.to_owned(), destination);
        destination.clear();
        let data = "333333qwer".as_bytes();
        assert_result!(Ok(()), padder.unpad(data, "33".as_bytes(), PaddingDirection::Right, &mut destination, &data_type));
        assert_eq!(data.to_owned(), destination);
    }

    #[test]
    fn identity_padder() {
        let padder = IdentityPadder;
        let data = "qwer".as_bytes();
        let mut destination = Vec::new();
        let data_type = BinaryType;
        assert_result!(Ok(()), padder.pad(data, 10, "3".as_bytes(), PaddingDirection::Right, &mut destination, &data_type));
        assert_eq!(data.to_owned(), destination);
        destination.clear();
        assert_result!(Ok(()), padder.pad(data, 10, "3".as_bytes(), PaddingDirection::Left, &mut destination, &data_type));
        assert_eq!(data.to_owned(), destination);
        let data_type = StringType;
        destination.clear();
        assert_result!(Ok(()), padder.unpad(data, "3".as_bytes(), PaddingDirection::Right, &mut destination, &data_type));
        assert_eq!(data.to_owned(), destination);
        destination.clear();
        assert_result!(Ok(()), padder.unpad(data, "3".as_bytes(), PaddingDirection::Left, &mut destination, &data_type));
        assert_eq!(data.to_owned(), destination);
    }

    #[test]
    fn padder_reference() {
        let padder = IdentityPadder;
        let data = "qwer".as_bytes();
        let mut destination = Vec::new();
        let data_type = BinaryType;
        assert_result!(Ok(()), Padder::pad(&&padder, data, 10, "3".as_bytes(), PaddingDirection::Right, &mut destination, &data_type));
        assert_result!(Ok(()), UnPadder::unpad(&&padder, data, "3".as_bytes(), PaddingDirection::Right, &mut destination, &data_type));
        let data_type = StringType;
        assert_result!(Ok(()), Padder::pad(&&padder, data, 10, "3".as_bytes(), PaddingDirection::Right, &mut destination, &data_type));
        assert_result!(Ok(()), UnPadder::unpad(&&padder, data, "3".as_bytes(), PaddingDirection::Right, &mut destination, &data_type));
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