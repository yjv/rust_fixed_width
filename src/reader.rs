use spec::{RecordSpec, FieldSpec};
use std::collections::{HashMap};
use std::io::{Read, BufRead, BufReader};
use std::borrow::{Borrow, BorrowMut};
use recognizer::LineRecordSpecRecognizer;
use super::{Error, Result, PositionalResult, Record, FieldResult};
use record::{Data, BuildableDataRanges, ReadType, ShouldReadMore};
use parser::FieldParser;
use std::collections::VecDeque;

pub struct FieldReader<T: FieldParser<U>, U: ReadType> {
    parser: T,
    read_type: U
}

impl<T: FieldParser<U>, U: ReadType> FieldReader<T, U> {
    pub fn new(parser: T, read_type: U) -> FieldReader<T, U> {
        FieldReader {
            parser: parser,
            read_type: read_type
        }
    }

    pub fn read_type(&self) -> &U {
        &self.read_type
    }
}

impl <T: FieldParser<U>, U: ReadType> FieldReader<T, U> {
    pub fn read<'a, V>(&self, reader: &'a mut V, field_spec: &'a FieldSpec, field_buffer: &'a mut Vec<u8>, buffer: &'a mut Vec<u8>) -> Result<()>
        where V: Read + 'a
    {
        buffer.clear();
        while let ShouldReadMore::More(amount) = self.read_type.should_read_more(field_spec.length, &buffer[..]) {
            let amount_read = reader.by_ref().take(amount as u64).read_to_end(buffer)?;

            if amount_read != amount {
                return Err(Error::CouldNotReadEnough(buffer.clone()))
            }
        }

        self.parser.parse(&buffer[..], field_spec, field_buffer, &self.read_type)?;

        Ok(())
    }
}

pub struct RecordReader<T: FieldParser<U>, U: ReadType> {
    field_reader: FieldReader<T, U>
}

impl<T: FieldParser<U>, U: ReadType> RecordReader<T, U> {
    pub fn new(field_writer: FieldReader<T, U>) -> RecordReader<T, U> {
        RecordReader {
            field_reader: field_writer
        }
    }

    pub fn read_type(&self) -> &U {
        self.field_reader.read_type()
    }
}

impl <T: FieldParser<U>, U: ReadType> RecordReader<T, U> {
    pub fn read<'a, V, X>(&self, reader: &'a mut V, spec: &'a RecordSpec, mut field_buffer: Vec<u8>, buffer: &'a mut Vec<u8>) -> FieldResult<Data<X, U::DataHolder>>
        where V: Read + 'a,
              X: BuildableDataRanges + 'a
    {
        let mut ranges = X::new();
        for (name, field_spec) in &spec.field_specs {
            let old_length = field_buffer.len();
            self.field_reader.read(reader, field_spec, &mut field_buffer, buffer).map_err(|e| (e, name))?;

            ranges.insert(name, self.field_reader.read_type().get_range(
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

        Ok(Data { ranges: ranges, data: self.field_reader.read_type().upcast_data(field_buffer)? })
    }
}

pub struct RecordRecognizer<T: LineRecordSpecRecognizer<U>, U: ReadType> {
    recognizer: T,
    read_type: U
}

impl<T: LineRecordSpecRecognizer<U>, U: ReadType> RecordRecognizer<T, U> {
    pub fn recognize<'a, 'b, V: BufRead + 'a>(&self, reader: &'a mut V, record_specs: &'b HashMap<String, RecordSpec>) -> Result<&'b str> {
        Ok(self.recognizer.recognize_for_line(reader, record_specs, &self.read_type)?)
    }

    pub fn buffer<'a, V: Read + 'a>(&self, reader: V, record_specs: &'a HashMap<String, RecordSpec>) -> BufReader<V> {
        match self.get_suggested_buffer_size(record_specs) {
            Some(size) => BufReader::with_capacity(size, reader),
            None => BufReader::new(reader)
        }
    }

    pub fn get_suggested_buffer_size<'a>(&self, record_specs: &'a HashMap<String, RecordSpec>) -> Option<usize> {
        self.recognizer.get_suggested_buffer_size(record_specs, &self.read_type)
    }
}

pub trait FieldBufferSource {
    fn get(&mut self) -> Option<Vec<u8>>;
}

impl<'a, T: FieldBufferSource + 'a> FieldBufferSource for &'a mut T {
    fn get(&mut self) -> Option<Vec<u8>> {
        FieldBufferSource::get(*self)
    }
}

impl FieldBufferSource for Vec<u8> {
    fn get(&mut self) -> Option<Vec<u8>> {
        Some(self.clone())
    }
}

impl FieldBufferSource for Option<Vec<u8>> {
    fn get(&mut self) -> Option<Vec<u8>> {
        self.take()
    }
}

impl FieldBufferSource for Vec<Vec<u8>> {
    fn get(&mut self) -> Option<Vec<u8>> {
        self.pop()
    }
}

