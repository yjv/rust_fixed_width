use spec::{RecordSpec, FieldSpec};
use padders::{Padder, IdentityPadder};
use std::collections::HashMap;
use std::io::{Write, Error as IoError};
use std::borrow::Borrow;
use super::recognizers::{DataRecordSpecRecognizer, NoneRecognizer};

#[derive(Debug)]
pub enum Error<T: Padder> {
    RecordSpecNameRequired,
    RecordSpecNotFound(String),
    FieldSpecNotFound(String, String),
    PaddingFailed(T::Error),
    PaddedValueNotLongEnough(usize, usize),
    IoError(IoError),
    NotEnoughWritten(usize, usize),
    FieldValueRequired(String, String)
}

impl<T: Padder> From<IoError> for Error<T> {
    fn from(e: IoError) -> Self {
        Error::IoError(e)
    }
}

pub struct Writer<T: Padder, U: DataRecordSpecRecognizer, V: Borrow<HashMap<String, RecordSpec>>> {
    padder: T,
    recognizer: U,
    specs: V
}

impl<T: Padder, U: DataRecordSpecRecognizer, V: Borrow<HashMap<String, RecordSpec>>> Writer<T, U, V> {
    pub fn write_field<'a, W, X, Y, Z>(&self, writer: &'a mut W, record_name: X, name: Y, value: Z) -> Result<(), Error<T>>
        where W: 'a + Write,
              X: Into<String>,
              Y: Into<String>,
              Z: Into<String>
    {
        let record_name = record_name.into();
        let name = name.into();
        let field_spec = self.specs.borrow()
            .get(&record_name)
            .ok_or_else(|| Error::RecordSpecNotFound(record_name.clone()))?
            .field_specs.get(&name)
            .ok_or_else(|| Error::FieldSpecNotFound(record_name.clone(), name.clone()))?
        ;
        Ok(self._write_field(writer, field_spec, value.into())?)
    }

    pub fn write_record<'a, W, X>(&self, writer: &'a mut W, record_name: X, data: HashMap<String, String>) -> Result<(), Error<T>>
        where W: 'a + Write,
              X: Into<Option<String>>
    {
        let record_name = record_name
            .into()
            .map(|v| v.into())
            .or_else(|| self.recognizer.recognize_for_data(&data, self.specs.borrow()))
            .ok_or_else(|| Error::RecordSpecNameRequired)?
        ;
        let record_spec = self.specs.borrow()
            .get(&record_name)
            .ok_or_else(|| Error::RecordSpecNotFound(record_name.clone()))?
        ;

        for (name, field_spec) in &record_spec.field_specs {
            self._write_field(writer, field_spec, data.get(name).or_else(|| field_spec.default.as_ref().clone()).ok_or_else(|| Error::FieldValueRequired(record_name.clone(), name.clone()))?.clone())?;
        }

        Ok(())
    }

    fn _write_field<'a, W: 'a + Write>(&self, writer: &'a mut W, field_spec: &FieldSpec, value: String) -> Result<(), Error<T>> {
        let value = self.padder.pad(value, field_spec.length, &field_spec.padding, field_spec.padding_direction).map_err(|e| Error::PaddingFailed(e))?;
        if value.len() != field_spec.length {
            return Err(Error::PaddedValueNotLongEnough(field_spec.length, value.len()));
        }

        Ok(writer.write_all(value.as_bytes())?)
    }
}

pub struct WriterBuilder<T: Padder, U: DataRecordSpecRecognizer, V: Borrow<HashMap<String, RecordSpec>>> {
    padder: Option<T>,
    recognizer: Option<U>,
    specs: Option<V>
}

impl<V: Borrow<HashMap<String, RecordSpec>>> WriterBuilder<IdentityPadder, NoneRecognizer, V> {
    pub fn new() -> WriterBuilder<IdentityPadder, NoneRecognizer, V> {
        WriterBuilder {
            padder: Some(IdentityPadder),
            recognizer: Some(NoneRecognizer),
            specs: None
        }
    }
}

impl<T: Padder, U: DataRecordSpecRecognizer, V: Borrow<HashMap<String, RecordSpec>>> WriterBuilder<T, U, V> {
    pub fn with_padder<W: Padder>(self, padder: W) -> WriterBuilder<W, U, V> {
        WriterBuilder {
            padder: Some(padder),
            recognizer: self.recognizer,
            specs: self.specs
        }
    }

    pub fn with_recognizer<W: DataRecordSpecRecognizer>(self, recognizer: W) -> WriterBuilder<T, W, V> {
        WriterBuilder {
            padder: self.padder,
            recognizer: Some(recognizer),
            specs: self.specs
        }
    }

    pub fn with_specs(mut self, specs: V) -> Self {
        self.specs = Some(specs);
        self
    }

