use std::collections::{HashMap};
use spec::RecordSpec;
use std::io::{Read, BufRead, Error as IoError};
use std::fmt::{Display, Error as FmtError, Formatter};
use std::error::Error as ErrorTrait;
use record::{Data, DataRanges, ReadType, WriteType};

#[derive(Debug)]
pub enum Error {
    CouldNotRecognize,
    Other {
        repr: Box<::std::error::Error + Send + Sync>
    }
}

impl Clone for Error {
    fn clone(&self) -> Self {
        match *self {
            Error::CouldNotRecognize => Error::CouldNotRecognize,
            Error::Other { .. } => Error::new("")
        }
    }
}

impl Error {
    pub fn new<E>(error: E) -> Self
        where E: Into<Box<::std::error::Error + Send + Sync>>
    {
        Error::Other { repr: error.into() }
    }

    pub fn downcast<E: ::std::error::Error + Send + Sync + 'static>(self) -> ::std::result::Result<E, Self> {
        match self {
            Error::CouldNotRecognize => Err(Error::CouldNotRecognize),
            Error::Other { repr } => Ok(*(repr.downcast::<E>().map_err(|e| Error::Other { repr: e })?))
        }
    }

    pub fn downcast_ref<E: ::std::error::Error + Send + Sync + 'static>(&self) -> Option<&E> {
        match *self {
            Error::CouldNotRecognize => None,
            Error::Other { ref repr } => repr.downcast_ref::<E>()
        }
    }
}

impl ErrorTrait for Error {
    fn description(&self) -> &str {
        match *self {
            Error::CouldNotRecognize => "Could not recognize as any specific record spec",
            Error::Other { repr: ref e } => e.description()
        }
    }

    fn cause(&self) -> Option<&::std::error::Error> {
        match *self {
            Error::CouldNotRecognize => None,
            Error::Other { repr: ref e } => e.cause()
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> ::std::result::Result<(), FmtError> {
        match *self {
            Error::CouldNotRecognize => write!(f, "{}", self.description()),
            Error::Other { repr: ref e } => e.fmt(f),
        }
    }
}

impl From<IoError> for Error {
    fn from(e: IoError) -> Self {
        Error::new(e)
    }
}

type Result<T> = ::std::result::Result<T, Error>;

pub struct LineBuffer<'a, T: Read + 'a> {
    reader: &'a mut T,
    line: &'a mut Vec<u8>
}

impl<'a, T: Read + 'a> LineBuffer<'a, T> {
    pub fn new(reader: &'a mut T, line: &'a mut Vec<u8>) -> Self {
        LineBuffer {
            reader: reader,
            line: line
        }
    }

    pub fn fill_to(&mut self, size: usize) -> ::std::result::Result<usize, IoError> {
        let length = self.line.len();
        if length < size {
            (*self).reader.by_ref().take((size - self.line.len()) as u64).read_to_end(self.line)
        } else {
            Ok(0)
        }
    }

    pub fn into_inner(self) -> (&'a mut T, &'a mut Vec<u8>) {
        (self.reader, self.line)
    }

    pub fn get_line(&mut self) -> &mut Vec<u8> {
        self.line
    }
}

pub trait LineRecordSpecRecognizer<T: ReadType> {
    fn recognize_for_line<'a, 'b, U: BufRead + 'a>(&self, buffer: &'a mut U, record_specs: &'b HashMap<String, RecordSpec>, read_type: &'a T) -> Result<&'b str>;
    fn get_suggested_buffer_size<'a>(&self, _: &'a HashMap<String, RecordSpec>, _: &'a T) -> Option<usize> {
        None
    }
}

impl<'a, T, U: ReadType + 'a> LineRecordSpecRecognizer<U> for &'a T where T: 'a + LineRecordSpecRecognizer<U> {
    fn recognize_for_line<'b, 'c, V: BufRead + 'b>(&self, buffer: &'b mut V, record_specs: &'c HashMap<String, RecordSpec>, read_type: &'b U) -> Result<&'c str> {
        (**self).recognize_for_line(buffer, record_specs, read_type)
    }

    fn get_suggested_buffer_size<'b>(&self, record_specs: &'b HashMap<String, RecordSpec>, read_type: &'b U) -> Option<usize> {
        (**self).get_suggested_buffer_size(record_specs, read_type)
    }
}

pub trait DataRecordSpecRecognizer<T: WriteType> {
    fn recognize_for_data<'a, 'b, U: DataRanges + 'a>(&self, data: &'a Data<U, T::DataHolder>, record_specs: &'b HashMap<String, RecordSpec>, write_type: &'a T) -> Result<&'b str>;
}