impl FieldBufferSource for VecDeque<Vec<u8>> {
    fn get(&mut self) -> Option<Vec<u8>> {
        self.pop_front()
    }
}

pub trait SpecSource<T: ReadType> {
    fn next<'a, 'b, U: BufRead + 'a>(&mut self, reader: &'a mut U, record_specs: &'b HashMap<String, RecordSpec>, read_type: &'a T) -> Result<&'b str>;
    fn get_suggested_buffer_size<'a>(&self, _: &'a HashMap<String, RecordSpec>) -> Option<usize> {
        None
    }
}

impl<'c, T: SpecSource<U> + 'c, U: ReadType + 'c> SpecSource<U> for &'c mut T {
    fn next<'a, 'b, V: BufRead + 'a>(&mut self, reader: &'a mut V, record_specs: &'b HashMap<String, RecordSpec>, read_type: &'a U) -> Result<&'b str> {
        SpecSource::next(*self, reader, record_specs, read_type)
    }
}

pub struct RecognizerSource<T: LineRecordSpecRecognizer<U>, U: ReadType> {
    recognizer: T,
    read_type: U
}

impl <T, U> RecognizerSource<T, U>
    where T: LineRecordSpecRecognizer<U>,
          U: ReadType {
    pub fn new(recognizer: T, read_type: U) -> Self {
        RecognizerSource {
            recognizer: recognizer,
            read_type: read_type
        }
    }
}

impl  <T, U> SpecSource<U> for RecognizerSource<T, U>
    where T: LineRecordSpecRecognizer<U>,
          U: ReadType {
    fn next<'a, 'b, X: BufRead + 'a>(&mut self, reader: &'a mut X, record_specs: &'b HashMap<String, RecordSpec>, read_type: &'a U) -> Result<&'b str> {
        Ok(self.recognizer.recognize_for_line(reader, record_specs, read_type)?)
    }

    fn get_suggested_buffer_size<'a>(&self, record_specs: &'a HashMap<String, RecordSpec>) -> Option<usize> {
        self.recognizer.get_suggested_buffer_size(record_specs, &self.read_type)
    }
}

pub struct Reader<
    'a,
    R: BufRead + 'a,
    T: FieldParser<V> + 'a,
    U: SpecSource<V> + 'a,
    V: ReadType + 'a,
    W: Borrow<HashMap<String, RecordSpec>> + 'a,
    X: BorrowMut<R> + 'a,
    Y: BorrowMut<Vec<u8>> + 'a,
    Z: FieldBufferSource + 'a
> {
    source: X,
    reader: RecordReader<T, V>,
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
          V: ReadType + 'a,
          W: Borrow<HashMap<String, RecordSpec>> + 'a,
          X: BorrowMut<R> + 'a,
          Y: BorrowMut<Vec<u8>> + 'a,
          Z: FieldBufferSource + 'a {
    pub fn read_record<'b, A: BuildableDataRanges + 'b>(&mut self) -> PositionalResult<Record<A, V::DataHolder>> {
        let spec_name = self.spec_source.next(self.source.borrow_mut(), self.record_specs.borrow(), self.reader.read_type())?;
        self.reader.read(
            self.source.borrow_mut(),
            self.record_specs.borrow().get(spec_name).ok_or_else(|| Error::RecordSpecNotFound(spec_name.to_string()))?,
            self.field_buffer_source.get().unwrap_or_else(|| Vec::new()),
            self.buffer.borrow_mut()
        )
            .map(|data| Record { data: data, name: spec_name.to_string() })
            .map_err(|e| (e, spec_name.to_string()).into())

    }

    pub fn into_inner(self) -> RecordReader<T, V> {
        self.reader
    }
}

pub struct ReaderBuilder<
    'a,
    R: BufRead + 'a,
    T: FieldParser<V> + 'a,
    U: SpecSource<V> + 'a,
    V: ReadType + 'a,
    W: Borrow<HashMap<String, RecordSpec>> + 'a,
    X: BorrowMut<R> + 'a,
    Y: BorrowMut<Vec<u8>> + 'a,
    Z: FieldBufferSource + 'a
> {
    source: Option<X>,
    reader: RecordReader<T, V>,
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
          V: ReadType + 'a,
          W: Borrow<HashMap<String, RecordSpec>> + 'a,
          X: BorrowMut<R> + 'a {
    pub fn new(record_reader: RecordReader<T, V>) -> Self {
        ReaderBuilder {
            source: None,
            reader: record_reader,
            spec_source: None,
            record_specs: None,
            buffer: Vec::new(),
            field_buffer_source: None,
            source_type: ::std::marker::PhantomData
        }
    }
}

