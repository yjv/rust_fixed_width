use spec::{RecordSpec, FieldSpec};
use padder::{UnPadder, IdentityPadder};
use std::collections::{HashMap};
use std::io::{Read, BufRead, BufReader};
use std::borrow::Borrow;
use std::ops::Range;
use recognizer::{LineBuffer, LineRecordSpecRecognizer, NoneRecognizer};
use super::{Error, Result, PositionalResult, Record, PositionalError, FieldResult};
use record::{Data, DataRanges, BuildableDataRanges, ReadType, BinaryType, ShouldReadMore};

pub struct Reader<T: UnPadder<W>, U: LineRecordSpecRecognizer<W>, V: Borrow<HashMap<String, RecordSpec>>, W: ReadType> {
    un_padder: T,
    recognizer: U,
    specs: V,
    buffer: Vec<u8>,
    read_type: W
}

impl<T: UnPadder<W>, U: LineRecordSpecRecognizer<W>, V: Borrow<HashMap<String, RecordSpec>>, W: ReadType> Reader<T, U, V, W> {
    pub fn read_field<'a, X, Y>(&mut self, reader: &'a mut X, record_name: &'a str, name: &'a str, field: Y) -> Result<Vec<u8>>
        where X: 'a + Read,
              Y: Into<Option<Vec<u8>>>
    {
        let mut field = field.into().unwrap_or_else(|| Vec::new());
        let record_spec = self.specs.borrow()
            .get(record_name)
            .ok_or_else(|| Error::RecordSpecNotFound(record_name.to_string()))?
        ;
        let field_spec = record_spec
            .field_specs.get(name)
            .ok_or_else(|| Error::FieldSpecNotFound(record_name.to_string(), name.to_string()))?
        ;
        self.buffer.clear();
        Self::_read_data(reader, field_spec.length, &mut self.buffer)?;
        self.un_padder.unpad(
            &self.buffer[..],
            &field_spec.padding,
            field_spec.padding_direction,
            &mut field,
            &self.read_type
        )?;
        Ok(field)
    }

    pub fn read_record<'a, X, Y, Z, A>(&mut self, reader: &'a mut X, record_name: Y, line: Z) -> PositionalResult<Record<A, W::DataHolder>>
        where X: 'a + Read,
              Y: Into<Option<&'a str>>,
              Z: Into<Option<Vec<u8>>>,
              A: BuildableDataRanges
    {
        let mut line = line.into().unwrap_or_else(|| Vec::new());
        let mut reader = RememberingReader::new(reader);
        let specs = self.specs.borrow();
        let record_name = record_name
            .into()
            .map_or_else(
                || self.recognizer.recognize_for_line(&mut BufReader::new(&mut reader), specs, &self.read_type),
                |name| Ok(name)
            )?
        ;

        let record_spec = specs
            .get(record_name)
            .ok_or_else(|| Error::RecordSpecNotFound(record_name.to_owned()))?
        ;
        reader.restart();
        let mut ranges = A::new();

        self.buffer.clear();

        for (name, field_spec) in &record_spec.field_specs {
            Self::_read_data(&mut reader, field_spec.length, &mut self.buffer).map_err(|e| (e, record_name.to_owned(), name.clone()))?;
            if !field_spec.write_only {
                let old_length = line.len();
                self.un_padder.unpad(
                    &self.buffer[..],
                    &field_spec.padding,
                    field_spec.padding_direction,
                    &mut line,
                    &self.read_type
                ).map_err(|e| (e, record_name.to_owned(), name.clone()))?;
                ranges.insert(name, old_length..line.len());
            }

            self.buffer.clear();
        }

        Self::_absorb_line_ending(&mut reader, &record_spec.line_ending, &mut self.buffer).map_err(|e| (e, record_name))?;

        Ok(Record { data: Data { data: self.read_type.upcast_data(line)?, ranges: ranges}, name: record_name.to_owned() })
    }

    pub fn absorb_line_ending<'a, Y: 'a + Read>(&mut self, reader: &'a mut Y, line_ending: &[u8]) -> Result<()> {
        Self::_absorb_line_ending(reader, line_ending, &mut self.buffer)
    }

    fn _absorb_line_ending<'a, Y: 'a + Read>(reader: &'a mut Y, line_ending: &[u8], buffer: &mut Vec<u8>) -> Result<()> {
        buffer.clear();
        reader.by_ref().take(line_ending.len() as u64).read_to_end(buffer)?;
        if buffer.len() != 0 && &buffer[..] != &line_ending[..] {
            return Err(Error::DataDoesNotMatchLineEnding(
                line_ending.to_owned(),
                buffer.clone()
            ));
        }

        Ok(())
    }

    fn _read_data<'a, Y: 'a + Read>(reader: &'a mut Y, length: usize, data: &mut Vec<u8>) -> Result<()> {
        let _ = reader.by_ref().take((length - data.len()) as u64).read_to_end(data)?;

        if data.len() < length {
            Err(Error::CouldNotReadEnough(data.clone()))
        } else {
            Ok(())
        }
    }

    pub fn iter<'a, X: 'a + Read, Y: BuildableDataRanges + 'a>(&'a mut self, reader: &'a mut X) -> Iter<'a, X, T, U, V, W, Y> {
        Iter {
            source: reader,
            reader: self,
            marker: ::std::marker::PhantomData
        }
    }

    pub fn into_iter<X: Read, Y: BuildableDataRanges>(self, reader: X) -> IntoIter<X, T, U, V, W, Y> {
        IntoIter {
            source: reader,
            reader: self,
            marker: ::std::marker::PhantomData
        }
    }
}