    pub fn build(self) -> Writer<T, U, V> {
        Writer {
            padder: self.padder.unwrap(),
            recognizer: self.recognizer.unwrap(),
            specs: self.specs.expect("specs is required to build a writer")
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use test::*;
    use std::collections::HashMap;
    use std::io::Cursor;
    use spec::PaddingDirection;

    #[test]
    fn write_record() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;zxcvbnm,./-=[];".to_string();
        let mut buf = Cursor::new(Vec::new());
        let mut padder = MockPadder::new();
        padder.add_pad_call("hello".to_string(), 4, "dsasd".to_string(), PaddingDirection::Left, Ok(string[0..4].to_string()));
        padder.add_pad_call("def".to_string(), 5, " ".to_string(), PaddingDirection::Right, Ok(string[4..9].to_string()));
        padder.add_pad_call("hello2".to_string(), 36, "xcvcxv".to_string(), PaddingDirection::Right, Ok(string[9..45].to_string()));
        let writer = WriterBuilder::new().with_padder(&padder).with_specs(&spec.record_specs).build();
        writer.write_record(&mut buf, "record1".to_string(), [("field1".to_string(), "hello".to_string()),
            ("field3".to_string(), "hello2".to_string())]
            .iter().cloned().collect()).unwrap();
        assert_eq!(string, String::from_utf8(buf.into_inner()).unwrap());
    }

    #[test]
    fn write_record_with_bad_record_name() {
        let spec = test_spec();
        let mut buf = Cursor::new(Vec::new());
        let padder = MockPadder::new();
        let writer = WriterBuilder::new().with_padder(&padder).with_specs(spec.record_specs).build();
        match writer.write_record(&mut buf, Some("record5".to_string()), HashMap::<String, String>::new()) {
            Err(Error::RecordSpecNotFound(record_name)) => assert_eq!("record5".to_string(), record_name),
            _ => panic!("should have returned a record spec not found error")
        }
    }

    #[test]
    fn write_record_with_no_record_name() {
        let spec = test_spec();
        let mut buf = Cursor::new(Vec::new());
        let padder = MockPadder::new();
        let writer = WriterBuilder::new().with_padder(&padder).with_specs(spec.record_specs).build();
        match writer.write_record(&mut buf, None, HashMap::<String, String>::new()) {
            Err(Error::RecordSpecNameRequired) => (),
            _ => panic!("should have returned a record spec name required error")
        }
    }

    #[test]
    fn write_record_with_no_record_name_but_guessable() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;zxcvbnm,./-=[];".to_string();
        let mut buf = Cursor::new(Vec::new());
        let mut padder = MockPadder::new();
        padder.add_pad_call("hello".to_string(), 4, "dsasd".to_string(), PaddingDirection::Left, Ok(string[0..4].to_string()));
        padder.add_pad_call("def".to_string(), 5, " ".to_string(), PaddingDirection::Right, Ok(string[4..9].to_string()));
        padder.add_pad_call("hello2".to_string(), 36, "xcvcxv".to_string(), PaddingDirection::Right, Ok(string[9..45].to_string()));
        let data = [("field1".to_string(), "hello".to_string()),
            ("field3".to_string(), "hello2".to_string())]
            .iter().cloned().collect();
        let mut recognizer = MockRecognizer::new();
        recognizer.add_data_recognize_call(&data, &spec.record_specs, Some("record1".to_string()));
        let writer = WriterBuilder::new().with_padder(&padder).with_specs(&spec.record_specs).with_recognizer(&recognizer).build();
        writer.write_record(&mut buf, None, data.clone()).unwrap();
        assert_eq!(string, String::from_utf8(buf.into_inner()).unwrap());
    }

    #[test]
    fn write_record_with_padding_error() {
        let spec = test_spec();
        let mut buf = Cursor::new(Vec::new());
        let mut padder = MockPadder::new();
        padder.add_pad_call("hello".to_string(), 4, "dsasd".to_string(), PaddingDirection::Left, Err(()));
        let writer = WriterBuilder::new().with_padder(&padder).with_specs(spec.record_specs).build();
        writer.write_record(&mut buf, "record1".to_string(), [("field3".to_string(), "hello2".to_string())]
        .iter().cloned().collect()).unwrap_err();
    }

    #[test]
    fn write_record_with_padded_value_not_correct_length() {
        let spec = test_spec();
        let mut buf = Cursor::new(Vec::new());
        let mut padder = MockPadder::new();
        padder.add_pad_call("hello".to_string(), 4, "dsasd".to_string(), PaddingDirection::Left, Ok("hello2".to_string()));
        let writer = WriterBuilder::new().with_padder(&padder).with_specs(spec.record_specs).build();
        writer.write_record(&mut buf, "record1".to_string(), [("field3".to_string(), "hello2".to_string())]
        .iter().cloned().collect()).unwrap_err();
    }

