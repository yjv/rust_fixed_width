use spec::PaddingDirection;
use std::fmt::{Display, Formatter, Error as FmtError};
use record::{ReadType, WriteType, BinaryType};
use spec::FieldSpec;

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

pub trait FieldFormatter<T: WriteType> {
    fn format<'a>(&self, data: &'a [u8], field_spec: &'a FieldSpec, destination: &'a mut Vec<u8>, write_type: &'a T) -> Result<()>;
}

impl<'a, T, U: WriteType> FieldFormatter<U> for &'a T where T: 'a + FieldFormatter<U> {
    fn format<'b>(&self, data: &'b [u8], field_spec: &'b FieldSpec, destination: &'b mut Vec<u8>, write_type: &'b U) -> Result<()> {
        (**self).format(data, field_spec, destination, write_type)
    }
}

pub struct DefaultFormatter;

#[derive(Debug)]
pub enum FormatError {
    DataSplitNotOnCharBoundary(usize),
    PaddingSplitNotOnCharBoundary(usize)
}

impl ::std::error::Error for FormatError {
    fn description(&self) -> &str {
        match *self {
            FormatError::DataSplitNotOnCharBoundary(_) => "The index needed for splitting the data is not on a char boundary",
            FormatError::PaddingSplitNotOnCharBoundary(_) => "The index needed for splitting the padding is not on a char boundary"
        }
    }
}

impl Display for FormatError {
    fn fmt(&self, f: &mut Formatter) -> ::std::result::Result<(), FmtError> {
        match *self {
            FormatError::DataSplitNotOnCharBoundary(index) => write!(
                f,
                "The index {} needed for splitting the data is not on a char boundary",
                index
            ),
            FormatError::PaddingSplitNotOnCharBoundary(index) => write!(
                f,
                "The index {} needed for splitting the padding is not on a char boundary",
                index
            )
        }
    }
}

impl From<FormatError> for Error {
    fn from(e: FormatError) -> Self {
        Error::new(e)
    }
}

impl FieldFormatter<BinaryType> for DefaultFormatter {
    fn format<'a>(&self, data: &'a [u8], field_spec: &'a FieldSpec, destination: &'a mut Vec<u8>, _: &'a BinaryType) -> Result<()> {
        if data.len() >= field_spec.length {
            destination.extend_from_slice(&data[..field_spec.length]);
            return Ok(());
        }

        let padding_iter = field_spec.padding.iter().cycle().take(field_spec.length - data.len());

        if field_spec.padding_direction == PaddingDirection::Left {
            destination.extend(padding_iter.chain(data.iter()));
        } else {
            destination.extend(data.iter().chain(padding_iter));
        }
        Ok(())
    }
}

pub struct IdentityFormatter;

impl<T: WriteType> FieldFormatter<T> for IdentityFormatter {
    fn format<'a>(&self, data: &'a [u8], _: &'a FieldSpec, destination: &'a mut Vec<u8>, _: &'a T) -> Result<()> {
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
    fn default_formatter() {
        let padder = DefaultFormatter;
        let data = "qwer".as_bytes();
        let mut destination = Vec::new();
        let data_type = BinaryType;
        let field_spec_builder = FieldSpecBuilder::new()
            .with_padding("33".to_owned())
            .with_length(10)
        ;
        let left_field_spec = field_spec_builder
            .clone()
            .with_padding_direction(PaddingDirection::Left)
            .build()
        ;
        let right_field_spec = field_spec_builder
            .with_padding_direction(PaddingDirection::Right)
            .build()
        ;
        assert_result!(Ok(()), padder.format(data, &right_field_spec, &mut destination, &data_type));
        assert_eq!("qwer333333".as_bytes().to_owned(), destination);
        destination.clear();
        let data = "qwer".as_bytes();
        assert_result!(Ok(()), padder.format(data, &left_field_spec, &mut destination, &data_type));
        assert_eq!("333333qwer".as_bytes().to_owned(), destination);
        destination.clear();
    }

    #[test]
    fn identity_formatter() {
        let padder = IdentityFormatter;
        let data = "qwer".as_bytes();
        let mut destination = Vec::new();
        let data_type = BinaryType;
        let field_spec = FieldSpecBuilder::new()
            .with_padding_direction(PaddingDirection::Right)
            .with_padding("33".to_owned())
            .with_length(0)
            .build()
        ;
        assert_result!(Ok(()), padder.format(data, &field_spec, &mut destination, &data_type));
        assert_eq!(data.to_owned(), destination);
        destination.clear();
        assert_result!(Ok(()), padder.format(data, &field_spec, &mut destination, &data_type));
        assert_eq!(data.to_owned(), destination);
    }

    #[test]
    fn padder_reference() {
        let padder = IdentityFormatter;
        let data = "qwer".as_bytes();
        let mut destination = Vec::new();
        let data_type = BinaryType;
        let field_spec = FieldSpecBuilder::new()
            .with_padding_direction(PaddingDirection::Right)
            .with_padding("33".to_owned())
            .with_length(0)
            .build()
        ;
        assert_result!(Ok(()), FieldFormatter::format(&&padder, data, &field_spec, &mut destination, &data_type));
        let data_type = StringType;
        assert_result!(Ok(()), FieldFormatter::format(&&padder, data, &field_spec, &mut destination, &data_type));
    }

    #[test]
    fn error() {
        let error = Error::new(FormatError::PaddingSplitNotOnCharBoundary(23));
        assert_option!(Some(&FormatError::PaddingSplitNotOnCharBoundary(23)), error.downcast_ref::<FormatError>());
        assert_option!(Some(&FormatError::PaddingSplitNotOnCharBoundary(23)), error.downcast_ref::<FormatError>());
        match error.downcast::<FormatError>() {
            Ok(FormatError::PaddingSplitNotOnCharBoundary(23)) => (),
            e => panic!("bad result returned {:?}", e)
        }
    }
}