use spec::{RecordSpec, FieldSpec};
use padders::Padder;
use std::collections::HashMap;
use std::io::{Write, Error as IoError};
use std::borrow::Borrow;

#[derive(Debug)]
pub enum Error<T: Padder> {
    RecordSpecNameRequired,
    RecordSpecNotFound(String),
    FieldSpecNotFound(String, String),
    PaddingFailed(T::Error),
    IoError(IoError),
    NotEnoughWritten(usize, usize),
    FieldValueRequired(String, String)
}

impl<T: Padder> From<IoError> for Error<T> {
    fn from(e: IoError) -> Self {
        Error::IoError(e)
    }
}

pub struct Writer<T: Padder, U: Borrow<HashMap<String, RecordSpec>>> {
    padder: T,
    specs: U
}

impl<T: Padder, U: Borrow<HashMap<String, RecordSpec>>> Writer<T, U> {
    pub fn new(padder: T, specs: U) -> Self {
        Writer {
            padder: padder,
            specs: specs
        }
    }

    pub fn write_field<'a, V: 'a + Write>(&self, writer: &'a mut V, record_name: String, name: String, value: String) -> Result<(), Error<T>> {
        let field_spec = self.specs.borrow()
            .get(&record_name)
            .ok_or_else(|| Error::RecordSpecNotFound(record_name.clone()))?
            .field_specs.get(&name)
            .ok_or_else(|| Error::FieldSpecNotFound(record_name.clone(), name.clone()))?
        ;
        Ok(self._write_field(writer, field_spec, value)?)
    }

    pub fn write_record<'a, V: 'a + Write>(&self, writer: &'a mut V, record_name: String, data: HashMap<String, String>) -> Result<(), Error<T>> {
        let record_spec = self.specs.borrow()
            .get(&record_name)
            .ok_or_else(|| Error::RecordSpecNotFound(record_name.clone()))?
        ;

        for (name, field_spec) in &record_spec.field_specs {
            self._write_field(writer, field_spec, data.get(name).or_else(|| field_spec.default.as_ref().clone()).ok_or_else(|| Error::FieldValueRequired(record_name.clone(), name.clone()))?.clone())?;
        }

        Ok(())
    }

    fn _write_field<'a, V: 'a + Write>(&self, writer: &'a mut V, field_spec: &FieldSpec, value: String) -> Result<(), Error<T>> {
        let value = self.padder.pad(value, field_spec.length, &field_spec.padding, field_spec.padding_direction).map_err(|e| Error::PaddingFailed(e))?;
        Ok(writer.write_all(value.as_bytes())?)
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
        let writer = Writer::new(&padder, &spec.record_specs);
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
        let writer = Writer::new(&padder, spec.record_specs);
        match writer.write_record(&mut buf, "record5".to_string(), HashMap::new()) {
            Err(Error::RecordSpecNotFound(record_name)) => assert_eq!("record5".to_string(), record_name),
            _ => panic!("should have returned a record spec not found error")
        }
    }

    #[test]
    fn write_record_with_padding_error() {
        let spec = test_spec();
        let mut buf = Cursor::new(Vec::new());
        let mut padder = MockPadder::new();
        padder.add_pad_call("hello".to_string(), 4, "dsasd".to_string(), PaddingDirection::Left, Err(()));
        let writer = Writer::new(&padder, spec.record_specs);
        writer.write_record(&mut buf, "record1".to_string(), [("field3".to_string(), "hello2".to_string())]
        .iter().cloned().collect()).unwrap_err();
    }

    #[test]
    fn write_record_with_write_error() {
        let spec = test_spec();
        let string: &mut [u8] = &mut [0; 4];
        let mut buf = Cursor::new(string);
        let mut padder = MockPadder::new();
        padder.add_pad_call("hello".to_string(), 4, "dsasd".to_string(), PaddingDirection::Left, Ok("hello2".to_string()));
        let writer = Writer::new(&padder, spec.record_specs);
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
        let writer = Writer::new(&padder, spec.record_specs);
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
        let writer = Writer::new(&padder, spec.record_specs);
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
        let writer = Writer::new(&padder, spec.record_specs);
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
        let writer = Writer::new(&padder, spec.record_specs);
        writer.write_field(&mut buf, "record1".to_string(), "field1".to_string(), "hello".to_string()).unwrap_err();
    }

    #[test]
    fn write_field_with_write_error() {
        let spec = test_spec();
        let string: &mut [u8] = &mut [0; 4];
        let mut buf = Cursor::new(string);
        let mut padder = MockPadder::new();
        padder.add_pad_call("hello".to_string(), 4, "dsasd".to_string(), PaddingDirection::Left, Ok("hello".to_string()));
        let writer = Writer::new(padder, spec.record_specs);
        writer.write_field(&mut buf, "record1".to_string(), "field1".to_string(), "hello".to_string()).unwrap_err();
    }
}