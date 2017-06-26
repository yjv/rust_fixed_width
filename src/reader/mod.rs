pub mod parser;
pub mod spec;
pub mod field_buffer;

use spec::{RecordSpec, FieldSpec};
use std::collections::{HashMap};
use std::io::{Read, BufRead};
use std::borrow::{Borrow, BorrowMut};
use error::Error;
use super::{Result, PositionalResult, FieldResult, Record};
use record::{Data, BuildableDataRanges};
use data_type::{FieldReadSupport, RecordReadSupport, ShouldReadMore};
use reader::parser::FieldParser;
use self::spec::Stream as SpecSource;
use self::field_buffer::Source as FieldBufferSource;

pub struct FieldReader<'a, T: FieldParser<U> + 'a, U: FieldReadSupport> {
    parser: T,
    read_support: U,
    lifetime: ::std::marker::PhantomData<&'a ()>
}

impl<'a, T: FieldParser<U> + 'a, U: FieldReadSupport> FieldReader<'a, T, U> {
    pub fn new(parser: T, read_support: U) -> FieldReader<'a, T, U> {
        FieldReader {
            parser: parser,
            read_support: read_support,
            lifetime: ::std::marker::PhantomData,
        }
    }

    pub fn read_support(&self) -> &U {
        &self.read_support
    }
}

impl <'a, T: FieldParser<U> + 'a, U: FieldReadSupport> FieldReader<'a, T, U> {
    pub fn read<'b, V>(&self, reader: &'b mut V, field_spec: &'b FieldSpec, field_buffer: &'b mut Vec<u8>, buffer: &'b mut Vec<u8>) -> Result<()>
        where V: Read + 'b
    {
        buffer.clear();
        while let ShouldReadMore::More(amount) = self.read_support.should_read_more(field_spec.length, &buffer[..]) {
            let amount_read = reader.by_ref().take(amount as u64).read_to_end(buffer)?;

            if amount_read != amount {
                return Err(Error::CouldNotReadEnough(buffer.clone()))
            }
        }

        self.parser.parse(&buffer[..], field_spec, field_buffer, &self.read_support).map_err(Error::ParserFailure)?;

        Ok(())
    }
}

pub struct RecordReader<'a, T: FieldParser<U> + 'a, U: RecordReadSupport> {
    field_reader: FieldReader<'a, T, U>
}

impl<'a, T: FieldParser<U> + 'a, U: RecordReadSupport> RecordReader<'a, T, U> {
    pub fn new(field_writer: FieldReader<'a, T, U>) -> RecordReader<T, U> {
        RecordReader {
            field_reader: field_writer
        }
    }

    pub fn read_support(&self) -> &U {
        self.field_reader.read_support()
    }

    pub fn into_inner(self) -> FieldReader<'a, T, U> {
        self.field_reader
    }
}

impl <'a, T: FieldParser<U> + 'a, U: RecordReadSupport> RecordReader<'a, T, U> {
    pub fn read<'b, V, X>(&self, reader: &'b mut V, spec: &'b RecordSpec, mut field_buffer: Vec<u8>, buffer: &'b mut Vec<u8>) -> FieldResult<Data<X, U::DataHolder>>
        where V: Read + 'b,
              X: BuildableDataRanges + 'b
    {
        let mut ranges = X::new();
        for (name, field_spec) in &spec.field_specs {
            let old_length = field_buffer.len();
            self.field_reader.read(reader, field_spec, &mut field_buffer, buffer).map_err(|e| (e, name))?;

            ranges.insert(name, self.field_reader.read_support().get_range(
                old_length,
                &field_buffer[..]
            ));
        }

        buffer.clear();

        if reader.by_ref().take(spec.line_ending.len() as u64).read_to_end(buffer)? != 0
            && &buffer[..] != &spec.line_ending[..] {
            return Err(Error::DataDoesNotMatchLineEnding(
                spec.line_ending.clone(),
                buffer[..].to_owned()
            ))?;
        }

        Ok(Data { ranges: ranges, data: self.field_reader.read_support().upcast_data(field_buffer).map_err(Error::DataHolderError)? })
    }
}