pub struct ReaderBuilder<T: UnPadder<W>, U: LineRecordSpecRecognizer<W>, V: Borrow<HashMap<String, RecordSpec>>, W: ReadType> {
    un_padder: T,
    recognizer: U,
    specs: Option<V>,
    read_type: W
}

impl<V: Borrow<HashMap<String, RecordSpec>>> ReaderBuilder<IdentityPadder, NoneRecognizer, V, BinaryType> {
    pub fn new() -> ReaderBuilder<IdentityPadder, NoneRecognizer, V, BinaryType> {
        ReaderBuilder {
            un_padder: IdentityPadder,
            recognizer: NoneRecognizer,
            specs: None,
            read_type: BinaryType
        }
    }
}

impl<T: UnPadder<W>, U: LineRecordSpecRecognizer<W>, V: Borrow<HashMap<String, RecordSpec>>, W: ReadType> ReaderBuilder<T, U, V, W> {
    pub fn from_reader(reader: Reader<T, U, V, W>) -> Self {
        ReaderBuilder {
            un_padder: reader.un_padder,
            recognizer: reader.recognizer,
            read_type: reader.read_type,
            specs: Some(reader.specs)
        }
    }

    pub fn with_un_padder<X: UnPadder<W>>(self, un_padder: X) -> ReaderBuilder<X, U, V, W> {
        ReaderBuilder {
            un_padder: un_padder,
            recognizer: self.recognizer,
            specs: self.specs,
            read_type: self.read_type
        }
    }

    pub fn with_recognizer<X: LineRecordSpecRecognizer<W>>(self, recognizer: X) -> ReaderBuilder<T, X, V, W> {
        ReaderBuilder {
            un_padder: self.un_padder,
            recognizer: recognizer,
            specs: self.specs,
            read_type: self.read_type
        }
    }

    pub fn with_specs(mut self, specs: V) -> Self {
        self.specs = Some(specs);
        self
    }

    pub fn with_read_type<X: ReadType>(self, read_type: X) -> ReaderBuilder<T, U, V, X>
        where T: UnPadder<X>,
              U: LineRecordSpecRecognizer<X>
    {
        ReaderBuilder {
            un_padder: self.un_padder,
            recognizer: self.recognizer,
            specs: self.specs,
            read_type: read_type
        }
    }

    pub fn build(self) -> Reader<T, U, V, W> {
        Reader {
            un_padder: self.un_padder,
            recognizer: self.recognizer,
            specs: self.specs.expect("specs is required to build a writer"),
            buffer: Vec::new(),
            read_type: self.read_type
        }
    }
}

