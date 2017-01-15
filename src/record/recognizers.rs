use std::collections::HashMap;
use spec::RecordSpec;
use std::io::{Read, Error as IoError};
use std::fmt::{Display, Error as FmtError, Formatter};

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
}

impl ::std::error::Error for Error {
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
            Error::CouldNotRecognize => write!(f, "CouldNotRecognize: could not recognize any record spec as apllying"),
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
    line: &'a mut String
}

impl<'a, T: Read + 'a> LineBuffer<'a, T> {
    pub fn new(reader: &'a mut T, line: &'a mut String) -> Self {
        LineBuffer {
            reader: reader,
            line: line
        }
    }

    pub fn fill_to(&mut self, size: usize) -> ::std::result::Result<(), IoError> {
        let length = self.line.len();
        if length < size {
            (*self).reader.by_ref().take((size - self.line.len()) as u64).read_to_string(self.line)?;
        }

        Ok(())
    }

    pub fn into_inner(self) -> (&'a mut T, &'a mut String) {
        (self.reader, self.line)
    }

    pub fn get_line(&mut self) -> &mut String {
        self.line
    }
}

pub trait LineRecordSpecRecognizer {
    fn recognize_for_line<'a, T: Read + 'a>(&self, buffer: LineBuffer<'a, T>, record_specs: &HashMap<String, RecordSpec>) -> Result<String>;
}

impl<'a, V> LineRecordSpecRecognizer for &'a V where V: 'a + LineRecordSpecRecognizer {
    fn recognize_for_line<'b, T: Read + 'b>(&self, buffer: LineBuffer<'b, T>, record_specs: &HashMap<String, RecordSpec>) -> Result<String> {
        (**self).recognize_for_line(buffer, record_specs)
    }
}

pub trait DataRecordSpecRecognizer {
    fn recognize_for_data(&self, data: &HashMap<String, String>, record_specs: &HashMap<String, RecordSpec>) -> Result<String>;
}

impl<'a, T> DataRecordSpecRecognizer for &'a T where T: 'a + DataRecordSpecRecognizer {
    fn recognize_for_data(&self, data: &HashMap<String, String>, record_specs: &HashMap<String, RecordSpec>) -> Result<String> {
        (**self).recognize_for_data(data, record_specs)
    }
}

pub struct IdFieldRecognizer {
    id_field: String
}

impl IdFieldRecognizer {
    pub fn new() -> Self {
        Self::new_with_field("$id")
    }

    pub fn new_with_field<T: Into<String>>(id_field: T) -> Self {
        IdFieldRecognizer { id_field: id_field.into() }
    }
}

impl LineRecordSpecRecognizer for IdFieldRecognizer {
    fn recognize_for_line<'a, T: Read + 'a>(&self, mut buffer: LineBuffer<'a, T>, record_specs: &HashMap<String, RecordSpec>) -> Result<String> {
        for (name, record_spec) in record_specs.iter() {
            if let Some(ref field_spec) = record_spec.field_specs.get(&self.id_field) {
                if let Some(ref default) = field_spec.default {
                    let field_range = record_spec.field_range(&self.id_field).expect("This should never be None");
                    buffer.fill_to(field_range.start + field_spec.length)?;

                    if buffer.get_line().len() < field_range.start + field_spec.length {
                        continue;
                    }

                    if &buffer.get_line()[field_range] == default {
                        return Ok(name.clone());
                    }
                }
            }
        }

        Err(Error::CouldNotRecognize)
    }
}

impl DataRecordSpecRecognizer for IdFieldRecognizer {
    fn recognize_for_data(&self, data: &HashMap<String, String>, record_specs: &HashMap<String, RecordSpec>) -> Result<String> {
        for (name, record_spec) in record_specs.iter() {
            if let Some(ref field_spec) = record_spec.field_specs.get(&self.id_field) {
                if let Some(ref default) = field_spec.default {
                    if let Some(string) = data.get(&self.id_field) {
                        if string == default {
                            return Ok(name.clone());
                        }
                    }
                }
            }
        }

        Err(Error::CouldNotRecognize)
    }
}

pub struct NoneRecognizer;

impl LineRecordSpecRecognizer for NoneRecognizer {
    fn recognize_for_line<'a, T: Read + 'a>(&self, _: LineBuffer<'a, T>, _: &HashMap<String, RecordSpec>) -> Result<String> {
        Err(Error::CouldNotRecognize)
    }
}

impl DataRecordSpecRecognizer for NoneRecognizer {
    fn recognize_for_data(&self, _: &HashMap<String, String>, _: &HashMap<String, RecordSpec>) -> Result<String> {
        Err(Error::CouldNotRecognize)
    }
}

#[cfg(test)]
#[macro_use]
mod test {
    use super::*;
    use spec::*;
    use std::collections::HashMap;
    use std::io::empty;

