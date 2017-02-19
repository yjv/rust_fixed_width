use spec::{RecordSpec};
use padder::{UnPadder, IdentityPadder};
use std::collections::{HashMap, BTreeMap};
use std::ops::Range;
use std::io::Read;
use std::borrow::Borrow;
use recognizer::{LineBuffer, LineRecordSpecRecognizer, NoneRecognizer};
use super::{Error, Result, PositionalResult, Record, PositionalError};
use record::{Data, DataRanges, DataHolder, RecordType};

pub struct Reader<T: UnPadder, U: LineRecordSpecRecognizer, V: Borrow<HashMap<String, RecordSpec>>, W: DataRanges, X: DataHolder> {
    un_padder: T,
    recognizer: U,
    specs: V,
    buffer: Vec<u8>,
    #[allow(dead_code)]
    record_type: RecordType<W, X>
}

impl<T: UnPadder, U: LineRecordSpecRecognizer, V: Borrow<HashMap<String, RecordSpec>>, W: DataRanges, X: DataHolder> Reader<T, U, V, W, X> {
    pub fn read_field<'a, Y, Z>(&mut self, reader: &'a mut Y, record_name: &'a str, name: &'a str, field: Z) -> Result<Vec<u8>>
        where Y: 'a + Read,
              Z: Into<Option<Vec<u8>>>
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
            &mut field
        )?;
        Ok(field)
    }

    pub fn read_record<'a, Y, Z, A>(&mut self, reader: &'a mut Y, record_name: Z, line: A) -> PositionalResult<Record<W, X>>
        where Y: 'a + Read,
              Z: Into<Option<&'a str>>,
              A: Into<Option<Vec<u8>>>
    {
        let mut line = line.into().unwrap_or_else(|| Vec::new());
        let mut reader = RewindableReader::new(reader);
        let record_name = record_name
            .into()
            .map_or_else(
                || self.recognizer.recognize_for_line(LineBuffer::new(&mut reader, &mut line), self.specs.borrow()),
                |name| Ok(name.to_string())
            )?
        ;

        let record_spec = self.specs.borrow()
            .get(&record_name)
            .ok_or_else(|| Error::RecordSpecNotFound(record_name.clone()))?
        ;
        reader.rewind();
        let mut ranges = W::new();

        self.buffer.clear();

        for (name, field_spec) in &record_spec.field_specs {
            Self::_read_data(&mut reader, field_spec.length, &mut self.buffer).map_err(|e| (e, record_name.clone(), name.clone()))?;
            if !field_spec.filler {
                let old_length = line.len();
                self.un_padder.unpad(
                    &self.buffer[..],
                    &field_spec.padding,
                    field_spec.padding_direction,
                    &mut line
                ).map_err(|e| (e, record_name.clone(), name.clone()))?;
                ranges.insert(name.clone(), old_length..line.len());
            }
            self.buffer.clear();
        }

        Self::_absorb_line_ending(&mut reader, &record_spec.line_ending, &mut self.buffer).map_err(|e| (e, record_name.clone()))?;

        Ok(Record { data: Data { data: X::new(line)?, ranges: ranges }, name: record_name })
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

    pub fn iter<'a, Y: 'a + Read>(&'a mut self, reader: &'a mut Y) -> Iter<'a, T, U, V, Y, W, X> {
        Iter {
            source: reader,
            reader: self
        }
    }

    pub fn into_iter<Y: Read>(self, reader: Y) -> IntoIter<T, U, V, Y, W, X> {
        IntoIter {
            source: reader,
            reader: self
        }
    }
}

pub struct ReaderBuilder<T: UnPadder, U: LineRecordSpecRecognizer, V: Borrow<HashMap<String, RecordSpec>>, W: DataRanges, X: DataHolder> {
    un_padder: T,
    recognizer: U,
    specs: Option<V>,
    record_type: RecordType<W, X>
}

impl<V: Borrow<HashMap<String, RecordSpec>>> ReaderBuilder<IdentityPadder, NoneRecognizer, V, BTreeMap<String, Range<usize>>, Vec<u8>> {
    pub fn new() -> ReaderBuilder<IdentityPadder, NoneRecognizer, V, BTreeMap<String, Range<usize>>, Vec<u8>> {
        ReaderBuilder {
            un_padder: IdentityPadder,
            recognizer: NoneRecognizer,
            specs: None,
            record_type: RecordType::<BTreeMap<String, Range<usize>>, Vec<u8>>::new()
        }
    }
}