pub struct Iter<'a, T: Read + 'a, U: UnPadder<X> + 'a, V: LineRecordSpecRecognizer<X> + 'a, W: Borrow<HashMap<String, RecordSpec>> + 'a, X: ReadType + 'a, Y: BuildableDataRanges + 'a> {
    source: &'a mut T,
    reader: &'a mut Reader<U, V, W, X>,
    marker: ::std::marker::PhantomData<Y>
}

impl<'a, T: Read + 'a, U: UnPadder<X> + 'a, V: LineRecordSpecRecognizer<X> + 'a, W: Borrow<HashMap<String, RecordSpec>> + 'a, X: ReadType + 'a, Y: BuildableDataRanges + 'a> Iterator for Iter<'a, T, U, V, W, X, Y> {
    type Item = PositionalResult<Record<Y, X::DataHolder>>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.reader.read_record(self.source, None, None) {
            Err(PositionalError { error: Error::CouldNotReadEnough(ref string), .. }) if string.len() == 0 => None,
            r => Some(r)
        }
    }
}

pub struct IntoIter<T: Read, U: UnPadder<X>, V: LineRecordSpecRecognizer<X>, W: Borrow<HashMap<String, RecordSpec>>, X: ReadType, Y: BuildableDataRanges> {
    source: T,
    reader: Reader<U, V, W, X>,
    marker: ::std::marker::PhantomData<Y>
}

impl<T: Read, U: UnPadder<X>, V: LineRecordSpecRecognizer<X>, W: Borrow<HashMap<String, RecordSpec>>, X: ReadType, Y: BuildableDataRanges> Iterator for IntoIter<T, U, V, W, X, Y> {
    type Item = PositionalResult<Record<Y, X::DataHolder>>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.reader.read_record(&mut self.source, None, None) {
            Err(PositionalError { error: Error::CouldNotReadEnough(ref string), .. }) if string.len() == 0 => None,
            r => Some(r)
        }
    }
}

pub struct RememberingReader<T: Read> {
    inner: T,
    pos: usize,
    buf: Vec<u8>
}

impl<T: Read> RememberingReader<T> {
    pub fn new(reader: T) -> Self {
        RememberingReader {
            inner: reader,
            pos: 0,
            buf: Vec::new()
        }
    }

    pub fn restart(&mut self) {
        self.pos = 0;
    }

    pub fn forget(&mut self) {
        self.buf = self.buf.split_off(self.pos);
        self.pos = 0;
    }

    pub fn into_inner(self) -> T { self.inner }

    pub fn get_ref(&self) -> &T { &self.inner }

    pub fn get_mut(&mut self) -> &mut T { &mut self.inner }

    pub fn read_range(&mut self, range: Range<usize>) -> ::std::result::Result<Option<&[u8]>, ::std::io::Error> {
        if self.buf.len() < range.end {
            let amount = self.inner.by_ref().take((range.end - self.buf.len()) as u64).read(&mut self.buf)?;
            self.pos += amount;
        }

        if self.buf.len() >= range.end {
            Ok(Some(&self.buf[range]))
        } else {
            Ok(None)
        }
    }
}

impl<T: Read> Read for RememberingReader<T> {
    fn read(&mut self, buf: &mut [u8]) -> ::std::io::Result<usize> {
        let mut already_read = 0;

        if self.pos <= self.buf.len() {
            already_read = (&self.buf[self.pos..]).read(buf)?;
            self.pos += already_read;

            if already_read == buf.len() {
                return Ok(already_read);
            }
        }

        let amount = self.inner.read(&mut buf[already_read..])?;
        self.buf.extend_from_slice(&buf[already_read..already_read + amount]);
        self.pos += amount;
        Ok(already_read + amount)
    }
}

pub trait FieldParser<T: ReadType> {
    fn parse<'a>(&self, data: &[u8], field_spec: &'a FieldSpec, destination: &'a mut Vec<u8>, read_type: &'a T) -> Result<()>;
}

impl<'a, T, U: ReadType> FieldParser<U> for &'a T where T: 'a + FieldParser<U> {
    fn parse<'b>(&self, data: &'b [u8], field_spec: &'b FieldSpec, destination: &'b mut Vec<u8>, read_type: &'b U) -> Result<()> {
        (**self).parse(data, field_spec, destination, read_type)
    }
}

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
    pub fn recognize<'a, V: BufRead + 'a>(&self, reader: &'a mut V, record_specs: &'a HashMap<String, RecordSpec>) -> Result<&'a str> {
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