    #[test]
    fn write_record_with_write_error() {
        let spec = test_spec();
        let string: &mut [u8] = &mut [0; 3];
        let mut buf = Cursor::new(string);
        let mut padder = MockPadder::new();
        padder.add_pad_call("hello".to_string(), 4, "dsasd".to_string(), PaddingDirection::Left, Ok("bye2".to_string()));
        let writer = WriterBuilder::new().with_padder(&padder).with_specs(spec.record_specs).build();
        writer.write_record(&mut buf, "record1".to_string(), [("field1".to_string(), "hello".to_string())]
        .iter().cloned().collect()).unwrap_err();
    }

    #[test]
    fn write_field() {
        let spec = test_spec();
        let string = "123456789".to_string();
        let mut buf = Cursor::new(Vec::new());
        let mut padder = MockPadder::new();
        padder.add_pad_call("hello".to_string(), 4, "dsasd".to_string(), PaddingDirection::Left, Ok(string[0..4].to_string()));
        padder.add_pad_call("hello2".to_string(), 5, " ".to_string(), PaddingDirection::Right, Ok(string[4..9].to_string()));
        let writer = WriterBuilder::new().with_padder(&padder).with_specs(spec.record_specs).build();
        writer.write_field(&mut buf, "record1".to_string(), "field1".to_string(), "hello".to_string()).unwrap();
        assert_eq!(string[0..4].to_string(), String::from_utf8(buf.get_ref().clone()).unwrap());
        writer.write_field(&mut buf, "record1".to_string(), "field2".to_string(), "hello2".to_string()).unwrap();
        assert_eq!(string, String::from_utf8(buf.into_inner()).unwrap());
    }

    #[test]
    fn write_field_with_bad_record_name() {
        let spec = test_spec();
        let mut buf = Cursor::new(Vec::new());
        let padder = MockPadder::new();
        let writer = WriterBuilder::new().with_padder(&padder).with_specs(spec.record_specs).build();
        match writer.write_field(&mut buf, "record5".to_string(), "field1".to_string(), "hello".to_string()) {
            Err(Error::RecordSpecNotFound(record_name)) => assert_eq!("record5".to_string(), record_name),
            _ => panic!("should have returned a record spec not found error")
        }
    }

    #[test]
    fn write_field_with_bad_field_name() {
        let spec = test_spec();
        let mut buf = Cursor::new(Vec::new());
        let padder = MockPadder::new();
        let writer = WriterBuilder::new().with_padder(&padder).with_specs(&spec.record_specs).build();
        match writer.write_field(&mut buf, "record1".to_string(), "field5".to_string(), "hello".to_string()) {
            Err(Error::FieldSpecNotFound(record_name, field_name)) => {
                assert_eq!("record1".to_string(), record_name);
                assert_eq!("field5".to_string(), field_name);
            },
            _ => panic!("should have returned a field spec not found error")
        }
    }

    #[test]
    fn write_field_with_padding_error() {
        let spec = test_spec();
        let mut buf = Cursor::new(Vec::new());
        let mut padder = MockPadder::new();
        padder.add_pad_call("hello".to_string(), 4, "dsasd".to_string(), PaddingDirection::Left, Err(()));
        let writer = WriterBuilder::new().with_padder(&padder).with_specs(spec.record_specs).build();
        writer.write_field(&mut buf, "record1".to_string(), "field1".to_string(), "hello".to_string()).unwrap_err();
    }

    #[test]
    fn write_field_with_padded_value_not_correct_length() {
        let spec = test_spec();
        let mut buf = Cursor::new(Vec::new());
        let mut padder = MockPadder::new();
        padder.add_pad_call("hello".to_string(), 4, "dsasd".to_string(), PaddingDirection::Left, Ok("123".to_string()));
        let writer = WriterBuilder::new().with_padder(&padder).with_specs(spec.record_specs).build();
        writer.write_field(&mut buf, "record1".to_string(), "field1".to_string(), "hello".to_string()).unwrap_err();
    }

    #[test]
    fn write_field_with_write_error() {
        let spec = test_spec();
        let string: &mut [u8] = &mut [0; 4];
        let mut buf = Cursor::new(string);
        let mut padder = MockPadder::new();
        padder.add_pad_call("hello".to_string(), 4, "dsasd".to_string(), PaddingDirection::Left, Ok("hello".to_string()));
        let writer = WriterBuilder::new().with_padder(padder).with_specs(spec.record_specs).build();
        writer.write_field(&mut buf, "record1".to_string(), "field1".to_string(), "hello".to_string()).unwrap_err();
    }
}