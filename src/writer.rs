use spec::{RecordSpec, FieldSpec};
use padder::{Padder, IdentityPadder};
use std::collections::{HashMap};
use std::io::Write;
use std::borrow::Borrow;
use recognizer::{DataRecordSpecRecognizer, NoneRecognizer};
use super::{Error, Result, PositionalResult, Record};
use record::{Data, DataRanges, WriteDataHolder};

pub struct Writer<T: Padder, U: DataRecordSpecRecognizer, V: Borrow<HashMap<String, RecordSpec>>> {
    padder: T,
    recognizer: U,
    specs: V
}

impl<T: Padder, U: DataRecordSpecRecognizer, V: Borrow<HashMap<String, RecordSpec>>> Writer<T, U, V> {
    pub fn write_field<'a, W>(&self, writer: &'a mut W, value: &'a [u8], record_name: &'a str, name: &'a str) -> Result<()>
        where W: 'a + Write
    {
        let record_spec = self.specs.borrow()
            .get(record_name)
            .ok_or_else(|| Error::RecordSpecNotFound(record_name.to_string()))?
        ;
        let field_spec = record_spec
            .field_specs.get(name)
            .ok_or_else(|| Error::FieldSpecNotFound(record_name.to_string(), name.to_string()))?
        ;
        self._write_field(writer, field_spec, value)?;

        Ok(())
    }

    pub fn write_record<'a, W, X, Y, Z, A>(&self, writer: &'a mut W, record: X, record_name: Y) -> PositionalResult<()>
        where W: 'a + Write,
              X: Into<(&'a Data<Z, A>, Option<&'a str>)>,
              Y: Into<Option<&'a str>>,
              Z: DataRanges + 'a,
              A: WriteDataHolder + 'a
    {
        let data_and_record_name = record.into();
        let (data, record_name) = (
            data_and_record_name.0,
            record_name.into().or(data_and_record_name.1)
        );
        let record_name = record_name
            .map_or_else(
                || self.recognizer.recognize_for_data(&data, self.specs.borrow()),
                |name| Ok(name.to_owned())
            )?
        ;
        let record_spec = self.specs.borrow()
            .get(&record_name)
            .ok_or_else(|| Error::RecordSpecNotFound(record_name.clone()))?
        ;

        for (name, field_spec) in &record_spec.field_specs {
            self._write_field(
                writer,
                field_spec,
                data.get_writable_data(name)
                    .or_else(|| field_spec.default.as_ref().map(|v| &v[..]))
                    .ok_or_else(|| (Error::FieldValueRequired, record_name.clone(), name.clone()))?
            ).map_err(|e| (e, record_name.clone(), name.clone()))?;
        }

        self.write_line_ending(writer, &record_spec.line_ending).map_err(|e| (e, record_name.clone()))?;

        Ok(())
    }

    pub fn write_line_ending<'a, W: 'a + Write>(&self, writer: &'a mut W, line_ending: &'a [u8]) -> Result<()> {
        writer.write(line_ending)?;
        Ok(())
    }

    fn _write_field<'a, W: 'a + Write>(&self, writer: &'a mut W, field_spec: &FieldSpec, value: &'a [u8]) -> Result<()> {
        let mut destination = Vec::new();
        self.padder.pad(value, field_spec.length, &field_spec.padding, field_spec.padding_direction, &mut destination)?;
        if destination.len() != field_spec.length {
            return Err(Error::PaddedValueWrongLength(field_spec.length, destination));
        }

        Ok(writer.write_all(&destination[..])?)
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

impl<'a, T: DataRanges + 'a, U: WriteDataHolder> Into<(&'a Data<T, U>, Option<&'a str>)> for &'a Data<T, U> {
    fn into(self) -> (&'a Data<T, U>, Option<&'a str>) {
        (self, None)
    }
}