#[cfg(test)]
mod test {

    use super::*;
    use super::super::{Error, PositionalError, Position, Record, Data, FieldError};
    use test::*;
    use std::io::Cursor;
    use spec::PaddingDirection;
    use padder::Error as PaddingError;
    use std::collections::{HashMap, BTreeMap};
    use std::io::{Seek, SeekFrom, Read};
    use std::ops::Range;
    use record::{BinaryType, ReadType};

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
        let mut reader = RecordReader::new(FieldReader::new(&parser, BinaryType));
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
        let mut reader = RecordReader::new(FieldReader::new(&parser, BinaryType));
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
        let mut reader = RecordReader::new(FieldReader::new(&parser, BinaryType));
        assert_result!(
            Err(FieldError {
                error: Error::RecordSpecNameRequired,
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
        let mut reader = RecordReader::new(FieldReader::new(&parser, BinaryType));
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
        let mut reader = FieldReader::new(&parser, BinaryType);
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
        let mut reader = FieldReader::new(&parser, BinaryType);
        assert_result!(
            Err(Error::RecordSpecNameRequired),
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
        let mut reader = FieldReader::new(&parser, BinaryType);
        assert_result!(Ok(()), reader.read(&mut buf, &record_spec.field_specs.get("field1").unwrap(), &mut buffer, &mut Vec::new()));
        assert_result!(Ok(()), reader.read(&mut buf, &record_spec.field_specs.get("field2").unwrap(), &mut buffer, &mut Vec::new()));
        assert_result!(
            Err(Error::CouldNotReadEnough(_)),
            reader.read(&mut buf, &record_spec.field_specs.get("field3").unwrap(), &mut buffer, &mut Vec::new())
        );
    }

    #[test]
    fn iterator() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;zxcvbnm,./-=[];\ndfszbvvitwyotywt4trjkvvbjsbrgh4oq3njm,k.l/[p]";
        let mut buf = Cursor::new(string.as_bytes());
        let mut un_padder = MockPadder::new();
        un_padder.add_unpad_call(string[4..9].as_bytes().to_owned(), " ".as_bytes().to_owned(), PaddingDirection::Right, Ok("hello".as_bytes().to_owned()));
        un_padder.add_unpad_call(string[9..45].as_bytes().to_owned(), "xcvcxv".as_bytes().to_owned(), PaddingDirection::Right, Ok("hello2".as_bytes().to_owned()));
        un_padder.add_unpad_call(string[50..55].as_bytes().to_owned(), " ".as_bytes().to_owned(), PaddingDirection::Right, Ok("hello3".as_bytes().to_owned()));
        un_padder.add_unpad_call(string[55..91].as_bytes().to_owned(), "xcvcxv".as_bytes().to_owned(), PaddingDirection::Right, Ok("hello4".as_bytes().to_owned()));
        let mut recognizer = MockRecognizer::new();
        recognizer.add_line_recognize_call(&spec.record_specs, Ok("record1"));
        let mut reader = ReaderBuilder::new()
            .with_un_padder(&un_padder)
            .with_recognizer(&recognizer)
            .with_specs(&spec.record_specs)
            .build()
        ;
        let mut vec = Vec::new();
        vec.push(Record {
            data: Data { data: "hellohello2".as_bytes().to_owned(),
            ranges: [("field2".to_string(), 0..5),
                ("field3".to_string(), 5..11)]
                .iter().cloned().collect::<HashMap<String, Range<usize>>>() },
            name: "record1".to_string()
        });
        vec.push(Record {
            data: Data { data: "hello3hello4".as_bytes().to_owned(),
            ranges: [("field2".to_string(), 0..6),
            ("field3".to_string(), 6..12)]
            .iter().cloned().collect::<HashMap<String, Range<usize>>>() },
            name: "record1".to_string()
        });
        assert_eq!(vec, reader.iter(&mut buf).map(|r| r.unwrap()).collect::<Vec<Record<HashMap<String, Range<usize>>, Vec<u8>>>>());
        let _ = buf.seek(SeekFrom::Start(0)).unwrap();

        let mut reader = ReaderBuilder::new()
            .with_un_padder(&un_padder)
            .with_recognizer(&recognizer)
            .with_specs(&spec.record_specs)
            .build()
        ;

        let mut vec = Vec::new();
        vec.push(Record {
            data: Data { data: "hellohello2".as_bytes().to_owned(),
            ranges: [("field2".to_string(), 0..5),
                ("field3".to_string(), 5..11)]
                .iter().cloned().collect::<BTreeMap<String, Range<usize>>>() },
            name: "record1".to_string()
        });
        vec.push(Record {
            data: Data { data: "hello3hello4".as_bytes().to_owned(),
            ranges: [("field2".to_string(), 0..6),
                ("field3".to_string(), 6..12)]
                .iter().cloned().collect::<BTreeMap<String, Range<usize>>>() },
            name: "record1".to_string()
        });
        assert_eq!(vec, reader.iter(&mut buf).map(|r| r.unwrap()).collect::<Vec<Record<BTreeMap<String, Range<usize>>, Vec<u8>>>>());

        let _ = buf.seek(SeekFrom::Start(0)).unwrap();

        let mut vec = Vec::new();
        vec.push(Record {
            data: Data { data: "hellohello2".as_bytes().to_owned(),
            ranges: [("field2".to_string(), 0..5),
                ("field3".to_string(), 5..11)]
                .iter().cloned().collect::<BTreeMap<String, Range<usize>>>() },
            name: "record1".to_string()
        });
        vec.push(Record {
            data: Data { data: "hello3hello4".as_bytes().to_owned(),
            ranges: [("field2".to_string(), 0..6),
                ("field3".to_string(), 6..12)]
                .iter().cloned().collect::<BTreeMap<String, Range<usize>>>() },
            name: "record1".to_string()
        });
        assert_eq!(vec, reader.into_iter(buf).map(|r| r.unwrap()).collect::<Vec<Record<BTreeMap<String, Range<usize>>, Vec<u8>>>>());
    }

    #[test]
    fn rewindable_reader() {
        let string = "1234567890qwertyuiopasdfghjkl;zxcvbnm,./-=[];dfszbvvitwyotywt4trjkvvbjsbrgh4oq3njm,k.l/[p]";
        let mut bytes = string.as_bytes();
        let mut buf = RememberingReader::new(&mut bytes);
        let mut data = [0; 45];
        assert_result!(
            Ok(45),
            buf.read(&mut data)
        );
        assert_eq!(&string[..45], ::std::str::from_utf8(&data).unwrap());
        buf.restart();
        let mut data = [0; 30];
        assert_result!(
            Ok(30),
            buf.read(&mut data)
        );
        assert_eq!(&string[..30], ::std::str::from_utf8(&data).unwrap());
        buf.forget();
        buf.restart();
        let mut data = [0; 45];
        assert_result!(
            Ok(45),
            buf.read(&mut data)
        );
        assert_eq!(&string[30..75], ::std::str::from_utf8(&data).unwrap());

        let mut bytes = string.as_bytes();
        let mut buf = RememberingReader::new(&mut bytes);
        assert_result!(
            Ok(45),
            buf.read(&mut data)
        );
        assert_eq!(&string[..45], ::std::str::from_utf8(&data).unwrap());
        buf.restart();
        assert_result!(
            Ok(45),
            buf.read(&mut data)
        );
        assert_eq!(&string[..45], ::std::str::from_utf8(&data).unwrap());
        assert_result!(
            Ok(45),
            buf.read(&mut data)
        );
        assert_eq!(&string[45..], ::std::str::from_utf8(&data).unwrap());
        buf.restart();
        let mut data = String::new();
        assert_result!(
            Ok(90),
            buf.read_to_string(&mut data)
        );
        assert_eq!(string, data);
        buf.restart();
        let mut data = String::new();
        assert_result!(
            Ok(90),
            buf.read_to_string(&mut data)
        );
        assert_eq!(string, data);
    }
}