impl<T: UnPadder, U: LineRecordSpecRecognizer, V: Borrow<HashMap<String, RecordSpec>>, W: DataRanges, X: DataHolder> ReaderBuilder<T, U, V, W, X> {
    pub fn with_un_padder<Z: UnPadder>(self, un_padder: Z) -> ReaderBuilder<Z, U, V, W, X> {
        ReaderBuilder {
            un_padder: un_padder,
            recognizer: self.recognizer,
            specs: self.specs,
            record_type: self.record_type
        }
    }

    pub fn with_recognizer<Z: LineRecordSpecRecognizer>(self, recognizer: Z) -> ReaderBuilder<T, Z, V, W, X> {
        ReaderBuilder {
            un_padder: self.un_padder,
            recognizer: recognizer,
            specs: self.specs,
            record_type: self.record_type
        }
    }

    pub fn with_specs(mut self, specs: V) -> Self {
        self.specs = Some(specs);
        self
    }

    pub fn with_record_type<Z: DataRanges, A: DataHolder>(self) -> ReaderBuilder<T, U, V, Z, A> {
        ReaderBuilder {
            un_padder: self.un_padder,
            recognizer: self.recognizer,
            specs: self.specs,
            record_type: RecordType::<Z, A>::new()
        }
    }

    pub fn build(self) -> Reader<T, U, V, W, X> {
        Reader {
            un_padder: self.un_padder,
            recognizer: self.recognizer,
            specs: self.specs.expect("specs is required to build a writer"),
            buffer: Vec::new(),
            record_type: self.record_type
        }
    }
}

pub struct Iter<'a, T: UnPadder + 'a, U: LineRecordSpecRecognizer + 'a, V: Borrow<HashMap<String, RecordSpec>> + 'a, W: Read + 'a, X: DataRanges + 'a, Y: DataHolder + 'a> {
    source: &'a mut W,
    reader: &'a mut Reader<T, U, V, X, Y>
}

impl<'a, T: UnPadder + 'a, U: LineRecordSpecRecognizer + 'a, V: Borrow<HashMap<String, RecordSpec>> + 'a, W: Read + 'a, X: DataRanges, Y: DataHolder> Iterator for Iter<'a, T, U, V, W, X, Y> {
    type Item = PositionalResult<Record<X, Y>>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.reader.read_record(self.source, None, None) {
            Err(PositionalError { error: Error::CouldNotReadEnough(ref string), .. }) if string.len() == 0 => None,
            r => Some(r)
        }
    }
}

pub struct IntoIter<T: UnPadder, U: LineRecordSpecRecognizer, V: Borrow<HashMap<String, RecordSpec>>, W: Read, X: DataRanges, Y: DataHolder> {
    source: W,
    reader: Reader<T, U, V, X, Y>
}

impl<T: UnPadder, U: LineRecordSpecRecognizer, V: Borrow<HashMap<String, RecordSpec>>, W: Read, X: DataRanges, Y: DataHolder> Iterator for IntoIter<T, U, V, W, X, Y> {
    type Item = PositionalResult<Record<X, Y>>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.reader.read_record(&mut self.source, None, None) {
            Err(PositionalError { error: Error::CouldNotReadEnough(ref string), .. }) if string.len() == 0 => None,
            r => Some(r)
        }
    }
}

pub struct RewindableReader<T: Read> {
    inner: T,
    pos: usize,
    buf: Vec<u8>
}

impl<T: Read> RewindableReader<T> {
    pub fn new(reader: T) -> Self {
        RewindableReader {
            inner: reader,
            pos: 0,
            buf: Vec::new()
        }
    }

    pub fn rewind(&mut self) {
        self.pos = 0;
    }

    pub fn forget(&mut self) {
        self.buf.clear();
        self.rewind();
    }

    pub fn into_inner(self) -> T { self.inner }

    pub fn get_ref(&self) -> &T { &self.inner }

    pub fn get_mut(&mut self) -> &mut T { &mut self.inner }
}

impl<T: Read> Read for RewindableReader<T> {
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

#[cfg(test)]
mod test {

    use super::*;
    use super::super::{Error, PositionalError, Position, Record, Data};
    use test::*;
    use std::io::Cursor;
    use spec::PaddingDirection;
    use padder::Error as PaddingError;
    use std::collections::{HashMap, BTreeMap};
    use std::io::{Seek, SeekFrom, Read};
    use std::ops::Range;

