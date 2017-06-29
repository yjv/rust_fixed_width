pub mod formatter;
pub mod spec;

use spec::{RecordSpec, FieldSpec};
use std::collections::{HashMap};
use std::io::Write;
use std::borrow::Borrow;
use error::Error;
use super::{Result, PositionalResult, FieldResult};
use record::{Data, DataRanges};
use data_type::WriteSupport;
use self::formatter::FieldFormatter;
use std::borrow::BorrowMut;
use self::spec::Stream as SpecSource;

pub struct FieldWriter<'a, T: FieldFormatter<U> + 'a, U: WriteSupport> {
    formatter: T,
    write_support: U,
    lifetime: ::std::marker::PhantomData<&'a ()>
}

impl<'a, T: FieldFormatter<U> + 'a, U: WriteSupport> FieldWriter<'a, T, U> {
    pub fn new(formatter: T, write_support: U) -> FieldWriter<'a, T, U> {
        FieldWriter {
            formatter: formatter,
            write_support: write_support,
            lifetime: ::std::marker::PhantomData
        }
    }

    pub fn write_support(&self) -> &U {
        &self.write_support
    }
}

impl <'a, T: FieldFormatter<U> + 'a, U: WriteSupport> FieldWriter<'a, T, U> {
    pub fn write<'b, V>(&self, writer: &'b mut V, spec: &'b FieldSpec, data: &'b [u8], buffer: &'b mut Vec<u8>) -> Result<usize>
        where V: Write + 'b
    {
        buffer.clear();
        self.formatter.format(data, spec, buffer, &self.write_support).map_err(Error::FormatterFailure)?;

        let length = self.write_support.get_length(&buffer[..]);

        if length.length != spec.length || length.remainder > 0 {
            return Err(Error::FormattedValueWrongLength(spec.length, buffer.clone()).into());
        }

        writer.write_all(&buffer[..])?;

        Ok(buffer.len())
    }
}

pub struct RecordWriter<'a, T: FieldFormatter<U> + 'a, U: WriteSupport> {
    field_writer: FieldWriter<'a, T, U>
}

impl<'a, T: FieldFormatter<U> + 'a, U: WriteSupport> RecordWriter<'a, T, U> {
    pub fn new(field_writer: FieldWriter<'a, T, U>) -> RecordWriter<'a, T, U> {
        RecordWriter {
            field_writer: field_writer
        }
    }

    pub fn write_support(&self) -> &U {
        self.field_writer.write_support()
    }
}

impl <'a, T: FieldFormatter<U> + 'a, U: WriteSupport> RecordWriter<'a, T, U> {
    pub fn write<'b, V, W>(&self, writer: &'b mut V, spec: &'b RecordSpec, data: &'b Data<W, U::DataHolder>, buffer: &mut Vec<u8>) -> FieldResult<usize>
        where V: Write + 'b,
              W: DataRanges + 'b
    {
        let mut amount_written = 0;

        for (name, field_spec) in &spec.field_specs {
            let field_data = self.field_writer.write_support().get_data_by_name(name, data)
                .or_else(|| field_spec.default.as_ref().map(|v| &v[..]))
                .ok_or_else(|| (Error::FieldValueRequired, name))?
            ;
            amount_written += self.field_writer.write(writer, field_spec, field_data, buffer).map_err(|e| (e, name))?;
        }

        writer.write_all(&spec.line_ending[..])?;

        Ok(amount_written + spec.line_ending.len())
    }
}

pub struct Writer<
    'a,
    R: Write + 'a,
    T: FieldFormatter<V> + 'a,
    U: SpecSource<V> + 'a,
    V: WriteSupport,
    W: Borrow<HashMap<String, RecordSpec>> + 'a,
    X: BorrowMut<R> + 'a,
    Y: BorrowMut<Vec<u8>> + 'a
> {
    destination: X,
    writer: RecordWriter<'a, T, V>,
    spec_source: U,
    record_specs: W,
    buffer: Y,
    destination_type: ::std::marker::PhantomData<&'a R>
}

