use spec::PaddingDirection;
use std::fmt::{Display, Formatter, Error as FmtError};
use data_type::{FieldReadSupport, BinarySupport};
use spec::FieldSpec;
use super::super::BoxedErrorResult as Result;

pub trait FieldParser<T: FieldReadSupport> {
    fn parse<'a>(&self, data: &[u8], field_spec: &'a FieldSpec, destination: &'a mut Vec<u8>, read_support: &'a T) -> Result<()>;
}

impl<'a, T, U: FieldReadSupport> FieldParser<U> for &'a T where T: FieldParser<U> + 'a {
    fn parse<'b>(&self, data: &'b [u8], field_spec: &'b FieldSpec, destination: &'b mut Vec<u8>, read_support: &'b U) -> Result<()> {
        (**self).parse(data, field_spec, destination, read_support)
    }
}

#[derive(Debug)]
pub enum ParseError {
    DataSplitNotOnCharBoundary(usize),
    PaddingSplitNotOnCharBoundary(usize)
}

impl ::std::error::Error for ParseError {
    fn description(&self) -> &str {
        match *self {
            ParseError::DataSplitNotOnCharBoundary(_) => "The index needed for splitting the data is not on a char boundary",
            ParseError::PaddingSplitNotOnCharBoundary(_) => "The index needed for splitting the padding is not on a char boundary"
        }
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter) -> ::std::result::Result<(), FmtError> {
        match *self {
            ParseError::DataSplitNotOnCharBoundary(index) => write!(
                f,
                "The index {} needed for splitting the data is not on a char boundary",
                index
            ),
            ParseError::PaddingSplitNotOnCharBoundary(index) => write!(
                f,
                "The index {} needed for splitting the padding is not on a char boundary",
                index
            )
        }
    }
}

pub struct DefaultParser;

impl FieldParser<BinarySupport> for DefaultParser {
    fn parse<'a>(&self, data: &'a [u8], field_spec: &'a FieldSpec, destination: &'a mut Vec<u8>, _: &'a BinarySupport) -> Result<()> {
        let mut index = 0;
        let mut iter = data.chunks(field_spec.padding.len());

        while let Some(chunk) = match field_spec.padding_direction {
            PaddingDirection::Left => iter.next(),
            PaddingDirection::Right => iter.next_back(),
        } {
            if chunk != &field_spec.padding[..] {
                break;
            }

            index += chunk.len();
        }

        destination.extend_from_slice(match field_spec.padding_direction {
            PaddingDirection::Left => &data[index..],
            PaddingDirection::Right => &data[..data.len() - index],
        });
        Ok(())
    }
}

pub struct IdentityParser;

impl<T: FieldReadSupport> FieldParser<T> for IdentityParser {
    fn parse<'a>(&self, data: &'a [u8], _: &'a FieldSpec, destination: &'a mut Vec<u8>, _: &'a T) -> Result<()> {
        destination.extend_from_slice(data);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use spec::*;
    use data_type::{BinarySupport, StringSupport};

    #[test]
    fn default_parser() {
        let padder = DefaultParser;
        let mut destination = Vec::new();
        let data_type = BinarySupport;
        let field_spec_builder = FieldSpecBuilder::new()
            .with_padding("33".to_owned())
            .with_length(0)
        ;
        let left_field_spec = field_spec_builder
            .clone()
            .with_padding_direction(PaddingDirection::Left)
            .build()
            .unwrap()
        ;
        let right_field_spec = field_spec_builder
            .with_padding_direction(PaddingDirection::Right)
            .build()
            .unwrap()
        ;
        let data = "qwer333333".as_bytes();
        assert_result!(Ok(()), padder.parse(data, &right_field_spec, &mut destination, &data_type));
        assert_eq!("qwer".as_bytes().to_owned(), destination);
        destination.clear();
        let data = "333333qwer".as_bytes();
        assert_result!(Ok(()), padder.parse(data, &left_field_spec, &mut destination, &data_type));
        assert_eq!("qwer".as_bytes().to_owned(), destination);
    }

    #[test]
    fn identity_parser() {
        let padder = IdentityParser;
        let data = "qwer".as_bytes();
        let mut destination = Vec::new();
        let data_type = BinarySupport;
        let field_spec = FieldSpecBuilder::new()
            .with_padding_direction(PaddingDirection::Right)
            .with_padding("33".to_owned())
            .with_length(0)
            .build()
            .unwrap()
        ;
        destination.clear();
        assert_result!(Ok(()), padder.parse(data, &field_spec, &mut destination, &data_type));
        assert_eq!(data.to_owned(), destination);
        destination.clear();
        assert_result!(Ok(()), padder.parse(data, &field_spec, &mut destination, &data_type));
        assert_eq!(data.to_owned(), destination);
    }

    #[test]
    fn parser_reference() {
        let padder = IdentityParser;
        let data = "qwer".as_bytes();
        let mut destination = Vec::new();
        let field_spec = FieldSpecBuilder::new()
            .with_padding_direction(PaddingDirection::Right)
            .with_padding("33".to_owned())
            .with_length(0)
            .build()
            .unwrap()
        ;
        let data_type = BinarySupport;
        assert_result!(Ok(()), FieldParser::parse(&&padder, data, &field_spec, &mut destination, &data_type));
        let data_type = StringSupport;
        assert_result!(Ok(()), FieldParser::parse(&&padder, data, &field_spec, &mut destination, &data_type));
    }
}