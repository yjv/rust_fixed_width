use spec::{RecordSpec, FieldSpec};
use padder::{Padder, IdentityPadder};
use std::collections::{HashMap};
use std::io::Write;
use std::borrow::Borrow;
use recognizer::{DataRecordSpecRecognizer, NoneRecognizer};
use super::{Error, Result, PositionalResult, Record};
use record::{Data, DataRanges, WriteDataHolder, FieldData, WriteType, BinaryType};

pub struct Writer<T: Padder<W>, U: DataRecordSpecRecognizer<W>, V: Borrow<HashMap<String, RecordSpec>>, W: WriteType> {
    padder: T,
    recognizer: U,
    specs: V,
    write_type: W
}

impl<T: Padder<W>, U: DataRecordSpecRecognizer<W>, V: Borrow<HashMap<String, RecordSpec>>, W: WriteType> Writer<T, U, V, W> {
    pub fn write_field<'a, X>(&self, writer: &'a mut X, value: &'a [u8], record_name: &'a str, name: &'a str) -> Result<()>
        where X: 'a + Write
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

    pub fn write_record<'a, X, Y, Z, A>(&self, writer: &'a mut X, record: Y, record_name: Z) -> PositionalResult<()>
        where X: 'a + Write,
              Y: Into<(&'a Data<A, W::DataHolder>, Option<&'a str>)>,
              Z: Into<Option<&'a str>>,
              A: DataRanges + 'a,
              W::DataHolder: 'a
    {
        let data_and_record_name = record.into();
        let (data, record_name) = (
            self.write_type.downcast_data(data_and_record_name.0)?,
            record_name.into().or(data_and_record_name.1)
        );
        let record_name = record_name
            .map_or_else(
                || self.recognizer.recognize_for_data(&data, self.specs.borrow(), &self.write_type),
                |name| Ok(name)
            )?
        ;
        let record_spec = self.specs.borrow()
            .get(record_name)
            .ok_or_else(|| Error::RecordSpecNotFound(record_name.to_owned()))?
        ;

        for (name, field_spec) in &record_spec.field_specs {
            self._write_field(
                writer,
                field_spec,
                data.get_write_data(name)
                    .or_else(|| field_spec.default.as_ref().map(|v| &v[..]))
                    .ok_or_else(|| (Error::FieldValueRequired, record_name.to_owned(), name.clone()))?
            ).map_err(|e| (e, record_name.to_owned(), name.clone()))?;
        }

        self.write_line_ending(writer, &record_spec.line_ending).map_err(|e| (e, record_name.to_owned()))?;

        Ok(())
    }

    pub fn write_line_ending<'a, X: 'a + Write>(&self, writer: &'a mut X, line_ending: &'a [u8]) -> Result<()> {
        writer.write(line_ending)?;
        Ok(())
    }

    fn _write_field<'a, X: 'a + Write>(&self, writer: &'a mut X, field_spec: &FieldSpec, value: &'a [u8]) -> Result<()> {
        let mut destination = Vec::new();
        self.padder.pad(value, field_spec.length, &field_spec.padding, field_spec.padding_direction, &mut destination, &self.write_type)?;
        if destination.len() != field_spec.length {
            return Err(Error::PaddedValueWrongLength(field_spec.length, destination));
        }

        Ok(writer.write_all(&destination[..])?)
    }
}

pub struct WriterBuilder<T: Padder<W>, U: DataRecordSpecRecognizer<W>, V: Borrow<HashMap<String, RecordSpec>>, W: WriteType> {
    padder: T,
    recognizer: U,
    specs: Option<V>,
    write_type: W
}

impl<V: Borrow<HashMap<String, RecordSpec>>> WriterBuilder<IdentityPadder, NoneRecognizer, V, BinaryType> {
    pub fn new() -> WriterBuilder<IdentityPadder, NoneRecognizer, V, BinaryType> {
        WriterBuilder {
            padder: IdentityPadder,
            recognizer: NoneRecognizer,
            specs: None,
            write_type: BinaryType
        }
    }
}

impl<T: Padder<W>, U: DataRecordSpecRecognizer<W>, V: Borrow<HashMap<String, RecordSpec>>, W: WriteType> WriterBuilder<T, U, V, W> {
    pub fn with_padder<X: Padder<W>>(self, padder: X) -> WriterBuilder<X, U, V, W> {
        WriterBuilder {
            padder: padder,
            recognizer: self.recognizer,
            specs: self.specs,
            write_type: self.write_type
        }
    }

