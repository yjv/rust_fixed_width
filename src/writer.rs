use spec::{RecordSpec, FieldSpec};
use padder::{Padder, IdentityPadder};
use std::collections::{HashMap};
use std::io::Write;
use std::borrow::Borrow;
use recognizer::{DataRecordSpecRecognizer, NoneRecognizer};
use super::{Error, Result, PositionalResult, Record, FieldResult};
use record::{Data, DataRanges, WriteDataHolder, WriteType, BinaryType, Length};

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
        let record_name = record_name.unwrap();
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

        self.write_line_ending(writer, &record_spec.line_ending).map_err(|e| (e, record_name))?;

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
    fn format<'a>(&self, data: &'a [u8], field_spec: &'a FieldSpec, destination: &'a mut Vec<u8>, write_type: &'a T) -> Result<()>;
}

impl<'a, T, U: WriteType> FieldFormatter<U> for &'a T where T: 'a + FieldFormatter<U> {
    fn format<'b>(&self, data: &'b [u8], field_spec: &'b FieldSpec, destination: &'b mut Vec<u8>, write_type: &'b U) -> Result<()> {
        (**self).format(data, field_spec, destination, write_type)
    }
}

pub struct FieldWriter<T: FieldFormatter<U>, U: WriteType> {
    formatter: T,
    write_type: U
}

impl<T: FieldFormatter<U>, U: WriteType> FieldWriter<T, U> {
    pub fn new(formatter: T, write_type: U) -> FieldWriter<T, U> {
        FieldWriter {
            formatter: formatter,
            write_type: write_type
        }
    }

    pub fn write_type(&self) -> &U {
        &self.write_type
    }
}

impl <T: FieldFormatter<U>, U: WriteType> FieldWriter<T, U> {
    pub fn write<'a, V>(&self, writer: &'a mut V, spec: &'a FieldSpec, data: &'a [u8], buffer: &'a mut Vec<u8>) -> Result<usize>
        where V: Write + 'a
    {
        buffer.clear();
        self.formatter.format(data, spec, buffer, &self.write_type)?;

        let length = self.write_type.get_length(&buffer[..]);

        if length.length != spec.length || length.remainder > 0 {
            return Err(Error::PaddedValueWrongLength(spec.length, buffer.clone()).into());
        }

        writer.write_all(&buffer[..])?;

        Ok(buffer.len())
    }
}

pub struct RecordWriter<T: FieldFormatter<U>, U: WriteType> {
    field_writer: FieldWriter<T, U>
}

impl<T: FieldFormatter<U>, U: WriteType> RecordWriter<T, U> {
    pub fn new(field_writer: FieldWriter<T, U>) -> RecordWriter<T, U> {
        RecordWriter {
            field_writer: field_writer
        }
    }
}

impl <T: FieldFormatter<U>, U: WriteType> RecordWriter<T, U> {
    pub fn write<'a, V, W>(&self, writer: &'a mut V, spec: &'a RecordSpec, data: &'a Data<W, U::DataHolder>, buffer: &mut Vec<u8>) -> FieldResult<usize>
        where V: Write + 'a,
              W: DataRanges + 'a
    {
        let mut amount_written = 0;

        for (name, field_spec) in &spec.field_specs {
            let field_data = self.field_writer.write_type().get_data_by_name(name, data)
                .or_else(|| field_spec.default.as_ref().map(|v| &v[..]))
                .ok_or_else(|| (Error::FieldValueRequired, name))?
            ;
            amount_written += self.field_writer.write(writer, field_spec, field_data, buffer).map_err(|e| (e, name))?;
        }

        writer.write_all(&spec.line_ending[..])?;

        Ok(amount_written + spec.line_ending.len())
    }
}

pub struct RecordRecognizer<T: DataRecordSpecRecognizer<U>, U: WriteType> {
    recognizer: T,
    write_type: U
}

impl<T: DataRecordSpecRecognizer<U>, U: WriteType> RecordRecognizer<T, U> {
    pub fn recognize<'a, V: DataRanges + 'a>(&self, data: &'a Data<V, U::DataHolder>, record_specs: &'a HashMap<String, RecordSpec>) -> Result<&'a str> {
        Ok(self.recognizer.recognize_for_data(data, record_specs, &self.write_type)?)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use super::super::{Error, PositionalError, Position, Record, Data, FieldError};
    use test::*;
    use std::collections::{HashMap, BTreeMap};
    use std::io::{Cursor, Write};
    use spec::PaddingDirection;
    use padder::Error as PaddingError;
    use std::ops::Range;
    use record::{BinaryType, WriteType};