impl<'a, R, T, U, V, W, X, Y> Writer<'a, R, T, U, V, W, X, Y>
    where R: Write + 'a,
          T: FieldFormatter<V> + 'a,
          U: SpecSource<V> + 'a,
          V: WriteSupport,
          W: Borrow<HashMap<String, RecordSpec>> + 'a,
          X: BorrowMut<R> + 'a,
          Y: BorrowMut<Vec<u8>> + 'a {
    pub fn write_record<'b, A: DataRanges + 'b>(&mut self, data: &'b Data<A, V::DataHolder>) -> PositionalResult<usize> {
        let spec_name = self.spec_source.next(data, self.record_specs.borrow(), self.writer.write_support())
            .map_err(Error::SpecStreamError)?
            .ok_or(Error::SpecStreamReturnedNone)?
        ;
        self.writer
            .write(
                self.destination.borrow_mut(),
                self.record_specs.borrow().get(spec_name).ok_or_else(|| Error::RecordSpecNotFound(spec_name.to_string()))?,
                data,
                self.buffer.borrow_mut()
            )
            .map_err(|e| (e, spec_name).into())
    }

    pub fn into_inner(self) -> RecordWriter<'a, T, V> {
        self.writer
    }
}

pub struct WriterBuilder<
    'a,
    WR: Write + 'a,
    T: FieldFormatter<V> + 'a,
    U: SpecSource<V> + 'a,
    V: WriteSupport,
    W: Borrow<HashMap<String, RecordSpec>> + 'a,
    X: BorrowMut<WR> + 'a,
    Y: BorrowMut<Vec<u8>> + 'a
> {
    destination: Option<X>,
    field_formatter: Option<T>,
    write_support: V,
    spec_source: Option<U>,
    record_specs: Option<W>,
    buffer: Y,
    destination_type: ::std::marker::PhantomData<&'a WR>
}

impl<'a, WR, T, U, V, W, X> WriterBuilder<'a, WR, T, U, V, W, X, Vec<u8>>
    where WR: Write + 'a,
          T: FieldFormatter<V> + 'a,
          U: SpecSource<V> + 'a,
          V: WriteSupport,
          W: Borrow<HashMap<String, RecordSpec>> + 'a,
          X: BorrowMut<WR> + 'a {
    pub fn new(write_support: V) -> Self {
        WriterBuilder {
            destination: None,
            field_formatter: None,
            write_support: write_support,
            spec_source: None,
            record_specs: None,
            buffer: Vec::new(),
            destination_type: ::std::marker::PhantomData
        }
    }
}