    pub fn with_recognizer<X: DataRecordSpecRecognizer<W>>(self, recognizer: X) -> WriterBuilder<T, X, V, W> {
        WriterBuilder {
            padder: self.padder,
            recognizer: recognizer,
            specs: self.specs,
            write_type: self.write_type
        }
    }

    pub fn with_specs(mut self, specs: V) -> Self {
        self.specs = Some(specs);
        self
    }

    pub fn with_write_type<X: WriteType>(self, write_type: X) -> WriterBuilder<T, U, V, X>
        where T: Padder<X>,
              U: DataRecordSpecRecognizer<X>
    {
        WriterBuilder {
            padder: self.padder,
            recognizer: self.recognizer,
            specs: self.specs,
            write_type: write_type
        }
    }

    pub fn build(self) -> Writer<T, U, V, W> {
        Writer {
            padder: self.padder,
            recognizer: self.recognizer,
            specs: self.specs.expect("specs is required to build a writer"),
            write_type: self.write_type
        }
    }
}

impl<'a, T: DataRanges + 'a, U> Into<(&'a Data<T, U>, Option<&'a str>)> for &'a Data<T, U> {
    fn into(self) -> (&'a Data<T, U>, Option<&'a str>) {
        (self, None)
    }
}

impl<'a, T: DataRanges + 'a, U> Into<(&'a Data<T, U>, Option<&'a str>)> for &'a Record<T, U> {
    fn into(self) -> (&'a Data<T, U>, Option<&'a str>) {
        (&self.data, Some(&self.name[..]))
    }
}

pub trait FieldFormatter<T: WriteType> {
    fn format<'a>(&self, data: &[u8], field_spec: &'a FieldSpec, destination: &'a mut Vec<u8>, write_type: &'a T) -> Result<()>;
}

pub trait DataWriter<T: WriteType> {
    fn write<'a, U: Write + 'a>(&mut self, writer: &'a mut U, source: &'a [u8], amount: usize, write_type: &'a T) -> Result<()>;
}

pub struct FieldWriter<T: DataWriter<V>, U: FieldFormatter<V>, V: WriteType> {
    buffer: Vec<u8>,
    data_writer: T,
    post_processor: U,
    write_type: V
}

impl <T: DataWriter<V>, U: FieldFormatter<V>, V: WriteType> FieldWriter<T, U, V> {
    pub fn write<'a, W>(&mut self, writer: &'a mut W, spec: &'a FieldSpec, data: &'a [u8]) -> Result<usize>
        where W: Write + 'a

    {
        self.post_processor.format(data, spec, &mut self.buffer, &self.write_type)?;
        self.data_writer.write(writer, &self.buffer[..], spec.length, &self.write_type)?;
        let amount = self.buffer.len();
        self.buffer.clear();
        Ok(amount)
    }
}

pub struct RecordWriter<T: DataWriter<V>, U: FieldFormatter<V>, V: WriteType> {
    buffer: Vec<u8>,
    data_writer: T,
    post_processor: U,
    write_type: V
}

impl <T: DataWriter<V>, U: FieldFormatter<V>, V: WriteType> RecordWriter<T, U, V> {
    pub fn write<'a, W, X>(&mut self, writer: &'a mut W, spec: &'a RecordSpec, data: &'a Data<X, V::DataHolder>) -> PositionalResult<()>
        where W: Write + 'a,
              X: DataRanges + 'a
    {
        let (ranges, data) = (&data.ranges, &data.data);

        for (name, field_spec) in &spec.field_specs {
            let field_data = ranges.get(name)
                .map(|range| data.get(range).expect("badly built record data somehow has value missing for ranges entry"))
                .or_else(|| field_spec.default.as_ref().map(|v| &v[..]))
                .ok_or_else(|| (Error::FieldValueRequired, name.clone()))?
            ;
            self.post_processor.format(field_data, field_spec, &mut self.buffer, &self.write_type)?;
            self.data_writer.write(writer, &self.buffer[..], field_spec.length, &self.write_type)?;
        }

        Ok(())
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
            .iter().cloned().collect::<Data<_, _>>();
        let mut recognizer = MockRecognizer::<BTreeMap<String, Range<usize>>>::new();
        recognizer.add_data_recognize_call(data.internal_references(), &spec.record_specs, Ok("record1"));
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