impl<'a, T, U: WriteType + 'a> DataRecordSpecRecognizer<U> for &'a T where T: 'a + DataRecordSpecRecognizer<U> {
    fn recognize_for_data<'b, 'c, V: DataRanges + 'b>(&self, data: &'b Data<V, U::DataHolder>, record_specs: &'c HashMap<String, RecordSpec>, write_type: &'b U) -> Result<&'c str> {
        (**self).recognize_for_data(data, record_specs, write_type)
    }
}

pub struct IdFieldRecognizer {
    id_field: String
}

impl IdFieldRecognizer {
    pub fn new() -> Self {
        Self::new_with_field("$id")
    }

    pub fn new_with_field<U: Into<String>>(id_field: U) -> Self {
        IdFieldRecognizer { id_field: id_field.into() }
    }
}

impl<T: ReadType> LineRecordSpecRecognizer<T> for IdFieldRecognizer {
    fn recognize_for_line<'a, 'b, U: BufRead + 'a>(&self, mut buffer: &'a mut U, record_specs: &'b HashMap<String, RecordSpec>, read_type: &'a T) -> Result<&'b str> {
        for (name, record_spec) in record_specs.iter() {
            if let Some(ref field_spec) = record_spec.field_specs.get(&self.id_field) {
                if let Some(ref default) = field_spec.default {
                    if let Some(field_range) = read_type.get_byte_range(
                        buffer.fill_buf()?,
                        record_spec.field_range(&self.id_field).expect("This should never be None")
                    ) {
                        if buffer.fill_buf()?.len() < field_range.end {
                            continue;
                        }

                        if &buffer.fill_buf()?[field_range] == &default[..] {
                            return Ok(name);
                        }
                    }
                }
            }
        }

        Err(Error::CouldNotRecognize)
    }

    fn get_suggested_buffer_size<'a>(&self, record_specs: &'a HashMap<String, RecordSpec>, read_type: &'a T) -> Option<usize> {
        let min = record_specs.iter().map(|(_, spec)| spec.field_range(&self.id_field).map(|range| range.end).unwrap_or(0)).min().unwrap_or(0);
        if min == 0 {
            None
        } else {
            read_type.get_size_hint(min).1
        }
    }
}

impl<T: WriteType> DataRecordSpecRecognizer<T> for IdFieldRecognizer {
    fn recognize_for_data<'a, 'b, U: DataRanges + 'a>(&self, data: &'a Data<U, T::DataHolder>, record_specs: &'b HashMap<String, RecordSpec>, write_type: &'a T) -> Result<&'b str> {
        for (name, record_spec) in record_specs.iter() {
            if let Some(ref field_spec) = record_spec.field_specs.get(&self.id_field) {
                if let Some(ref default) = field_spec.default {
                    if let Some(data) = write_type.get_data_by_name(&self.id_field, data) {
                        if data == &default[..] {
                            return Ok(name);
                        }
                    }
                }
            }
        }

        Err(Error::CouldNotRecognize)
    }
}

pub struct NoneRecognizer;

impl<T: ReadType> LineRecordSpecRecognizer<T> for NoneRecognizer {
    fn recognize_for_line<'a, 'b, U: Read + 'a>(&self, _: &'a mut U, _: &'b HashMap<String, RecordSpec>, _: &'a T) -> Result<&'b str> {
        Err(Error::CouldNotRecognize)
    }
}

impl<T: WriteType> DataRecordSpecRecognizer<T> for NoneRecognizer {
    fn recognize_for_data<'a, 'b, U: DataRanges + 'a>(&self, _: &'a Data<U, T::DataHolder>, _: &'b HashMap<String, RecordSpec>, _: &'a T) -> Result<&'b str> {
        Err(Error::CouldNotRecognize)
    }
}

#[cfg(test)]
#[macro_use]
mod test {
    use super::*;
    use super::super::Data;
    use spec::*;
    use std::collections::{HashMap, BTreeMap};
    use std::io::{empty, BufReader};
    use record::{BinaryType, StringType};
    use writer::formatter::FormatError;