impl<'a, T: DataRanges + 'a, U: WriteDataHolder> Into<(&'a Data<T, U>, Option<&'a str>)> for &'a Record<T, U> {
    fn into(self) -> (&'a Data<T, U>, Option<&'a str>) {
        (&self.data, Some(&self.name[..]))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use super::super::{Error, PositionalError, Position, Record, Data};
    use test::*;
    use std::collections::{HashMap, BTreeMap};
    use std::io::Cursor;
    use spec::PaddingDirection;
    use padder::Error as PaddingError;
    use std::ops::Range;

    #[test]
    fn write_record() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;zxcvbnm,./-=[];\n".to_string();
        let mut buf = Cursor::new(Vec::new());
        let mut padder = MockPadder::new();
        padder.add_pad_call("hello".as_bytes().to_owned(), 4, "dsasd".as_bytes().to_owned(), PaddingDirection::Left, Ok(string[0..4].as_bytes().to_owned()));
        padder.add_pad_call("def".as_bytes().to_owned(), 5, " ".as_bytes().to_owned(), PaddingDirection::Right, Ok(string[4..9].as_bytes().to_owned()));
        padder.add_pad_call("hello2".as_bytes().to_owned(), 36, "xcvcxv".as_bytes().to_owned(), PaddingDirection::Right, Ok(string[9..45].as_bytes().to_owned()));
        let writer = WriterBuilder::new().with_padder(&padder).with_specs(&spec.record_specs).build();
        writer.write_record(&mut buf, &Data::from([("field1".to_string(), "hello".as_bytes().to_owned()),
            ("field3".to_string(), "hello2".as_bytes().to_owned())]
            .iter().cloned().collect::<HashMap<_, _>>()), "record1").unwrap();
        assert_eq!(string, String::from_utf8(buf.into_inner()).unwrap());
    }

    #[test]
    fn write_record_with_bad_record_name() {
        let spec = test_spec();
        let mut buf = Cursor::new(Vec::new());
        let padder = MockPadder::new();
        let writer = WriterBuilder::new().with_padder(&padder).with_specs(spec.record_specs).build();
        assert_result!(
            Err(PositionalError { error: Error::RecordSpecNotFound(ref record), .. }) if record == "record5",
            writer.write_record(&mut buf, &Record { data: BTreeMap::new().into(), name: "record5".to_string() }, None)
        );
        assert_result!(
            Err(PositionalError { error: Error::RecordSpecNotFound(ref record), .. }) if record == "record5",
            writer.write_record(&mut buf, &Record { data: BTreeMap::new().into(), name: "record1".to_string() }, "record5")
        );
    }

    #[test]
    fn write_record_with_no_record_name() {
        let spec = test_spec();
        let mut buf = Cursor::new(Vec::new());
        let padder = MockPadder::new();
        let writer = WriterBuilder::new().with_padder(&padder).with_specs(spec.record_specs).build();
        assert_result!(
            Err(PositionalError { error: Error::RecordSpecNameRequired, .. }),
            writer.write_record(&mut buf, &Data::from(HashMap::new()), None)
        );
    }

    #[test]
    fn write_record_with_no_record_name_but_guessable() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;zxcvbnm,./-=[];\n".to_string();
        let mut buf = Cursor::new(Vec::new());
        let mut padder = MockPadder::new();
        padder.add_pad_call("hello".as_bytes().to_owned(), 4, "dsasd".as_bytes().to_owned(), PaddingDirection::Left, Ok(string[0..4].as_bytes().to_owned()));
        padder.add_pad_call("def".as_bytes().to_owned(), 5, " ".as_bytes().to_owned(), PaddingDirection::Right, Ok(string[4..9].as_bytes().to_owned()));
        padder.add_pad_call("hello2".as_bytes().to_owned(), 36, "xcvcxv".as_bytes().to_owned(), PaddingDirection::Right, Ok(string[9..45].as_bytes().to_owned()));
        let data = [("field1".to_string(), "hello".as_bytes().to_owned()),
            ("field3".to_string(), "hello2".as_bytes().to_owned())]
            .iter().cloned().collect();
        let mut recognizer = MockRecognizer::<BTreeMap<String, Range<usize>>>::new();
        recognizer.add_data_recognize_call(&data, &spec.record_specs, Ok("record1".to_string()));
        let writer = WriterBuilder::new().with_padder(&padder).with_specs(&spec.record_specs).with_recognizer(&recognizer).build();
        writer.write_record(&mut buf, &data, None).unwrap();
        assert_eq!(string, String::from_utf8(buf.into_inner()).unwrap());
    }

    #[test]
    fn write_record_with_field_require_error() {
        let spec = test_spec();
        let mut buf = Cursor::new(Vec::new());
        let mut padder = MockPadder::new();
        padder.add_pad_call("hello".as_bytes().to_owned(), 4, "dsasd".as_bytes().to_owned(), PaddingDirection::Left, Err(PaddingError::new("")));
        let writer = WriterBuilder::new().with_padder(&padder).with_specs(spec.record_specs).build();
        assert_result!(
            Err(PositionalError {
                error: Error::PadderFailure(_),
                position: Some(Position { ref record, field: Some(ref field) })
            }) if record == "record1" && field == "field1",
            writer.write_record(&mut buf, &Data::from([("field1".to_string(), "hello".as_bytes().to_owned())]
                .iter().cloned().collect::<BTreeMap<_, _>>()), "record1")
        );
    }

    #[test]
    fn write_record_with_padding_error() {
        let spec = test_spec();
        let mut buf = Cursor::new(Vec::new());
        let padder = MockPadder::new();
        let writer = WriterBuilder::new().with_padder(&padder).with_specs(spec.record_specs).build();
        assert_result!(
            Err(PositionalError {
                error: Error::FieldValueRequired,
                position: Some(Position { ref record, field: Some(ref field) })
            }) if record == "record1" && field == "field1",
            writer.write_record(&mut buf, &Data::from([("field3".to_string(), "hello".as_bytes().to_owned())]
                .iter().cloned().collect::<BTreeMap<_, _>>()), "record1")
        );
    }

    #[test]
    fn write_record_with_padded_value_not_correct_length() {
        let spec = test_spec();
        let mut buf = Cursor::new(Vec::new());
        let mut padder = MockPadder::new();
        padder.add_pad_call("hello".as_bytes().to_owned(), 4, "dsasd".as_bytes().to_owned(), PaddingDirection::Left, Ok("hello2".as_bytes().to_owned()));
        let writer = WriterBuilder::new().with_padder(&padder).with_specs(spec.record_specs).build();
        assert_result!(
            Err(PositionalError {
                error: Error::PaddedValueWrongLength(4, ref value),
                position: Some(Position { ref record, field: Some(ref field) })
            }) if *value == "hello2".as_bytes().to_owned() && record == "record1" && field == "field1",
            writer.write_record(&mut buf, &Data::from([("field1".to_string(), "hello".as_bytes().to_owned())]
                .iter().cloned().collect::<HashMap<_, _>>()), "record1")
        );
    }

    #[test]
    fn write_record_with_write_error() {
        let spec = test_spec();
        let string: &mut [u8] = &mut [0; 3];
        let mut buf = Cursor::new(string);
        let mut padder = MockPadder::new();
        padder.add_pad_call("hello".as_bytes().to_owned(), 4, "dsasd".as_bytes().to_owned(), PaddingDirection::Left, Ok("bye2".as_bytes().to_owned()));
        let writer = WriterBuilder::new().with_padder(&padder).with_specs(spec.record_specs).build();
        assert_result!(
            Err(PositionalError {
                error: Error::IoError(_),
                position: Some(Position { ref record, field: Some(ref field) })
            }) if record == "record1" && field == "field1",
            writer.write_record(&mut buf, &Data::from([("field1".to_string(), "hello".as_bytes().to_owned())]
                .iter().cloned().collect::<HashMap<_, _>>()), "record1")
        );
    }

    #[test]
    fn write_field() {
        let spec = test_spec();
        let string = "123456789".to_string();
        let mut buf = Cursor::new(Vec::new());
        let mut padder = MockPadder::new();
        padder.add_pad_call("hello".as_bytes().to_owned(), 4, "dsasd".as_bytes().to_owned(), PaddingDirection::Left, Ok(string[0..4].as_bytes().to_owned()));
        padder.add_pad_call("hello2".as_bytes().to_owned(), 5, " ".as_bytes().to_owned(), PaddingDirection::Right, Ok(string[4..9].as_bytes().to_owned()));
        let writer = WriterBuilder::new().with_padder(&padder).with_specs(spec.record_specs).build();
        assert_result!(
            Ok(()),
            writer.write_field(&mut buf, "hello".as_bytes(), "record1", "field1")
        );
        assert_eq!(string[0..4].to_string(), String::from_utf8(buf.get_ref().clone()).unwrap());
        assert_result!(
            Ok(()),
            writer.write_field(&mut buf, "hello2".as_bytes(), "record1", "field2")
        );
        assert_eq!(string, String::from_utf8(buf.into_inner()).unwrap());
    }

    #[test]
    fn write_field_with_bad_record_name() {
        let spec = test_spec();
        let mut buf = Cursor::new(Vec::new());
        let padder = MockPadder::new();
        let writer = WriterBuilder::new().with_padder(&padder).with_specs(spec.record_specs).build();
        assert_result!(
            Err(Error::RecordSpecNotFound(ref record_name)) if record_name == "record5",
            writer.write_field(&mut buf, "hello".as_bytes(), "record5", "field1")
        );
    }

    #[test]
    fn write_field_with_bad_field_name() {
        let spec = test_spec();
        let mut buf = Cursor::new(Vec::new());
        let padder = MockPadder::new();
        let writer = WriterBuilder::new().with_padder(&padder).with_specs(&spec.record_specs).build();
        assert_result!(
            Err(Error::FieldSpecNotFound(ref record_name, ref field_name)) if record_name == "record1" && field_name == "field5",
            writer.write_field(&mut buf, "hello".as_bytes(), "record1", "field5")
        );
    }

    #[test]
    fn write_field_with_padding_error() {
        let spec = test_spec();
        let mut buf = Cursor::new(Vec::new());
        let mut padder = MockPadder::new();
        padder.add_pad_call("hello".as_bytes().to_owned(), 4, "dsasd".as_bytes().to_owned(), PaddingDirection::Left, Err(PaddingError::new("")));
        let writer = WriterBuilder::new().with_padder(&padder).with_specs(spec.record_specs).build();
        assert_result!(
            Err(Error::PadderFailure(_)),
            writer.write_field(&mut buf, "hello".as_bytes(), "record1", "field1")
        );
    }

    #[test]
    fn write_field_with_padded_value_not_correct_length() {
        let spec = test_spec();
        let mut buf = Cursor::new(Vec::new());
        let mut padder = MockPadder::new();
        padder.add_pad_call("hello".as_bytes().to_owned(), 4, "dsasd".as_bytes().to_owned(), PaddingDirection::Left, Ok("123".as_bytes().to_owned()));
        let writer = WriterBuilder::new().with_padder(&padder).with_specs(spec.record_specs).build();
        assert_result!(
            Err(Error::PaddedValueWrongLength(4, ref value)) if *value == "123".as_bytes().to_owned(),
            writer.write_field(&mut buf, "hello".as_bytes(), "record1", "field1")
        );
    }

    #[test]
    fn write_field_with_write_error() {
        let spec = test_spec();
        let string: &mut [u8] = &mut [0; 3];
        let mut buf = Cursor::new(string);
        let mut padder = MockPadder::new();
        padder.add_pad_call("hello".as_bytes().to_owned(), 4, "dsasd".as_bytes().to_owned(), PaddingDirection::Left, Ok("1234".as_bytes().to_owned()));
        let writer = WriterBuilder::new().with_padder(padder).with_specs(spec.record_specs).build();
        assert_result!(
            Err(Error::IoError(_)),
            writer.write_field(&mut buf, "hello".as_bytes(), "record1", "field1")
        );
    }
}