    #[test]
    fn none_recognizer() {
        let recognizer = NoneRecognizer;
        assert_result!(Err(Error::CouldNotRecognize), recognizer.recognize_for_data(&HashMap::new(), &HashMap::new()));
        assert_result!(Err(Error::CouldNotRecognize), recognizer.recognize_for_line(
            LineBuffer::new(&mut empty(), &mut String::new()),
            &HashMap::new()
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
        let mut data = HashMap::new();

        data.insert("$id".to_string(), "bar".to_string());
        assert_result!(Ok("record2".to_string()), recognizer.recognize_for_data(&data, &specs));
        assert_result!(Err(Error::CouldNotRecognize), recognizer_with_field.recognize_for_data(&data, &specs));

        data.insert("$id".to_string(), "foo".to_string());
        assert_result!(Ok("record4".to_string()), recognizer.recognize_for_data(&data, &specs));
        assert_result!(Err(Error::CouldNotRecognize), recognizer_with_field.recognize_for_data(&data, &specs));

        data.insert("$id".to_string(), "foobar".to_string());
        assert_result!(Err(Error::CouldNotRecognize), recognizer.recognize_for_data(&data, &specs));
        assert_result!(Err(Error::CouldNotRecognize), recognizer_with_field.recognize_for_data(&data, &specs));

        data.insert("field1".to_string(), "bar".to_string());
        assert_result!(Err(Error::CouldNotRecognize), recognizer.recognize_for_data(&data, &specs));
        assert_result!(Ok("record3".to_string()), recognizer_with_field.recognize_for_data(&data, &specs));
        data.remove(&"$id".to_string());

        assert_result!(Err(Error::CouldNotRecognize), recognizer.recognize_for_data(&data, &specs));
        assert_result!(Ok("record3".to_string()), recognizer_with_field.recognize_for_data(&data, &specs));

        data.insert("field1".to_string(), "foo".to_string());
        assert_result!(Err(Error::CouldNotRecognize), recognizer.recognize_for_data(&data, &specs));
        assert_result!(Ok("record1".to_string()), recognizer_with_field.recognize_for_data(&data, &specs));

        data.insert("field1".to_string(), "foobar".to_string());
        assert_result!(Err(Error::CouldNotRecognize), recognizer.recognize_for_data(&data, &specs));
        assert_result!(Err(Error::CouldNotRecognize), recognizer_with_field.recognize_for_data(&data, &specs));

        assert_result!(Err(Error::CouldNotRecognize), recognizer.recognize_for_line(LineBuffer::new(&mut "dsfdsfsdfd".as_bytes(), &mut String::new()), &specs));
        assert_result!(Err(Error::CouldNotRecognize), recognizer_with_field.recognize_for_line(LineBuffer::new(&mut "dsfdsfsdfd".as_bytes(), &mut String::new()), &specs));
        assert_result!(Ok("record2".to_string()), recognizer.recognize_for_line(LineBuffer::new(&mut "barasdasdd".as_bytes(), &mut String::new()), &specs));
        assert_result!(Ok("record3".to_string()), recognizer_with_field.recognize_for_line(LineBuffer::new(&mut "barasdasdd".as_bytes(), &mut String::new()), &specs));
        assert_result!(Ok("record4".to_string()), recognizer.recognize_for_line(LineBuffer::new(&mut "foodsfsdfd".as_bytes(), &mut String::new()), &specs));
        assert_result!(Ok("record1".to_string()), recognizer_with_field.recognize_for_line(LineBuffer::new(&mut "foodsfsdfd".as_bytes(), &mut String::new()), &specs));
    }

    #[test]
    fn recognizer_reference() {
        let recognizer = NoneRecognizer;
        assert_result!(Err(Error::CouldNotRecognize), DataRecordSpecRecognizer::recognize_for_data(&&recognizer, &HashMap::new(), &HashMap::new()));
        assert_result!(Err(Error::CouldNotRecognize), LineRecordSpecRecognizer::recognize_for_line(
            &&recognizer,
            LineBuffer::new(&mut empty(), &mut String::new()),
            &HashMap::new()
        ));
    }

    #[test]
    fn line_buffer() {
        let reader = &mut "dsfdsfsdfd".as_bytes();
        let mut string = String::new();
        let mut buffer = LineBuffer::new(reader, &mut string);
        buffer.fill_to(5).unwrap();
        buffer.fill_to(5).unwrap();
        assert_eq!(&mut "dsfds".to_string(), buffer.get_line());
        buffer.fill_to(6).unwrap();
        assert_eq!(&mut "dsfdsf".to_string(), buffer.get_line());
        let (buf, line) = buffer.into_inner();
        assert_eq!(&mut "dsfdsf".to_string(), line);
        assert_eq!(&mut "sdfd".as_bytes(), buf);
    }
}