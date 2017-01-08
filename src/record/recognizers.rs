use std::collections::HashMap;
use spec::RecordSpec;
use std::io::{Read, Error as IoError};
use std::fmt::{Display, Error as FmtError, Formatter};

#[derive(Debug)]
pub enum Error {
    CouldNotRecognize,
    Other(Box<::std::error::Error + Send + Sync>)
}

impl Clone for Error {
    fn clone(&self) -> Self {
        match *self {
            Error::CouldNotRecognize => Error::CouldNotRecognize,
            Error::Other(_) => Error::Other("".into())
        }
    }
}

impl Error {
    pub fn new<E>(error: E) -> Self
        where E: Into<Box<::std::error::Error + Send + Sync>>
    {
        Error::Other(error.into())
    }
}

impl ::std::error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::CouldNotRecognize => "Could not recognize as any specific record spec",
            Error::Other(ref e) => e.description()
        }
    }

    fn cause(&self) -> Option<&::std::error::Error> {
        match *self {
            Error::CouldNotRecognize => None,
            Error::Other(ref error) => error.cause()
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> ::std::result::Result<(), FmtError> {
        match *self {
            Error::CouldNotRecognize => write!(f, "CouldNotRecognize: could not recognize any record spec as apllying"),
            Error::Other(ref error) => error.fmt(f),
        }
    }
}

impl From<IoError> for Error {
    fn from(e: IoError) -> Self {
        Error::new(e)
    }
}

type Result<String> = ::std::result::Result<String, Error>;

pub struct LineBuffer<'a, T: Read + 'a> {
    reader: &'a mut T,
    line: String
}

impl<'a, T: Read + 'a> LineBuffer<'a, T> {
    pub fn new(reader: &'a mut T) -> Self {
        LineBuffer {
            reader: reader,
            line: String::new()
        }
    }
}

impl<'a, T: Read + 'a> LineBuffer<'a, T> {
    pub fn fill_to(&mut self, size: usize) -> ::std::result::Result<&String, IoError> {
        let length = self.line.len();
        if length < size {
            self.reader.by_ref().take((size - self.line.len()) as u64).read_to_string(&mut self.line)?;
        }

        Ok(&self.line)
    }

    pub fn into_line(self) -> String {
        self.line
    }
}

pub trait LineRecordSpecRecognizer {
    fn recognize_for_line<'a, T: Read + 'a>(&self, buffer: &'a mut LineBuffer<'a, T>, record_specs: &HashMap<String, RecordSpec>) -> Result<String>;
}

impl<'a, U> LineRecordSpecRecognizer for &'a U where U: 'a + LineRecordSpecRecognizer {
    fn recognize_for_line<'b, T: Read + 'b>(&self, buffer: &'b mut LineBuffer<'b, T>, record_specs: &HashMap<String, RecordSpec>) -> Result<String> {
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
    fn recognize_for_line<'a, T: Read + 'a>(&self, buffer: &'a mut LineBuffer<'a, T>, record_specs: &HashMap<String, RecordSpec>) -> Result<String> {
        for (name, record_spec) in record_specs.iter() {
            if let Some(ref field_spec) = record_spec.field_specs.get(&self.id_field) {
                if let Some(ref default) = field_spec.default {
                    let field_index = record_spec.get_field_index(&self.id_field);
                    let line = buffer.fill_to(field_index + field_spec.length)?;

                    if line.len() < field_index + field_spec.length {
                        continue;
                    }

                    if &line[field_index..field_index + field_spec.length] == default {
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
    fn recognize_for_line<'a, T: Read + 'a>(&self, _: &'a mut LineBuffer<'a, T>, _: &HashMap<String, RecordSpec>) -> Result<String> {
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
            &mut LineBuffer::new(&mut empty()),
            &HashMap::new()
        ));
    }

    #[test]
    fn id_spec_recognizer() {
        let specs = FileSpecBuilder::new()
            .with_record(
                "record1",
                RecordSpecBuilder::new(LineSpecBuilder::new().with_length(10))
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
                RecordSpecBuilder::new(LineSpecBuilder::new().with_length(10))
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
            RecordSpecBuilder::new(LineSpecBuilder::new().with_length(10))
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
                RecordSpecBuilder::new(LineSpecBuilder::new().with_length(10))
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

        assert_result!(Err(Error::CouldNotRecognize), recognizer.recognize_for_line(&mut LineBuffer::new(&mut "dsfdsfsdfd".as_bytes()), &specs));
        assert_result!(Err(Error::CouldNotRecognize), recognizer_with_field.recognize_for_line(&mut LineBuffer::new(&mut "dsfdsfsdfd".as_bytes()), &specs));
        assert_result!(Ok("record2".to_string()), recognizer.recognize_for_line(&mut LineBuffer::new(&mut "barasdasdd".as_bytes()), &specs));
        assert_result!(Ok("record3".to_string()), recognizer_with_field.recognize_for_line(&mut LineBuffer::new(&mut "barasdasdd".as_bytes()), &specs));
        assert_result!(Ok("record4".to_string()), recognizer.recognize_for_line(&mut LineBuffer::new(&mut "foodsfsdfd".as_bytes()), &specs));
        assert_result!(Ok("record1".to_string()), recognizer_with_field.recognize_for_line(&mut LineBuffer::new(&mut "foodsfsdfd".as_bytes()), &specs));
    }

    #[test]
    fn recognizer_reference() {
        let recognizer = NoneRecognizer;
        assert_result!(Err(Error::CouldNotRecognize), DataRecordSpecRecognizer::recognize_for_data(&&recognizer, &HashMap::new(), &HashMap::new()));
        assert_result!(Err(Error::CouldNotRecognize), LineRecordSpecRecognizer::recognize_for_line(
            &&recognizer,
            &mut LineBuffer::new(&mut empty()),
            &HashMap::new()
        ));
    }
}