    #[test]
    fn read_record() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;zxcvbnm,./-=[];\ndfszbvvitwyotywt4trjkvvbjsbrgh4oq3njm,k.l/[p]";
        let mut buf = Cursor::new(string.as_bytes());
        let mut un_padder = MockPadder::new();
        un_padder.add_unpad_call(string[4..9].as_bytes().to_owned(), " ".as_bytes().to_owned(), PaddingDirection::Right, Ok("hello".as_bytes().to_owned()));
        un_padder.add_unpad_call(string[9..45].as_bytes().to_owned(), "xcvcxv".as_bytes().to_owned(), PaddingDirection::Right, Ok("hello2".as_bytes().to_owned()));
        un_padder.add_unpad_call(string[50..55].as_bytes().to_owned(), " ".as_bytes().to_owned(), PaddingDirection::Right, Ok("hello3".as_bytes().to_owned()));
        un_padder.add_unpad_call(string[55..91].as_bytes().to_owned(), "xcvcxv".as_bytes().to_owned(), PaddingDirection::Right, Ok("hello4".as_bytes().to_owned()));
        let mut reader = ReaderBuilder::new()
            .with_un_padder(&un_padder)
            .with_specs(&spec.record_specs)
            .with_record_type::<HashMap<_, _>, Vec<_>>()
            .build()
        ;
        assert_result!(Ok(Record {
            data: Data { data: "hellohello2".as_bytes().to_owned(),
            ranges: [("field2".to_owned(), 0..5),
                ("field3".to_owned(), 5..11)]
                .iter().cloned().collect::<HashMap<String, Range<usize>>>() },
            name: "record1".to_string()
        }), reader.read_record(&mut buf, "record1", Vec::new()));
        let mut reader = ReaderBuilder::new()
            .with_un_padder(&un_padder)
            .with_specs(&spec.record_specs)
            .build()
        ;
        assert_result!(Ok(Record {
            data: Data { data: "hello3hello4".as_bytes().to_owned(),
            ranges: [("field2".to_owned(), 0..6),
                ("field3".to_owned(), 6..12)]
                .iter().cloned().collect::<BTreeMap<String, Range<usize>>>() },
            name: "record1".to_string()
        }), reader.read_record(&mut buf, "record1", Vec::new()));
    }

