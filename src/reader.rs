use spec::{RecordSpec};
use padder::{UnPadder, IdentityPadder};
use std::collections::{HashMap};
use std::io::Read;
use std::borrow::Borrow;
use recognizer::{LineBuffer, LineRecordSpecRecognizer, NoneRecognizer};
use super::{Error, Result, PositionalResult, Record, PositionalError};
use record::{Data, DataRanges};

pub struct Reader<T: UnPadder, U: LineRecordSpecRecognizer, V: Borrow<HashMap<String, RecordSpec>>> {
    un_padder: T,
    recognizer: U,
    specs: V,
    buffer: Vec<u8>
}

impl<T: UnPadder, U: LineRecordSpecRecognizer, V: Borrow<HashMap<String, RecordSpec>>> Reader<T, U, V> {
    pub fn read_field<'a, W, X>(&mut self, reader: &'a mut W, record_name: &'a str, name: &'a str, field: X) -> Result<Vec<u8>>
        where W: 'a + Read,
              X: Into<Option<Vec<u8>>>
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

    pub fn read_record<'a, W, X, Y, Z>(&mut self, reader: &'a mut W, record_name: X, line: Z) -> PositionalResult<Record<Y>>
        where W: 'a + Read,
              X: Into<Option<&'a str>>,
              Y: DataRanges,
              Z: Into<Option<Vec<u8>>>
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
        let mut ranges = Y::new();

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

        Ok(Record { data: Data { data: line, ranges: ranges }, name: record_name })
    }

    pub fn absorb_line_ending<'a, W: 'a + Read>(&mut self, reader: &'a mut W, line_ending: &[u8]) -> Result<()> {
        Self::_absorb_line_ending(reader, line_ending, &mut self.buffer)
    }

    fn _absorb_line_ending<'a, W: 'a + Read>(reader: &'a mut W, line_ending: &[u8], buffer: &mut Vec<u8>) -> Result<()> {
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

    fn _read_data<'a, W: 'a + Read>(reader: &'a mut W, length: usize, data: &mut Vec<u8>) -> Result<()> {
        let _ = reader.by_ref().take((length - data.len()) as u64).read_to_end(data)?;

        if data.len() < length {
            Err(Error::CouldNotReadEnough(data.clone()))
        } else {
            Ok(())
        }
    }

    pub fn iter<'a, W: 'a + Read, X: DataRanges>(&'a mut self, reader: &'a mut W) -> Iter<'a, T, U, V, W, X> {
        Iter {
            source: reader,
            reader: self,
            marker: ::std::marker::PhantomData
        }
    }

    pub fn into_iter<W: Read, X: DataRanges>(self, reader: W) -> IntoIter<T, U, V, W, X> {
        IntoIter {
            source: reader,
            reader: self,
            marker: ::std::marker::PhantomData
        }
    }
}

pub struct ReaderBuilder<T: UnPadder, U: LineRecordSpecRecognizer, V: Borrow<HashMap<String, RecordSpec>>> {
    un_padder: Option<T>,
    recognizer: Option<U>,
    specs: Option<V>
}

impl<V: Borrow<HashMap<String, RecordSpec>>> ReaderBuilder<IdentityPadder, NoneRecognizer, V> {
    pub fn new() -> ReaderBuilder<IdentityPadder, NoneRecognizer, V> {
        ReaderBuilder {
            un_padder: Some(IdentityPadder),
            recognizer: Some(NoneRecognizer),
            specs: None
        }
    }
}

impl<T: UnPadder, U: LineRecordSpecRecognizer, V: Borrow<HashMap<String, RecordSpec>>> ReaderBuilder<T, U, V> {
    pub fn with_un_padder<W: UnPadder>(self, padder: W) -> ReaderBuilder<W, U, V> {
        ReaderBuilder {
            un_padder: Some(padder),
            recognizer: self.recognizer,
            specs: self.specs
        }
    }

    pub fn with_recognizer<W: LineRecordSpecRecognizer>(self, recognizer: W) -> ReaderBuilder<T, W, V> {
        ReaderBuilder {
            un_padder: self.un_padder,
            recognizer: Some(recognizer),
            specs: self.specs
        }
    }

    pub fn with_specs(mut self, specs: V) -> Self {
        self.specs = Some(specs);
        self
    }