pub struct Reader<
    'a,
    R: BufRead + 'a,
    T: FieldParser<V> + 'a,
    U: SpecSource<V> + 'a,
    V: RecordReadSupport,
    W: Borrow<HashMap<String, RecordSpec>> + 'a,
    X: BorrowMut<R> + 'a,
    Y: BorrowMut<Vec<u8>> + 'a,
    Z: FieldBufferSource + 'a
> {
    source: X,
    reader: RecordReader<'a, T, V>,
    spec_source: U,
    record_specs: W,
    buffer: Y,
    field_buffer_source: Z,
    source_type: ::std::marker::PhantomData<&'a R>
}

impl<'a, R, T, U, V, W, X, Y, Z> Reader<'a, R, T, U, V, W, X, Y, Z>
    where R: BufRead + 'a,
          T: FieldParser<V> + 'a,
          U: SpecSource<V> + 'a,
          V: RecordReadSupport,
          W: Borrow<HashMap<String, RecordSpec>> + 'a,
          X: BorrowMut<R> + 'a,
          Y: BorrowMut<Vec<u8>> + 'a,
          Z: FieldBufferSource + 'a {
    pub fn read_record<'b, A: BuildableDataRanges + 'b>(&mut self) -> PositionalResult<Record<A, V::DataHolder>> {
        let spec_name = self.spec_source.next(self.source.borrow_mut(), self.record_specs.borrow(), self.reader.read_support())
            .map_err(Error::SpecStreamError)?
            .ok_or(Error::SpecStreamReturnedNone)?
        ;
        self.reader
            .read(
                self.source.borrow_mut(),
                self.record_specs.borrow().get(spec_name).ok_or_else(|| Error::RecordSpecNotFound(spec_name.to_string()))?,
                self.field_buffer_source.get().unwrap_or_else(|| Vec::new()),
                self.buffer.borrow_mut()
            )
            .map(|data| Record { data: data, name: spec_name.to_string() })
            .map_err(|e| (e, spec_name).into())

    }

    pub fn into_inner(self) -> RecordReader<'a, T, V> {
        self.reader
    }
}

pub struct ReaderBuilder<
    'a,
    R: BufRead + 'a,
    T: FieldParser<V> + 'a,
    U: SpecSource<V> + 'a,
    V: FieldReadSupport,
    W: Borrow<HashMap<String, RecordSpec>> + 'a,
    X: BorrowMut<R> + 'a,
    Y: BorrowMut<Vec<u8>> + 'a,
    Z: FieldBufferSource + 'a
> {
    read_support: V,
    source: Option<X>,
    field_parser: Option<T>,
    spec_source: Option<U>,
    record_specs: Option<W>,
    buffer: Y,
    field_buffer_source: Z,
    source_type: ::std::marker::PhantomData<&'a R>
}

impl<'a, R, T, U, V, W, X> ReaderBuilder<'a, R, T, U, V, W, X, Vec<u8>, Option<Vec<u8>>>
    where R: BufRead + 'a,
          T: FieldParser<V> + 'a,
          U: SpecSource<V> + 'a,
          V: RecordReadSupport,
          W: Borrow<HashMap<String, RecordSpec>> + 'a,
          X: BorrowMut<R> + 'a {
    pub fn new(read_support: V) -> Self {
        ReaderBuilder {
            read_support: read_support,
            source: None,
            field_parser: None,
            spec_source: None,
            record_specs: None,
            buffer: Vec::new(),
            field_buffer_source: None,
            source_type: ::std::marker::PhantomData
        }
    }
}