    #[test]
    fn read_record_with_bad_line_ending() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;zxcvbnm,./-=[];bla";
        let mut buf = Cursor::new(string.as_bytes());
        let mut un_padder = MockPadder::new();
        un_padder.add_unpad_call(string[4..9].as_bytes().to_owned(), " ".as_bytes().to_owned(), PaddingDirection::Right, Ok("hello".as_bytes().to_owned()));
        un_padder.add_unpad_call(string[9..45].as_bytes().to_owned(), "xcvcxv".as_bytes().to_owned(), PaddingDirection::Right, Ok("hello2".as_bytes().to_owned()));
        let mut reader = ReaderBuilder::new()
            .with_un_padder(&un_padder)
            .with_specs(spec.record_specs)
            .build()
        ;
        assert_result!(
            Err(PositionalError {
                error: Error::DataDoesNotMatchLineEnding(_, _),
                position: Some(Position { ref record, field: None })
            }) if record == "record1",
            reader.read_record(&mut buf, "record1", None)
        );
    }

    #[test]
    fn read_record_with_bad_record_name() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;zxcvbnm,./-=[];\ndfszbvvitwyotywt4trjkvvbjsbrgh4oq3njm,k.l/[p]";
        let mut buf = Cursor::new(string.as_bytes());
        let un_padder = MockPadder::new();
        let mut reader = ReaderBuilder::new()
            .with_un_padder(&un_padder)
            .with_specs(&spec.record_specs)
            .build()
        ;
        assert_result!(
            Err(PositionalError { error: Error::RecordSpecNotFound(ref record_name), .. }) if record_name == "record5",
            reader.read_record(&mut buf, "record5", Vec::new())
        );
    }

    #[test]
    fn read_record_with_no_record_name() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;zxcvbnm,./-=[];\ndfszbvvitwyotywt4trjkvvbjsbrgh4oq3njm,k.l/[p]";
        let mut buf = Cursor::new(string.as_bytes());
        let un_padder = MockPadder::new();
        let mut reader = ReaderBuilder::new()
            .with_un_padder(&un_padder)
            .with_specs(&spec.record_specs)
            .build()
        ;
        assert_result!(
            Err(PositionalError { error: Error::RecordSpecNameRequired, .. }),
            reader.read_record(&mut buf, None, Vec::new())
        );
    }

    #[test]
    fn read_record_with_no_record_name_but_guessable() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;zxcvbnm,./-=[];\ndfszbvvitwyotywt4trjkvvbjsbrgh4oq3njm,k.l/[p]";
        let mut buf = Cursor::new(string.as_bytes());
        let mut un_padder = MockPadder::new();
        un_padder.add_unpad_call(string[4..9].as_bytes().to_owned(), " ".as_bytes().to_owned(), PaddingDirection::Right, Ok("hello".as_bytes().to_owned()));
        un_padder.add_unpad_call(string[9..45].as_bytes().to_owned(), "xcvcxv".as_bytes().to_owned(), PaddingDirection::Right, Ok("hello2".as_bytes().to_owned()));
        let mut recognizer = MockRecognizer::<()>::new();
        recognizer.add_line_recognize_call(&spec.record_specs, Ok("record1".to_string()));
        let mut reader = ReaderBuilder::new()
            .with_un_padder(&un_padder)
            .with_specs(&spec.record_specs)
            .with_record_type::<HashMap<_, _>, Vec<_>>()
            .with_recognizer(recognizer)
            .build()
        ;
        assert_result!(Ok(Record {
            data: Data { data: "hellohello2".as_bytes().to_owned(),
            ranges: [("field2".to_owned(), 0..5),
                ("field3".to_owned(), 5..11)]
                .iter().cloned().collect::<HashMap<String, Range<usize>>>() },
            name: "record1".to_string()
        }), reader.read_record(&mut buf, None, None));
    }

    #[test]
    fn read_record_with_padding_error() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;zxcvbnm,./-=[];\ndfszbvvitwyotywt4trjkvvbjsbrgh4oq3njm,k.l/[p]";
        let mut buf = Cursor::new(string.as_bytes());
        let mut un_padder = MockPadder::new();
        un_padder.add_unpad_call(string[4..9].as_bytes().to_owned(), " ".as_bytes().to_owned(), PaddingDirection::Right, Err(PaddingError::new("")));
        let mut reader = ReaderBuilder::new()
            .with_un_padder(&un_padder)
            .with_specs(&spec.record_specs)
            .with_record_type::<HashMap<_, _>, Vec<_>>()
            .build()
        ;
        assert_result!(
            Err(PositionalError {
                error: Error::PadderFailure(_),
                position: Some(Position { ref record, field: Some(ref field) })
            }) if record == "record1" && field == "field2",
            reader.read_record(&mut buf, "record1", None)
        );
    }

    #[test]
    fn read_record_with_read_error() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;";
        let mut buf = Cursor::new(string.as_bytes());
        let mut un_padder = MockPadder::new();
        un_padder.add_unpad_call(string[4..9].as_bytes().to_owned(), " ".as_bytes().to_owned(), PaddingDirection::Right, Ok("hello".as_bytes().to_owned()));
        let mut reader = ReaderBuilder::new()
            .with_un_padder(&un_padder)
            .with_specs(&spec.record_specs)
            .with_record_type::<HashMap<_, _>, Vec<_>>()
            .build()
        ;
        assert_result!(
            Err(PositionalError {
                error: Error::CouldNotReadEnough(_),
                position: Some(Position { ref record, field: Some(ref field) })
            }) if record == "record1" && field == "field3",
            reader.read_record(&mut buf, "record1", None)
        );
    }

    #[test]
    fn read_field() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;zxcvbnm,./-=[];dfszbvvitwyotywt4trjkvvbjsbrgh4oq3njm,k.l/[p]";
        let mut buf = Cursor::new(string.as_bytes());
        let mut un_padder = MockPadder::new();
        un_padder.add_unpad_call(string[0..4].as_bytes().to_owned(), "dsasd".as_bytes().to_owned(), PaddingDirection::Left, Ok("hello".as_bytes().to_owned()));
        un_padder.add_unpad_call(string[4..9].as_bytes().to_owned(), " ".as_bytes().to_owned(), PaddingDirection::Right, Ok("hello2".as_bytes().to_owned()));
        let mut reader = ReaderBuilder::new()
            .with_un_padder(&un_padder)
            .with_specs(&spec.record_specs)
            .build()
        ;
        assert_result!(Ok("hello".as_bytes().to_owned()), reader.read_field(&mut buf, "record1", "field1", None));
        assert_result!(Ok("hello2".as_bytes().to_owned()), reader.read_field(&mut buf, "record1", "field2", None));
    }

    #[test]
    fn read_field_with_bad_record_name() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;zxcvbnm,./-=[];dfszbvvitwyotywt4trjkvvbjsbrgh4oq3njm,k.l/[p]";
        let mut buf = Cursor::new(string.as_bytes());
        let un_padder = MockPadder::new();
        let mut reader = ReaderBuilder::new()
            .with_un_padder(&un_padder)
            .with_specs(&spec.record_specs)
            .build()
        ;
        assert_result!(
            Err(Error::RecordSpecNotFound(ref record_name)) if record_name == "record5",
            reader.read_field(&mut buf, "record5", "field1", None)
        );
    }

    #[test]
    fn read_field_with_bad_field_name() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;zxcvbnm,./-=[];dfszbvvitwyotywt4trjkvvbjsbrgh4oq3njm,k.l/[p]";
        let mut buf = Cursor::new(string.as_bytes());
        let un_padder = MockPadder::new();
        let mut reader = ReaderBuilder::new()
            .with_un_padder(&un_padder)
            .with_specs(&spec.record_specs)
            .build()
        ;
        assert_result!(
            Err(Error::FieldSpecNotFound(ref record_name, ref field_name)) if record_name == "record1" && field_name == "field5",
            reader.read_field(&mut buf, "record1", "field5", None)
        );
    }

    #[test]
    fn read_field_with_padding_error() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;zxcvbnm,./-=[];dfszbvvitwyotywt4trjkvvbjsbrgh4oq3njm,k.l/[p]";
        let mut buf = Cursor::new(string.as_bytes());
        let mut un_padder = MockPadder::new();
        un_padder.add_unpad_call(string[0..4].as_bytes().to_owned(), "dsasd".as_bytes().to_owned(), PaddingDirection::Left, Err(PaddingError::new("")));
        let mut reader = ReaderBuilder::new()
            .with_un_padder(&un_padder)
            .with_specs(&spec.record_specs)
            .build()
        ;
        assert_result!(
            Err(Error::PadderFailure(_)),
            reader.read_field(&mut buf, "record1", "field1", None)
        );
    }

    #[test]
    fn read_field_with_read_error() {
        let spec = test_spec();
        let string = "123";
        let mut buf = Cursor::new(string.as_bytes());
        let mut reader = ReaderBuilder::new()
            .with_un_padder(MockPadder::new())
            .with_specs(&spec.record_specs)
            .build()
        ;
        assert_result!(
            Err(Error::CouldNotReadEnough(_)),
            reader.read_field(&mut buf, "record1", "field1", None)
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
        let mut recognizer = MockRecognizer::<()>::new();
        recognizer.add_line_recognize_call(&spec.record_specs, Ok("record1".to_string()));
        let mut reader = ReaderBuilder::new()
            .with_un_padder(&un_padder)
            .with_recognizer(&recognizer)
            .with_record_type::<HashMap<_, _>, Vec<_>>()
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
        let mut buf = RewindableReader::new(&mut bytes);
        let mut data = [0; 45];
        assert_result!(
            Ok(45),
            buf.read(&mut data)
        );
        assert_eq!(&string[..45], ::std::str::from_utf8(&data).unwrap());
        buf.forget();
        buf.rewind();
        assert_result!(
            Ok(45),
            buf.read(&mut data)
        );
        assert_eq!(&string[45..], ::std::str::from_utf8(&data).unwrap());
        buf.rewind();
        assert_result!(
            Ok(45),
            buf.read(&mut data)
        );
        assert_eq!(&string[45..], ::std::str::from_utf8(&data).unwrap());
        let mut bytes = string.as_bytes();
        let mut buf = RewindableReader::new(&mut bytes);
        assert_result!(
            Ok(45),
            buf.read(&mut data)
        );
        assert_eq!(&string[..45], ::std::str::from_utf8(&data).unwrap());
        buf.rewind();
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
        buf.rewind();
        let mut data = String::new();
        assert_result!(
            Ok(90),
            buf.read_to_string(&mut data)
        );
        assert_eq!(string, data);
        buf.rewind();
        let mut data = String::new();
        assert_result!(
            Ok(90),
            buf.read_to_string(&mut data)
        );
        assert_eq!(string, data);
    }
}