    pub fn build(self) -> Reader<T, U, V> {
        Reader {
            un_padder: self.un_padder.unwrap(),
            recognizer: self.recognizer.unwrap(),
            specs: self.specs.expect("specs is required to build a writer"),
            buffer: Vec::new()
        }
    }
}

pub struct Iter<'a, T: UnPadder + 'a, U: LineRecordSpecRecognizer + 'a, V: Borrow<HashMap<String, RecordSpec>> + 'a, W: Read + 'a, X: DataRanges> {
    source: &'a mut W,
    reader: &'a mut Reader<T, U, V>,
    marker: ::std::marker::PhantomData<X>
}

impl<'a, T: UnPadder + 'a, U: LineRecordSpecRecognizer + 'a, V: Borrow<HashMap<String, RecordSpec>> + 'a, W: Read + 'a, X: DataRanges> Iterator for Iter<'a, T, U, V, W, X> {
    type Item = PositionalResult<Record<X>>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.reader.read_record(self.source, None, Vec::new()) {
            Err(PositionalError { error: Error::CouldNotReadEnough(ref string), .. }) if string.len() == 0 => None,
            r => Some(r)
        }
    }
}

pub struct IntoIter<T: UnPadder, U: LineRecordSpecRecognizer, V: Borrow<HashMap<String, RecordSpec>>, W: Read, X: DataRanges> {
    source: W,
    reader: Reader<T, U, V>,
    marker: ::std::marker::PhantomData<X>,
}

impl<T: UnPadder, U: LineRecordSpecRecognizer, V: Borrow<HashMap<String, RecordSpec>>, W: Read, X: DataRanges> Iterator for IntoIter<T, U, V, W, X> {
    type Item = PositionalResult<Record<X>>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.reader.read_record(&mut self.source, None, Vec::new()) {
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
            .with_specs(spec.record_specs)
            .build()
        ;
        assert_result!(Ok(Record {
            data: Data { data: "hellohello2".as_bytes().to_owned(),
            ranges: [("field2".to_owned(), 0..5),
                ("field3".to_owned(), 5..11)]
                .iter().cloned().collect::<HashMap<String, Range<usize>>>() },
            name: "record1".to_string()
        }), reader.read_record(&mut buf, "record1", Vec::new()));
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
            reader.read_record::<_, _, HashMap<String, Range<usize>>, _>(&mut buf, "record1", None)
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
            reader.read_record::<_, _, HashMap<String, Range<usize>>, _>(&mut buf, "record5", Vec::new())
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
            reader.read_record::<_, _, HashMap<String, Range<usize>>, _>(&mut buf, None, Vec::new())
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
        let mut recognizer = MockRecognizer::new();
        recognizer.add_line_recognize_call(&spec.record_specs, Ok("record1".to_string()));
        let mut reader = ReaderBuilder::new()
            .with_un_padder(&un_padder)
            .with_specs(&spec.record_specs)
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
            .build()
        ;
        assert_result!(
            Err(PositionalError {
                error: Error::PadderFailure(_),
                position: Some(Position { ref record, field: Some(ref field) })
            }) if record == "record1" && field == "field2",
            reader.read_record::<_, _, HashMap<String, Range<usize>>, _>(&mut buf, "record1", None)
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
            .build()
        ;
        assert_result!(
            Err(PositionalError {
                error: Error::CouldNotReadEnough(_),
                position: Some(Position { ref record, field: Some(ref field) })
            }) if record == "record1" && field == "field3",
            reader.read_record::<_, _, HashMap<String, Range<usize>>, _>(&mut buf, "record1", None)
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
        let mut recognizer = MockRecognizer::new();
        recognizer.add_line_recognize_call(&spec.record_specs, Ok("record1".to_string()));
        let mut reader = ReaderBuilder::new()
            .with_un_padder(&un_padder)
            .with_recognizer(recognizer)
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
        assert_eq!(vec, reader.iter(&mut buf).map(|r| r.unwrap()).collect::<Vec<Record<HashMap<String, Range<usize>>>>>());
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
        assert_eq!(vec, reader.iter(&mut buf).map(|r| r.unwrap()).collect::<Vec<Record<BTreeMap<String, Range<usize>>>>>());let _ = buf.seek(SeekFrom::Start(0)).unwrap();
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
        assert_eq!(vec, reader.into_iter(buf).map(|r| r.unwrap()).collect::<Vec<Record<BTreeMap<String, Range<usize>>>>>());
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