impl<'a, WR, T, U, V, W, X, Y> WriterBuilder<'a, WR, T, U, V, W, X, Y>
    where WR: Write + 'a,
          T: FieldFormatter<V> + 'a,
          U: SpecSource<V> + 'a,
          V: WriteSupport,
          W: Borrow<HashMap<String, RecordSpec>> + 'a,
          X: BorrowMut<WR> + 'a,
          Y: BorrowMut<Vec<u8>> + 'a {
    pub fn with_source<Z: Write + 'a, A: BorrowMut<Z> + 'a>(self, destination: A) -> WriterBuilder<'a, Z, T, U, V, W, A, Y> {
        WriterBuilder {
            destination: Some(destination),
            write_support: self.write_support,
            field_formatter: self.field_formatter,
            spec_source: self.spec_source,
            record_specs: self.record_specs,
            buffer: self.buffer,
            destination_type: ::std::marker::PhantomData
        }
    }

    pub fn with_field_formatter<Z: FieldFormatter<V> + 'a>(self, field_formatter: Z) -> WriterBuilder<'a, WR, Z, U, V, W, X, Y> {
        WriterBuilder {
            destination: self.destination,
            write_support: self.write_support,
            field_formatter: Some(field_formatter),
            spec_source: self.spec_source,
            record_specs: self.record_specs,
            buffer: self.buffer,
            destination_type: ::std::marker::PhantomData
        }
    }

    pub fn with_spec_source<Z: SpecSource<V> + 'a>(self, spec_source: Z) -> WriterBuilder<'a, WR, T, Z, V, W, X, Y> {
        WriterBuilder {
            destination: self.destination,
            write_support: self.write_support,
            field_formatter: self.field_formatter,
            spec_source: Some(spec_source),
            record_specs: self.record_specs,
            buffer: self.buffer,
            destination_type: ::std::marker::PhantomData
        }
    }

    pub fn with_record_specs<Z: Borrow<HashMap<String, RecordSpec>> + 'a>(self, record_specs: Z) -> WriterBuilder<'a, WR, T, U, V, Z, X, Y> {
        WriterBuilder {
            destination: self.destination,
            write_support: self.write_support,
            field_formatter: self.field_formatter,
            spec_source: self.spec_source,
            record_specs: Some(record_specs),
            buffer: self.buffer,
            destination_type: ::std::marker::PhantomData
        }
    }

    pub fn with_buffer<Z: BorrowMut<Vec<u8>> + 'a>(self, buffer: Z) -> WriterBuilder<'a, WR, T, U, V, W, X, Z> {
        WriterBuilder {
            destination: self.destination,
            write_support: self.write_support,
            field_formatter: self.field_formatter,
            spec_source: self.spec_source,
            record_specs: self.record_specs,
            buffer: buffer,
            destination_type: ::std::marker::PhantomData
        }
    }

    pub fn build(self) -> Result<Writer<'a, WR, T, U, V, W, X, Y>> {
        Ok(Writer {
            destination: self.destination.ok_or(Error::FieldRequiredToBuild("source needs to be defined in order to build"))?,
            writer: RecordWriter::new(FieldWriter::new(
                self.field_formatter.ok_or(Error::FieldRequiredToBuild("field_formatter needs to be defined in order to build"))?,
                self.write_support
            )),
            spec_source: self.spec_source.ok_or(Error::FieldRequiredToBuild("spec_source needs to be defined in order to build"))?,
            record_specs: self.record_specs.ok_or(Error::FieldRequiredToBuild("record_specs needs to be defined in order to build"))?,
            buffer: self.buffer,
            destination_type: ::std::marker::PhantomData
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use record::Data;
    use error::{Error, FieldError};
    use test::*;
    use std::collections::{HashMap, BTreeMap};
    use std::io::Cursor;
    use data_type::BinarySupport;

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
        let writer = RecordWriter::new(FieldWriter::new(&formatter, BinarySupport));
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
        formatter.add_format_call("hello".as_bytes().to_owned(), record_spec.field_specs.get("field1").unwrap().clone(), Err("".into()));
        let writer = RecordWriter::new(FieldWriter::new(&formatter, BinarySupport));
        assert_result!(
            Err(FieldError {
                error: Error::FormatterFailure(_),
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
        let writer = RecordWriter::new(FieldWriter::new(MockFormatter::new(), BinarySupport));
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
        let writer = RecordWriter::new(FieldWriter::new(&formatter, BinarySupport));
        assert_result!(
            Err(FieldError {
                error: Error::FormattedValueWrongLength(4, ref value),
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
        let writer = RecordWriter::new(FieldWriter::new(&formatter, BinarySupport));
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
        let writer = FieldWriter::new(&formatter, BinarySupport);
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
        formatter.add_format_call("hello".as_bytes().to_owned(), record_spec.field_specs.get("field1").unwrap().clone(), Err("".into()));
        let writer = FieldWriter::new(&formatter, BinarySupport);
        assert_result!(
            Err(Error::FormatterFailure(_)),
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
        let writer = FieldWriter::new(&formatter, BinarySupport);
        assert_result!(
            Err(Error::FormattedValueWrongLength(4, ref value)) if *value == "hello2".as_bytes().to_owned(),
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
        let writer = FieldWriter::new(&formatter, BinarySupport);
        assert_result!(
            Err(Error::IoError(_)),
            writer.write(&mut buf, record_spec.field_specs.get("field1").unwrap(), "hello".as_bytes(), &mut Vec::new())
        );
    }
}