impl<'a, R, T, U, V, W, X, Y, Z> ReaderBuilder<'a, R, T, U, V, W, X, Y, Z>
    where R: BufRead + 'a,
          T: FieldParser<V> + 'a,
          U: SpecSource<V> + 'a,
          V: ReadType + 'a,
          W: Borrow<HashMap<String, RecordSpec>> + 'a,
          X: BorrowMut<R> + 'a,
          Y: BorrowMut<Vec<u8>> + 'a,
          Z: FieldBufferSource + 'a {
    pub fn with_source<A: BufRead + 'a, B: BorrowMut<A> + 'a>(self, source: B) -> ReaderBuilder<'a, A, T, U, V, W, B, Y, Z> {
        ReaderBuilder {
            source: Some(source),
            reader: self.reader,
            spec_source: self.spec_source,
            record_specs: self.record_specs,
            buffer: self.buffer,
            field_buffer_source: self.field_buffer_source,
            source_type: ::std::marker::PhantomData
        }
    }

    pub fn with_spec_source<A: SpecSource<V> + 'a>(self, spec_source: A) -> ReaderBuilder<'a, R, T, A, V, W, X, Y, Z> {
        ReaderBuilder {
            source: self.source,
            reader: self.reader,
            spec_source: Some(spec_source),
            record_specs: self.record_specs,
            buffer: self.buffer,
            field_buffer_source: self.field_buffer_source,
            source_type: ::std::marker::PhantomData
        }
    }

    pub fn with_record_specs<A: Borrow<HashMap<String, RecordSpec>> + 'a>(self, record_specs: A) -> ReaderBuilder<'a, R, T, U, V, A, X, Y, Z> {
        ReaderBuilder {
            source: self.source,
            reader: self.reader,
            spec_source: self.spec_source,
            record_specs: Some(record_specs),
            buffer: self.buffer,
            field_buffer_source: self.field_buffer_source,
            source_type: ::std::marker::PhantomData
        }
    }

    pub fn with_buffer<A: BorrowMut<Vec<u8>> + 'a>(self, buffer: A) -> ReaderBuilder<'a, R, T, U, V, W, X, A, Z> {
        ReaderBuilder {
            source: self.source,
            reader: self.reader,
            spec_source: self.spec_source,
            record_specs: self.record_specs,
            buffer: buffer,
            field_buffer_source: self.field_buffer_source,
            source_type: ::std::marker::PhantomData
        }
    }

    pub fn with_field_buffer_source<A: FieldBufferSource + 'a>(self, field_buffer_source: A) -> ReaderBuilder<'a, R, T, U, V, W, X, Y, A> {
        ReaderBuilder {
            source: self.source,
            reader: self.reader,
            spec_source: self.spec_source,
            record_specs: self.record_specs,
            buffer: self.buffer,
            field_buffer_source: field_buffer_source,
            source_type: ::std::marker::PhantomData
        }
    }

    pub fn build(self) -> Reader<'a, R, T, U, V, W, X, Y, Z> {
        Reader {
            source: self.source.expect("source needs to be defined in order to build"),
            reader: self.reader,
            spec_source: self.spec_source.expect("spec_source needs to be defined in order to build"),
            record_specs: self.record_specs.expect("record_specs needs to be defined in order to build"),
            buffer: self.buffer,
            field_buffer_source: self.field_buffer_source,
            source_type: ::std::marker::PhantomData
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use super::super::{Error, Data, FieldError};
    use test::*;
    use std::io::Cursor;
    use std::collections::{HashMap, BTreeMap};
    use std::ops::Range;
    use record::BinaryType;

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
        let reader = RecordReader::new(FieldReader::new(&parser, BinaryType));
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
        let reader = RecordReader::new(FieldReader::new(&parser, BinaryType));
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
        parser.add_parse_call(string[..4].as_bytes().to_owned(), record_spec.field_specs.get("field1").unwrap().clone(), Err(Error::CouldNotReadEnough(Vec::new())));
        let reader = RecordReader::new(FieldReader::new(&parser, BinaryType));
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
        let reader = RecordReader::new(FieldReader::new(&parser, BinaryType));
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
        let reader = FieldReader::new(&parser, BinaryType);
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
        parser.add_parse_call(string[..5].as_bytes().to_owned(), record_spec.field_specs.get("field2").unwrap().clone(), Err(Error::CouldNotReadEnough(Vec::new())));
        let reader = FieldReader::new(&parser, BinaryType);
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
        let reader = FieldReader::new(&parser, BinaryType);
        assert_result!(Ok(()), reader.read(&mut buf, &record_spec.field_specs.get("field1").unwrap(), &mut buffer, &mut Vec::new()));
        assert_result!(Ok(()), reader.read(&mut buf, &record_spec.field_specs.get("field2").unwrap(), &mut buffer, &mut Vec::new()));
        assert_result!(
            Err(Error::CouldNotReadEnough(_)),
            reader.read(&mut buf, &record_spec.field_specs.get("field3").unwrap(), &mut buffer, &mut Vec::new())
        );
    }
}