    #[test]
    fn none_recognizer() {
        let recognizer = NoneRecognizer;
        let data_type = BinaryType;
        assert_result!(Err(Error::CouldNotRecognize), recognizer.recognize_for_data(&Data { data: Vec::<u8>::new(), ranges: &BTreeMap::new() }, &HashMap::new(), &data_type));
        assert_result!(Err(Error::CouldNotRecognize), recognizer.recognize_for_line(
            &mut BufReader::new(&mut empty()),
            &HashMap::new(),
            &data_type
        ));
        let recognizer = NoneRecognizer;
        let data_type = StringType;
        assert_result!(Err(Error::CouldNotRecognize), recognizer.recognize_for_data(&Data { data: String::new(), ranges: BTreeMap::new() }, &HashMap::new(), &data_type));
        assert_result!(Err(Error::CouldNotRecognize), recognizer.recognize_for_line(
            &mut BufReader::new(&mut empty()),
            &HashMap::new(),
            &data_type
        ));
    }

    #[test]
    fn id_spec_recognizer() {
        let specs = SpecBuilder::new()
            .with_record(
                "record1",
                RecordSpecBuilder::new()
                    .with_field(
                        "field1",
                        FieldSpecBuilder::new()
                            .with_default("foo")
                            .with_length(3)
                            .with_padding("dsasd")
                            .with_padding_direction(PaddingDirection::Left)
                    )
                    .with_field(
                        "field2",
                        FieldSpecBuilder::new()
                            .with_length(5)
                            .with_padding("sdf".to_string())
                            .with_padding_direction(PaddingDirection::Right)
                    )
            )
            .with_record(
                "record2",
                RecordSpecBuilder::new()
                    .with_field(
                        "$id",
                        FieldSpecBuilder::new_string()
                            .with_default("bar")
                            .with_length(3)
                    )
                    .with_field(
                        "field2".to_string(),
                        FieldSpecBuilder::new_string()
                            .with_length(5)
                    )
            ).with_record(
            "record3",
            RecordSpecBuilder::new()
                .with_field(
                    "field1",
                    FieldSpecBuilder::new_string()
                        .with_default("bar")
                        .with_length(3)
                )
                .with_field(
                    "field2",
                    FieldSpecBuilder::new_string()
                        .with_length(5)
                )
        )
            .with_record(
                "record4",
                RecordSpecBuilder::new()
                    .with_field(
                        "$id",
                        FieldSpecBuilder::new_string()
                            .with_default("foo")
                            .with_length(3)
                    )
                    .with_field(
                        "field2".to_string(),
                        FieldSpecBuilder::new_string()
                            .with_length(5)
                    )
            )
            .build()
            .record_specs
        ;
        let recognizer = IdFieldRecognizer::new();
        let recognizer_with_field = IdFieldRecognizer::new_with_field("field1");
        let mut data = BTreeMap::new();
        let data_type = BinaryType;

        data.insert("$id".to_string(), "bar".as_bytes().to_owned());
        assert_result!(Ok("record2".to_string()), recognizer.recognize_for_data(&Into::<Data<_, Vec<u8>>>::into(data.clone()), &specs, &data_type));
        assert_result!(Err(Error::CouldNotRecognize), recognizer_with_field.recognize_for_data(&Into::<Data<_, Vec<u8>>>::into(data.clone()), &specs, &data_type));

        data.insert("$id".to_string(), "foo".as_bytes().to_owned());
        assert_result!(Ok("record4".to_string()), recognizer.recognize_for_data(&Into::<Data<_, Vec<u8>>>::into(data.clone()), &specs, &data_type));
        assert_result!(Err(Error::CouldNotRecognize), recognizer_with_field.recognize_for_data(&Into::<Data<_, Vec<u8>>>::into(data.clone()), &specs, &data_type));

        data.insert("$id".to_string(), "foobar".as_bytes().to_owned());
        assert_result!(Err(Error::CouldNotRecognize), recognizer.recognize_for_data(&Into::<Data<_, Vec<u8>>>::into(data.clone()), &specs, &data_type));
        assert_result!(Err(Error::CouldNotRecognize), recognizer_with_field.recognize_for_data(&Into::<Data<_, Vec<u8>>>::into(data.clone()), &specs, &data_type));

        data.insert("field1".to_string(), "bar".as_bytes().to_owned());
        assert_result!(Err(Error::CouldNotRecognize), recognizer.recognize_for_data(&Into::<Data<_, Vec<u8>>>::into(data.clone()), &specs, &data_type));
        assert_result!(Ok("record3".to_string()), recognizer_with_field.recognize_for_data(&Into::<Data<_, Vec<u8>>>::into(data.clone()), &specs, &data_type));
        data.remove(&"$id".to_string());

        assert_result!(Err(Error::CouldNotRecognize), recognizer.recognize_for_data(&Into::<Data<_, Vec<u8>>>::into(data.clone()), &specs, &data_type));
        assert_result!(Ok("record3".to_string()), recognizer_with_field.recognize_for_data(&Into::<Data<_, Vec<u8>>>::into(data.clone()), &specs, &data_type));

        data.insert("field1".to_string(), "foo".as_bytes().to_owned());
        assert_result!(Err(Error::CouldNotRecognize), recognizer.recognize_for_data(&Into::<Data<_, Vec<u8>>>::into(data.clone()), &specs, &data_type));
        assert_result!(Ok("record1".to_string()), recognizer_with_field.recognize_for_data(&Into::<Data<_, Vec<u8>>>::into(data.clone()), &specs, &data_type));

        data.insert("field1".to_string(), "foobar".as_bytes().to_owned());
        assert_result!(Err(Error::CouldNotRecognize), recognizer.recognize_for_data(&Into::<Data<_, Vec<u8>>>::into(data.clone()), &specs, &data_type));
        assert_result!(Err(Error::CouldNotRecognize), recognizer_with_field.recognize_for_data(&Into::<Data<_, Vec<u8>>>::into(data.clone()), &specs, &data_type));

        assert_result!(Err(Error::CouldNotRecognize), recognizer.recognize_for_line(&mut BufReader::new(&mut "dsfdsfsdfd".as_bytes()), &specs, &data_type));
        assert_result!(Err(Error::CouldNotRecognize), recognizer_with_field.recognize_for_line(&mut BufReader::new(&mut "dsfdsfsdfd".as_bytes()), &specs, &data_type));
        assert_result!(Err(Error::CouldNotRecognize), recognizer_with_field.recognize_for_line(&mut BufReader::new(&mut "ba".as_bytes()), &specs, &data_type));
        assert_result!(Ok("record2".to_string()), recognizer.recognize_for_line(&mut BufReader::new(&mut "barasdasdd".as_bytes()), &specs, &data_type));
        assert_result!(Ok("record3".to_string()), recognizer_with_field.recognize_for_line(&mut BufReader::new(&mut "barasdasdd".as_bytes()), &specs, &data_type));
        assert_result!(Ok("record4".to_string()), recognizer.recognize_for_line(&mut BufReader::new(&mut "foodsfsdfd".as_bytes()), &specs, &data_type));
        assert_result!(Ok("record1".to_string()), recognizer_with_field.recognize_for_line(&mut BufReader::new(&mut "foodsfsdfd".as_bytes()), &specs, &data_type));
    }

