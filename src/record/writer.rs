use spec::{RecordSpec, FieldSpec};
use padders::{Padder, IdentityPadder};
use std::collections::{HashMap, BTreeMap};
use std::io::{Write, Error as IoError};
use std::borrow::Borrow;
use super::recognizers::{DataRecordSpecRecognizer, NoneRecognizer};
use super::{Error, Result, Record};

pub struct Writer<T: Padder, U: DataRecordSpecRecognizer, V: Borrow<HashMap<String, RecordSpec>>> {
    padder: T,
    recognizer: U,
    specs: V
}

impl<T: Padder, U: DataRecordSpecRecognizer, V: Borrow<HashMap<String, RecordSpec>>> Writer<T, U, V> {
    pub fn write_field<'a, W>(&self, writer: &'a mut W, value: String, record_name: String, name: String) -> Result<()>
        where W: 'a + Write
    {
        let record_spec = self.specs.borrow()
            .get(&record_name)
            .ok_or_else(|| Error::RecordSpecNotFound(record_name.clone()))?
        ;
        let field_spec = record_spec
            .field_specs.get(&name)
            .ok_or_else(|| Error::FieldSpecNotFound(record_name.clone(), name.clone()))?
        ;
        self._write_field(writer, field_spec, value)?;

        Ok(())
    }

    pub fn write_record<'a, W, X>(&self, writer: &'a mut W, record: X) -> Result<()>
        where W: 'a + Write,
              X: Into<DataAndRecordName>
    {
        let (data, record_name) = record.into().into();
        let record_name = record_name
            .map_or_else(
                || self.recognizer.recognize_for_data(&data, self.specs.borrow()),
                |name| Ok(name)
            )?
        ;
        let record_spec = self.specs.borrow()
            .get(&record_name)
            .ok_or_else(|| Error::RecordSpecNotFound(record_name.clone()))?
        ;

        for (name, field_spec) in &record_spec.field_specs {
            self._write_field(writer, field_spec, data.get(name).or_else(|| field_spec.default.as_ref().clone()).ok_or_else(|| Error::FieldValueRequired(record_name.clone(), name.clone()))?.clone())?;
        }

        self.write_line_ending(writer, &record_spec.line_ending)?;

        Ok(())
    }

    fn _write_field<'a, W: 'a + Write>(&self, writer: &'a mut W, field_spec: &FieldSpec, value: String) -> Result<()> {
        let value = self.padder.pad(value, field_spec.length, &field_spec.padding, field_spec.padding_direction)?;
        if value.len() != field_spec.length {
            return Err(Error::PaddedValueWrongLength(field_spec.length, value.len()));
        }

        Ok(writer.write_all(value.as_bytes())?)
    }

    pub fn write_line_ending<'a, W: 'a + Write>(&self, writer: &'a mut W, line_ending: &String) -> ::std::result::Result<(), IoError> {
        writer.write(&line_ending.as_bytes())?;
        Ok(())
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

pub struct DataAndRecordName {
    pub data: BTreeMap<String, String>,
    pub name: Option<String>
}

impl From<HashMap<String, String>> for DataAndRecordName {
    fn from(data: HashMap<String, String>) -> Self {
        DataAndRecordName {
            data: data.into_iter().collect(),
            name: None
        }
    }
}

impl From<BTreeMap<String, String>> for DataAndRecordName {
    fn from(data: BTreeMap<String, String>) -> Self {
        DataAndRecordName {
            data: data,
            name: None
        }
    }
}

impl From<Record<BTreeMap<String, String>>> for DataAndRecordName {
    fn from(record: Record<BTreeMap<String, String>>) -> Self {
        DataAndRecordName {
            data: record.data,
            name: Some(record.name)
        }
    }
}

impl From<Record<HashMap<String, String>>> for DataAndRecordName {
    fn from(record: Record<HashMap<String, String>>) -> Self {
        DataAndRecordName {
            data: record.data.into_iter().collect(),
            name: Some(record.name)
        }
    }
}

impl From<(BTreeMap<String, String>, String)> for DataAndRecordName {
    fn from(record: (BTreeMap<String, String>, String)) -> Self {
        DataAndRecordName {
            data: record.0,
            name: Some(record.1)
        }
    }
}

impl From<(BTreeMap<String, String>, Option<String>)> for DataAndRecordName {
    fn from(record: (BTreeMap<String, String>, Option<String>)) -> Self {
        DataAndRecordName {
            data: record.0,
            name: record.1
        }
    }
}

impl From<(HashMap<String, String>, String)> for DataAndRecordName {
    fn from(record: (HashMap<String, String>, String)) -> Self {
        DataAndRecordName {
            data: record.0.into_iter().collect(),
            name: Some(record.1)
        }
    }
}

impl From<(HashMap<String, String>, Option<String>)> for DataAndRecordName {
    fn from(record: (HashMap<String, String>, Option<String>)) -> Self {
        DataAndRecordName {
            data: record.0.into_iter().collect(),
            name: record.1
        }
    }
}

impl Into<(BTreeMap<String, String>, Option<String>)> for DataAndRecordName {
    fn into(self) -> (BTreeMap<String, String>, Option<String>) {
        (self.data, self.name)
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use super::super::{Error, Record};
    use test::*;
    use std::collections::{HashMap, BTreeMap};
    use std::io::Cursor;
    use spec::PaddingDirection;
    use padders::Error as PaddingError;

    #[test]
    fn write_record() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;zxcvbnm,./-=[];\n".to_string();
        let mut buf = Cursor::new(Vec::new());
        let mut padder = MockPadder::new();
        padder.add_pad_call("hello".to_string(), 4, "dsasd".to_string(), PaddingDirection::Left, Ok(string[0..4].to_string()));
        padder.add_pad_call("def".to_string(), 5, " ".to_string(), PaddingDirection::Right, Ok(string[4..9].to_string()));
        padder.add_pad_call("hello2".to_string(), 36, "xcvcxv".to_string(), PaddingDirection::Right, Ok(string[9..45].to_string()));
        let writer = WriterBuilder::new().with_padder(&padder).with_specs(&spec.record_specs).build();
        writer.write_record(&mut buf, ([("field1".to_string(), "hello".to_string()),
            ("field3".to_string(), "hello2".to_string())]
            .iter().cloned().collect::<HashMap<_, _>>(), "record1".to_string())).unwrap();
        assert_eq!(string, String::from_utf8(buf.into_inner()).unwrap());
    }

    #[test]
    fn write_record_with_bad_record_name() {
        let spec = test_spec();
        let mut buf = Cursor::new(Vec::new());
        let padder = MockPadder::new();
        let writer = WriterBuilder::new().with_padder(&padder).with_specs(spec.record_specs).build();
        match writer.write_record(&mut buf, Record { data: BTreeMap::new(), name: "record5".to_string() }) {
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
        match writer.write_record(&mut buf, (HashMap::new(), None)) {
            Err(Error::RecordSpecNameRequired) => (),
            _ => panic!("should have returned a record spec name required error")
        }
    }

    #[test]
    fn write_record_with_no_record_name_but_guessable() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;zxcvbnm,./-=[];\n".to_string();
        let mut buf = Cursor::new(Vec::new());
        let mut padder = MockPadder::new();
        padder.add_pad_call("hello".to_string(), 4, "dsasd".to_string(), PaddingDirection::Left, Ok(string[0..4].to_string()));
        padder.add_pad_call("def".to_string(), 5, " ".to_string(), PaddingDirection::Right, Ok(string[4..9].to_string()));
        padder.add_pad_call("hello2".to_string(), 36, "xcvcxv".to_string(), PaddingDirection::Right, Ok(string[9..45].to_string()));
        let data = [("field1".to_string(), "hello".to_string()),
            ("field3".to_string(), "hello2".to_string())]
            .iter().cloned().collect();
        let mut recognizer = MockRecognizer::new();
        recognizer.add_data_recognize_call(&data, &spec.record_specs, Ok("record1".to_string()));
        let writer = WriterBuilder::new().with_padder(&padder).with_specs(&spec.record_specs).with_recognizer(&recognizer).build();
        writer.write_record(&mut buf, (data.clone(), None)).unwrap();
        assert_eq!(string, String::from_utf8(buf.into_inner()).unwrap());
    }

    #[test]
    fn write_record_with_padding_error() {
        let spec = test_spec();
        let mut buf = Cursor::new(Vec::new());
        let mut padder = MockPadder::new();
        padder.add_pad_call("hello".to_string(), 4, "dsasd".to_string(), PaddingDirection::Left, Err(PaddingError::new("")));
        let writer = WriterBuilder::new().with_padder(&padder).with_specs(spec.record_specs).build();
        writer.write_record(&mut buf, ([("field3".to_string(), "hello2".to_string())]
        .iter().cloned().collect::<BTreeMap<_, _>>(), "record1".to_string())).unwrap_err();
    }

    #[test]
    fn write_record_with_padded_value_not_correct_length() {
        let spec = test_spec();
        let mut buf = Cursor::new(Vec::new());
        let mut padder = MockPadder::new();
        padder.add_pad_call("hello".to_string(), 4, "dsasd".to_string(), PaddingDirection::Left, Ok("hello2".to_string()));
        let writer = WriterBuilder::new().with_padder(&padder).with_specs(spec.record_specs).build();
        writer.write_record(&mut buf, ([("field3".to_string(), "hello2".to_string())]
        .iter().cloned().collect::<HashMap<_, _>>(), "record1".to_string())).unwrap_err();
    }

    #[test]
    fn write_record_with_write_error() {
        let spec = test_spec();
        let string: &mut [u8] = &mut [0; 3];
        let mut buf = Cursor::new(string);
        let mut padder = MockPadder::new();
        padder.add_pad_call("hello".to_string(), 4, "dsasd".to_string(), PaddingDirection::Left, Ok("bye2".to_string()));
        let writer = WriterBuilder::new().with_padder(&padder).with_specs(spec.record_specs).build();
        writer.write_record(&mut buf, ([("field1".to_string(), "hello".to_string())]
        .iter().cloned().collect::<HashMap<_, _>>(), "record1".to_string())).unwrap_err();
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
        writer.write_field(&mut buf, "hello".to_string(), "record1".to_string(), "field1".to_string()).unwrap();
        assert_eq!(string[0..4].to_string(), String::from_utf8(buf.get_ref().clone()).unwrap());
        writer.write_field(&mut buf, "hello2".to_string(), "record1".to_string(), "field2".to_string()).unwrap();
        assert_eq!(string, String::from_utf8(buf.into_inner()).unwrap());
    }

    #[test]
    fn write_field_with_bad_record_name() {
        let spec = test_spec();
        let mut buf = Cursor::new(Vec::new());
        let padder = MockPadder::new();
        let writer = WriterBuilder::new().with_padder(&padder).with_specs(spec.record_specs).build();
        match writer.write_field(&mut buf, "hello".to_string(), "record5".to_string(), "field1".to_string()) {
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
        match writer.write_field(&mut buf, "hello".to_string(), "record1".to_string(), "field5".to_string()) {
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
        padder.add_pad_call("hello".to_string(), 4, "dsasd".to_string(), PaddingDirection::Left, Err(PaddingError::new("")));
        let writer = WriterBuilder::new().with_padder(&padder).with_specs(spec.record_specs).build();
        writer.write_field(&mut buf, "hello".to_string(), "record1".to_string(), "field1".to_string()).unwrap_err();
    }

    #[test]
    fn write_field_with_padded_value_not_correct_length() {
        let spec = test_spec();
        let mut buf = Cursor::new(Vec::new());
        let mut padder = MockPadder::new();
        padder.add_pad_call("hello".to_string(), 4, "dsasd".to_string(), PaddingDirection::Left, Ok("123".to_string()));
        let writer = WriterBuilder::new().with_padder(&padder).with_specs(spec.record_specs).build();
        writer.write_field(&mut buf, "hello".to_string(), "record1".to_string(), "field1".to_string()).unwrap_err();
    }

    #[test]
    fn write_field_with_write_error() {
        let spec = test_spec();
        let string: &mut [u8] = &mut [0; 4];
        let mut buf = Cursor::new(string);
        let mut padder = MockPadder::new();
        padder.add_pad_call("hello".to_string(), 4, "dsasd".to_string(), PaddingDirection::Left, Ok("hello".to_string()));
        let writer = WriterBuilder::new().with_padder(padder).with_specs(spec.record_specs).build();
        writer.write_field(&mut buf, "hello".to_string(), "record1".to_string(), "field1".to_string()).unwrap_err();
    }
}