impl<'a, R, T, U, V, W, X> From<FieldReader<'a, T, V>> for ReaderBuilder<'a, R, T, U, V, W, X, Vec<u8>, Option<Vec<u8>>>
    where R: BufRead + 'a,
          T: FieldParser<V> + 'a,
          U: SpecSource<V> + 'a,
          V: RecordReadSupport,
          W: Borrow<HashMap<String, RecordSpec>> + 'a,
          X: BorrowMut<R> + 'a {
    fn from(field_reader: FieldReader<'a, T, V>) -> Self {
        Self::new(field_reader.read_support).with_field_parser(field_reader.parser)
    }
}

impl<'a, R, T, U, V, W, X> From<RecordReader<'a, T, V>> for ReaderBuilder<'a, R, T, U, V, W, X, Vec<u8>, Option<Vec<u8>>>
    where R: BufRead + 'a,
          T: FieldParser<V> + 'a,
          U: SpecSource<V> + 'a,
          V: RecordReadSupport,
          W: Borrow<HashMap<String, RecordSpec>> + 'a,
          X: BorrowMut<R> + 'a {
    fn from(record_reader: RecordReader<'a, T, V>) -> Self {
        Self::from(record_reader.field_reader)
    }
}

impl<'a, R, T, U, V, W, X, Y, Z> From<Reader<'a, R, T, U, V, W, X, Y, Z>> for ReaderBuilder<'a, R, T, U, V, W, X, Y, Z>
    where R: BufRead + 'a,
          T: FieldParser<V> + 'a,
          U: SpecSource<V> + 'a,
          V: RecordReadSupport,
          W: Borrow<HashMap<String, RecordSpec>> + 'a,
          X: BorrowMut<R> + 'a,
          Y: BorrowMut<Vec<u8>> + 'a,
          Z: FieldBufferSource + 'a {
    fn from(reader: Reader<'a, R, T, U, V, W, X, Y, Z>) -> Self {
        ReaderBuilder {
            read_support: reader.reader.field_reader.read_support,
            source: Some(reader.source),
            field_parser: Some(reader.reader.field_reader.parser),
            spec_source: Some(reader.spec_source),
            record_specs: Some(reader.record_specs),
            buffer: reader.buffer,
            field_buffer_source: reader.field_buffer_source,
            source_type: ::std::marker::PhantomData
        }
    }
}