    #[test]
    fn recognizer_reference() {
        let recognizer = NoneRecognizer;
        let data_type = BinaryType;
        assert_result!(Err(Error::CouldNotRecognize), DataRecordSpecRecognizer::recognize_for_data(&&recognizer, &Data::<HashMap<_, _>, _>::new(), &HashMap::new(), &data_type));
        assert_result!(Err(Error::CouldNotRecognize), LineRecordSpecRecognizer::recognize_for_line(
            &&recognizer,
            &mut BufReader::new(&mut empty()),
            &HashMap::new(),
            &data_type
        ));
        let recognizer = NoneRecognizer;
        let data_type = StringType;
        assert_result!(Err(Error::CouldNotRecognize), DataRecordSpecRecognizer::recognize_for_data(&&recognizer, &Data { ranges: HashMap::<_, _>::new(), data: String::new() }, &HashMap::new(), &data_type));
        assert_result!(Err(Error::CouldNotRecognize), LineRecordSpecRecognizer::recognize_for_line(
            &&recognizer,
            &mut BufReader::new(&mut empty()),
            &HashMap::new(),
            &data_type
        ));
    }

    #[test]
    fn line_buffer() {
        let reader = &mut "dsfdsfsdfd".as_bytes();
        let mut string = Vec::new();
        let mut buffer = LineBuffer::new(reader, &mut string);
        buffer.fill_to(5).unwrap();
        buffer.fill_to(5).unwrap();
        assert_eq!(&mut "dsfds".as_bytes().to_owned(), buffer.get_line());
        buffer.fill_to(6).unwrap();
        assert_eq!(&mut "dsfdsf".as_bytes().to_owned(), buffer.get_line());
        let (buf, line) = buffer.into_inner();
        assert_eq!(&mut "dsfdsf".as_bytes().to_owned(), line);
        assert_eq!(&mut "sdfd".as_bytes(), buf);
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