    #[test]
    fn write_record() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;zxcvbnm,./-=[];\n".to_string();
        let mut buf = Cursor::new(Vec::new());
        let mut formatter = MockFormatter::new();
        let record_spec = &spec.record_specs.get("record1").unwrap();
        formatter.add_format_call("hello".as_bytes().to_owned(), record_spec.field_specs.get("field1").unwrap().clone(), Ok(string[0..4].as_bytes().to_owned()));
        formatter.add_format_call("def".as_bytes().to_owned(), record_spec.field_specs.get("field2").unwrap().clone(), Ok(string[4..9].as_bytes().to_owned()));
        formatter.add_format_call("hello2".as_bytes().to_owned(), record_spec.field_specs.get("field3").unwrap().clone(), Ok(string[9..45].as_bytes().to_owned()));
        let writer = RecordWriter::new(FieldWriter::new(&formatter, BinaryType));
        writer.write(&mut buf, record_spec, &Data::from([("field1".to_string(), "hello".as_bytes().to_owned()),
            ("field3".to_string(), "hello2".as_bytes().to_owned())]
            .iter().cloned().collect::<HashMap<_, _>>()), &mut Vec::new()).unwrap();
        assert_eq!(string, String::from_utf8(buf.into_inner()).unwrap());
    }

    #[test]
    fn write_record_with_formatting_error() {
        let spec = test_spec();
        let mut buf = Cursor::new(Vec::new());
        let mut formatter = MockFormatter::new();
        let record_spec = &spec.record_specs.get("record1").unwrap();
        formatter.add_format_call("hello".as_bytes().to_owned(), record_spec.field_specs.get("field1").unwrap().clone(), Err(Error::CouldNotReadEnough(Vec::new())));
        let writer = RecordWriter::new(FieldWriter::new(&formatter, BinaryType));
        assert_result!(
            Err(FieldError {
                error: Error::RecordSpecNameRequired,
                field: Some(ref field)
            }) if field == "field1",
            writer.write(&mut buf, record_spec, &Data::from([("field1".to_string(), "hello".as_bytes().to_owned())]
                .iter().cloned().collect::<BTreeMap<_, _>>()), &mut Vec::new())
        );
    }

    #[test]
    fn write_record_with_field_require_error() {
        let spec = test_spec();
        let mut buf = Cursor::new(Vec::new());
        let record_spec = &spec.record_specs.get("record1").unwrap();
        let writer = RecordWriter::new(FieldWriter::new(MockFormatter::new(), BinaryType));
        assert_result!(
            Err(FieldError {
                error: Error::FieldValueRequired,
                field: Some(ref field)
            }) if field == "field1",
            writer.write(&mut buf, record_spec, &Data::from([("field3".to_string(), "hello".as_bytes().to_owned())]
                .iter().cloned().collect::<BTreeMap<_, _>>()), &mut Vec::new())
        );
    }

    #[test]
    fn write_record_with_formatted_value_not_correct_length() {
        let spec = test_spec();
        let mut buf = Cursor::new(Vec::new());
        let mut formatter = MockFormatter::new();
        let record_spec = &spec.record_specs.get("record1").unwrap();
        formatter.add_format_call("hello".as_bytes().to_owned(), record_spec.field_specs.get("field1").unwrap().clone(), Ok("hello2".as_bytes().to_owned()));
        let writer = RecordWriter::new(FieldWriter::new(&formatter, BinaryType));
        assert_result!(
            Err(FieldError {
                error: Error::PaddedValueWrongLength(4, ref value),
                field: Some(ref field)
            }) if *value == "hello2".as_bytes().to_owned() && field == "field1",
            writer.write(&mut buf, record_spec, &Data::from([("field1".to_string(), "hello".as_bytes().to_owned())]
                .iter().cloned().collect::<BTreeMap<_, _>>()), &mut Vec::new())
        );
    }

    #[test]
    fn write_record_with_write_error() {
        let spec = test_spec();
        let string: &mut [u8] = &mut [0; 3];
        let mut buf = Cursor::new(string);
        let mut formatter = MockFormatter::new();
        let record_spec = &spec.record_specs.get("record1").unwrap();
        formatter.add_format_call("hello".as_bytes().to_owned(), record_spec.field_specs.get("field1").unwrap().clone(), Ok("bye2".as_bytes().to_owned()));
        let writer = RecordWriter::new(FieldWriter::new(&formatter, BinaryType));
        assert_result!(
            Err(FieldError {
                error: Error::IoError(_),
                field: Some(ref field)
            }) if field == "field1",
            writer.write(&mut buf, record_spec, &Data::from([("field1".to_string(), "hello".as_bytes().to_owned())]
                .iter().cloned().collect::<BTreeMap<_, _>>()), &mut Vec::new())
        );
    }

    #[test]
    fn write_field() {
        let spec = test_spec();
        let string = "123456789".to_string();
        let mut buf = Cursor::new(Vec::new());
        let mut formatter = MockFormatter::new();
        let record_spec = &spec.record_specs.get("record1").unwrap();
        formatter.add_format_call("hello".as_bytes().to_owned(), record_spec.field_specs.get("field1").unwrap().clone(), Ok(string[0..4].as_bytes().to_owned()));
        formatter.add_format_call("hello2".as_bytes().to_owned(), record_spec.field_specs.get("field2").unwrap().clone(), Ok(string[4..9].as_bytes().to_owned()));
        let writer = FieldWriter::new(&formatter, BinaryType);
        assert_result!(
            Ok(4),
            writer.write(&mut buf, record_spec.field_specs.get("field1").unwrap(), "hello".as_bytes(), &mut Vec::new())
        );
        assert_eq!(string[0..4].to_string(), String::from_utf8(buf.get_ref().clone()).unwrap());
        assert_result!(
            Ok(5),
            writer.write(&mut buf, record_spec.field_specs.get("field2").unwrap(), "hello2".as_bytes(), &mut Vec::new())
        );
        assert_eq!(string, String::from_utf8(buf.into_inner()).unwrap());
    }

    #[test]
    fn write_field_with_formatting_error() {
        let spec = test_spec();
        let mut buf = Cursor::new(Vec::new());
        let mut formatter = MockFormatter::new();
        let record_spec = &spec.record_specs.get("record1").unwrap();
        formatter.add_format_call("hello".as_bytes().to_owned(), record_spec.field_specs.get("field1").unwrap().clone(), Err(Error::CouldNotReadEnough(Vec::new())));
        let writer = FieldWriter::new(&formatter, BinaryType);
        assert_result!(
            Err(Error::RecordSpecNameRequired),
            writer.write(&mut buf, record_spec.field_specs.get("field1").unwrap(), "hello".as_bytes(), &mut Vec::new())
        );
    }

    #[test]
    fn write_field_with_padded_value_not_correct_length() {
        let spec = test_spec();
        let mut buf = Cursor::new(Vec::new());
        let mut formatter = MockFormatter::new();
        let record_spec = &spec.record_specs.get("record1").unwrap();
        formatter.add_format_call("hello".as_bytes().to_owned(), record_spec.field_specs.get("field1").unwrap().clone(), Ok("hello2".as_bytes().to_owned()));
        let writer = FieldWriter::new(&formatter, BinaryType);
        assert_result!(
            Err(Error::PaddedValueWrongLength(4, ref value)) if *value == "hello2".as_bytes().to_owned(),
            writer.write(&mut buf, record_spec.field_specs.get("field1").unwrap(), "hello".as_bytes(), &mut Vec::new())
        );
    }

    #[test]
    fn write_field_with_write_error() {
        let spec = test_spec();
        let string: &mut [u8] = &mut [0; 3];
        let mut buf = Cursor::new(string);
        let mut formatter = MockFormatter::new();
        let record_spec = &spec.record_specs.get("record1").unwrap();
        formatter.add_format_call("hello".as_bytes().to_owned(), record_spec.field_specs.get("field1").unwrap().clone(), Ok("bye2".as_bytes().to_owned()));
        let writer = FieldWriter::new(&formatter, BinaryType);
        assert_result!(
            Err(Error::IoError(_)),
            writer.write(&mut buf, record_spec.field_specs.get("field1").unwrap(), "hello".as_bytes(), &mut Vec::new())
        );
    }
}