impl<'a, R, T, U, V, W, X, Y, Z> ReaderBuilder<'a, R, T, U, V, W, X, Y, Z>
    where R: BufRead + 'a,
          T: FieldParser<V> + 'a,
          U: SpecSource<V> + 'a,
          V: RecordReadSupport,
          W: Borrow<HashMap<String, RecordSpec>> + 'a,
          X: BorrowMut<R> + 'a,
          Y: BorrowMut<Vec<u8>> + 'a,
          Z: FieldBufferSource + 'a {
    pub fn with_source<A: BufRead + 'a, B: BorrowMut<A> + 'a>(self, source: B) -> ReaderBuilder<'a, A, T, U, V, W, B, Y, Z> {
        ReaderBuilder {
            read_support: self.read_support,
            source: Some(source),
            field_parser: self.field_parser,
            spec_source: self.spec_source,
            record_specs: self.record_specs,
            buffer: self.buffer,
            field_buffer_source: self.field_buffer_source,
            source_type: ::std::marker::PhantomData
        }
    }

    pub fn with_field_parser<A: FieldParser<V> + 'a>(self, field_parser: A) -> ReaderBuilder<'a, R, A, U, V, W, X, Y, Z> {
        ReaderBuilder {
            read_support: self.read_support,
            source: self.source,
            field_parser: Some(field_parser),
            spec_source: self.spec_source,
            record_specs: self.record_specs,
            buffer: self.buffer,
            field_buffer_source: self.field_buffer_source,
            source_type: ::std::marker::PhantomData
        }
    }

    pub fn with_spec_source<A: SpecSource<V> + 'a>(self, spec_source: A) -> ReaderBuilder<'a, R, T, A, V, W, X, Y, Z> {
        ReaderBuilder {
            read_support: self.read_support,
            source: self.source,
            field_parser: self.field_parser,
            spec_source: Some(spec_source),
            record_specs: self.record_specs,
            buffer: self.buffer,
            field_buffer_source: self.field_buffer_source,
            source_type: ::std::marker::PhantomData
        }
    }

    pub fn with_record_specs<B: Borrow<HashMap<String, RecordSpec>> + 'a>(self, record_specs: B) -> ReaderBuilder<'a, R, T, U, V, B, X, Y, Z> {
        ReaderBuilder {
            read_support: self.read_support,
            source: self.source,
            field_parser: self.field_parser,
            spec_source: self.spec_source,
            record_specs: Some(record_specs),
            buffer: self.buffer,
            field_buffer_source: self.field_buffer_source,
            source_type: ::std::marker::PhantomData
        }
    }

    pub fn with_buffer<A: BorrowMut<Vec<u8>> + 'a>(self, buffer: A) -> ReaderBuilder<'a, R, T, U, V, W, X, A, Z> {
        ReaderBuilder {
            read_support: self.read_support,
            source: self.source,
            field_parser: self.field_parser,
            spec_source: self.spec_source,
            record_specs: self.record_specs,
            buffer: buffer,
            field_buffer_source: self.field_buffer_source,
            source_type: ::std::marker::PhantomData
        }
    }

    pub fn with_field_buffer_source<A: FieldBufferSource + 'a>(self, field_buffer_source: A) -> ReaderBuilder<'a, R, T, U, V, W, X, Y, A> {
        ReaderBuilder {
            read_support: self.read_support,
            source: self.source,
            field_parser: self.field_parser,
            spec_source: self.spec_source,
            record_specs: self.record_specs,
            buffer: self.buffer,
            field_buffer_source: field_buffer_source,
            source_type: ::std::marker::PhantomData
        }
    }

    pub fn build(self) -> Result<Reader<'a, R, T, U, V, W, X, Y, Z>> {
        Ok(Reader {
            source: self.source.ok_or(Error::BuildError("source needs to be defined in order to build"))?,
            reader: RecordReader::new(FieldReader::new(
                self.field_parser.ok_or(Error::BuildError("field_parser needs to be defined in order to build"))?,
                self.read_support
            )),
            spec_source: self.spec_source.ok_or(Error::BuildError("spec_source needs to be defined in order to build"))?,
            record_specs: self.record_specs.ok_or(Error::BuildError("record_specs needs to be defined in order to build"))?,
            buffer: self.buffer,
            field_buffer_source: self.field_buffer_source,
            source_type: ::std::marker::PhantomData
        })
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use error::{Error, FieldError};
    use test::*;
    use std::io::Cursor;
    use std::collections::{HashMap, BTreeMap};
    use std::ops::Range;
    use data_type::BinarySupport;

    #[test]
    fn read_record() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;zxcvbnm,./-=[];\ndfszbvvitwyotywt4trjkvvbjsbrgh4oq3njm,k.l/[p]";
        let mut buf = Cursor::new(string.as_bytes());
        let mut parser = MockParser::new();
        let record_spec = &spec.record_specs.get("record1").unwrap();
        parser.add_parse_call(string[..4].as_bytes().to_owned(), record_spec.field_specs.get("field1").unwrap().clone(), Ok("hello".as_bytes().to_owned()));
        parser.add_parse_call(string[4..9].as_bytes().to_owned(), record_spec.field_specs.get("field2").unwrap().clone(), Ok("hello2".as_bytes().to_owned()));
        parser.add_parse_call(string[9..45].as_bytes().to_owned(), record_spec.field_specs.get("field3").unwrap().clone(), Ok("hello3".as_bytes().to_owned()));
        parser.add_parse_call(string[46..50].as_bytes().to_owned(), record_spec.field_specs.get("field1").unwrap().clone(), Ok("hello4".as_bytes().to_owned()));
        parser.add_parse_call(string[50..55].as_bytes().to_owned(), record_spec.field_specs.get("field2").unwrap().clone(), Ok("hello5".as_bytes().to_owned()));
        parser.add_parse_call(string[55..91].as_bytes().to_owned(), record_spec.field_specs.get("field3").unwrap().clone(), Ok("hello6".as_bytes().to_owned()));
        let reader = RecordReader::new(FieldReader::new(&parser, BinarySupport));
        assert_result!(
        Ok(Data {
                data: "hellohello2hello3".as_bytes().to_owned(),
                ranges: [("field1".to_owned(), 0..5),
                    ("field2".to_owned(), 5..11),
                    ("field3".to_owned(), 11..17)]
                    .iter().cloned().collect::<HashMap<String, Range<usize>>>()
            }),
            reader.read(&mut buf, record_spec, Vec::new(), &mut Vec::new())
        );
        assert_result!(Ok(Data {
                data: "hello4hello5hello6".as_bytes().to_owned(),
                ranges: [("field1".to_owned(), 0..6),
                    ("field2".to_owned(), 6..12),
                    ("field3".to_owned(), 12..18)]
                    .iter().cloned().collect::<BTreeMap<String, Range<usize>>>()
            }),
            reader.read(&mut buf, record_spec, Vec::new(), &mut Vec::new())
        );
    }

    #[test]
    fn read_record_with_bad_line_ending() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;zxcvbnm,./-=[];bla";
        let mut buf = Cursor::new(string.as_bytes());
        let mut parser = MockParser::new();
        let record_spec = &spec.record_specs.get("record1").unwrap();
        parser.add_parse_call(string[..4].as_bytes().to_owned(), record_spec.field_specs.get("field1").unwrap().clone(), Ok("hello".as_bytes().to_owned()));
        parser.add_parse_call(string[4..9].as_bytes().to_owned(), record_spec.field_specs.get("field2").unwrap().clone(), Ok("hello2".as_bytes().to_owned()));
        parser.add_parse_call(string[9..45].as_bytes().to_owned(), record_spec.field_specs.get("field3").unwrap().clone(), Ok("hello3".as_bytes().to_owned()));
        let reader = RecordReader::new(FieldReader::new(&parser, BinarySupport));
        assert_result!(
            Err(FieldError {
                error: Error::DataDoesNotMatchLineEnding(_, _),
                field: None
            }),
            reader.read::<_, HashMap<_, _>>(&mut buf, record_spec, Vec::new(), &mut Vec::new())
        );
    }

    #[test]
    fn read_record_with_parsing_error() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;zxcvbnm,./-=[];\ndfszbvvitwyotywt4trjkvvbjsbrgh4oq3njm,k.l/[p]";
        let mut buf = Cursor::new(string.as_bytes());
        let mut parser = MockParser::new();
        let record_spec = &spec.record_specs.get("record1").unwrap();
        parser.add_parse_call(string[..4].as_bytes().to_owned(), record_spec.field_specs.get("field1").unwrap().clone(), Err("".into()));
        let reader = RecordReader::new(FieldReader::new(&parser, BinarySupport));
        assert_result!(
            Err(FieldError {
                error: Error::ParserFailure(_),
                field: Some(ref field)
            }) if field == "field1",
            reader.read::<_, BTreeMap<_, _>>(&mut buf, record_spec, Vec::new(), &mut Vec::new())
        );
    }

    #[test]
    fn read_record_with_read_error() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;";
        let mut buf = Cursor::new(string.as_bytes());
        let mut parser = MockParser::new();
        let record_spec = &spec.record_specs.get("record1").unwrap();
        parser.add_parse_call(string[..4].as_bytes().to_owned(), record_spec.field_specs.get("field1").unwrap().clone(), Ok("hello".as_bytes().to_owned()));
        parser.add_parse_call(string[4..9].as_bytes().to_owned(), record_spec.field_specs.get("field2").unwrap().clone(), Ok("hello2".as_bytes().to_owned()));
        let reader = RecordReader::new(FieldReader::new(&parser, BinarySupport));
        assert_result!(
            Err(FieldError {
                error: Error::CouldNotReadEnough(_),
                field: Some(ref field)
            }) if field == "field3",
            reader.read::<_, BTreeMap<_, _>>(&mut buf, record_spec, Vec::new(), &mut Vec::new())
        );
    }

    #[test]
    fn read_field() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;zxcvbnm,./-=[];dfszbvvitwyotywt4trjkvvbjsbrgh4oq3njm,k.l/[p]";
        let mut buf = Cursor::new(string.as_bytes());
        let mut buffer = Vec::new();
        let mut parser = MockParser::new();
        let record_spec = &spec.record_specs.get("record1").unwrap();
        parser.add_parse_call(string[..4].as_bytes().to_owned(), record_spec.field_specs.get("field1").unwrap().clone(), Ok("hello".as_bytes().to_owned()));
        parser.add_parse_call(string[4..9].as_bytes().to_owned(), record_spec.field_specs.get("field2").unwrap().clone(), Ok("hello2".as_bytes().to_owned()));
        let reader = FieldReader::new(&parser, BinarySupport);
        assert_result!(Ok(()), reader.read(&mut buf,&record_spec.field_specs.get("field1").unwrap(), &mut buffer, &mut Vec::new()));
        assert_eq!("hello".as_bytes().to_owned(), buffer);
        assert_result!(Ok(()), reader.read(&mut buf,&record_spec.field_specs.get("field2").unwrap(), &mut buffer, &mut Vec::new()));
        assert_eq!("hellohello2".as_bytes().to_owned(), buffer);
    }

    #[test]
    fn read_field_with_parsing_error() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;zxcvbnm,./-=[];\ndfszbvvitwyotywt4trjkvvbjsbrgh4oq3njm,k.l/[p]";
        let mut buf = Cursor::new(string.as_bytes());
        let mut buffer = Vec::new();
        let mut parser = MockParser::new();
        let record_spec = &spec.record_specs.get("record1").unwrap();
        parser.add_parse_call(string[..5].as_bytes().to_owned(), record_spec.field_specs.get("field2").unwrap().clone(), Err("".into()));
        let reader = FieldReader::new(&parser, BinarySupport);
        assert_result!(
            Err(Error::ParserFailure(_)),
            reader.read(&mut buf, &record_spec.field_specs.get("field2").unwrap(), &mut buffer, &mut Vec::new())
        );
    }

    #[test]
    fn read_field_with_read_error() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;";
        let mut buf = Cursor::new(string.as_bytes());
        let mut buffer = Vec::new();
        let mut parser = MockParser::new();
        let record_spec = &spec.record_specs.get("record1").unwrap();
        parser.add_parse_call(string[..4].as_bytes().to_owned(), record_spec.field_specs.get("field1").unwrap().clone(), Ok("hello".as_bytes().to_owned()));
        parser.add_parse_call(string[4..9].as_bytes().to_owned(), record_spec.field_specs.get("field2").unwrap().clone(), Ok("hello2".as_bytes().to_owned()));
        let reader = FieldReader::new(&parser, BinarySupport);
        assert_result!(Ok(()), reader.read(&mut buf, &record_spec.field_specs.get("field1").unwrap(), &mut buffer, &mut Vec::new()));
        assert_result!(Ok(()), reader.read(&mut buf, &record_spec.field_specs.get("field2").unwrap(), &mut buffer, &mut Vec::new()));
        assert_result!(
            Err(Error::CouldNotReadEnough(_)),
            reader.read(&mut buf, &record_spec.field_specs.get("field3").unwrap(), &mut buffer, &mut Vec::new())
        );
    }
}