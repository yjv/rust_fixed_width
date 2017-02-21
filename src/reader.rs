use spec::{RecordSpec};
use padder::{UnPadder, IdentityPadder};
use std::collections::{HashMap};
use std::io::Read;
use std::borrow::Borrow;
use recognizer::{LineBuffer, LineRecordSpecRecognizer, NoneRecognizer};
use super::{Error, Result, PositionalResult, Record, PositionalError};
use record::{Data, DataRanges, ReadableDataType, BinaryType};

pub struct Reader<T: UnPadder<W>, U: LineRecordSpecRecognizer<W>, V: Borrow<HashMap<String, RecordSpec>>, W: ReadableDataType> {
    un_padder: T,
    recognizer: U,
    specs: V,
    buffer: Vec<u8>,
    data_type: W
}

impl<T: UnPadder<W>, U: LineRecordSpecRecognizer<W>, V: Borrow<HashMap<String, RecordSpec>>, W: ReadableDataType> Reader<T, U, V, W> {
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
            &self.data_type
        )?;
        Ok(field)
    }

    pub fn read_record<'a, X, Y, Z, A>(&mut self, reader: &'a mut X, record_name: Y, line: Z) -> PositionalResult<Record<A, W::DataHolder>>
        where X: 'a + Read,
              Y: Into<Option<&'a str>>,
              Z: Into<Option<Vec<u8>>>,
              A: DataRanges
    {
        let mut line = line.into().unwrap_or_else(|| Vec::new());
        let mut reader = RewindableReader::new(reader);
        let record_name = record_name
            .into()
            .map_or_else(
                || self.recognizer.recognize_for_line(LineBuffer::new(&mut reader, &mut line), self.specs.borrow(), &self.data_type),
                |name| Ok(name.to_string())
            )?
        ;

        let record_spec = self.specs.borrow()
            .get(&record_name)
            .ok_or_else(|| Error::RecordSpecNotFound(record_name.clone()))?
        ;
        reader.rewind();
        let mut ranges = A::new();

        self.buffer.clear();

        for (name, field_spec) in &record_spec.field_specs {
            Self::_read_data(&mut reader, field_spec.length, &mut self.buffer).map_err(|e| (e, record_name.clone(), name.clone()))?;
            if !field_spec.filler {
                let old_length = line.len();
                self.un_padder.unpad(
                    &self.buffer[..],
                    &field_spec.padding,
                    field_spec.padding_direction,
                    &mut line,
                    &self.data_type
                ).map_err(|e| (e, record_name.clone(), name.clone()))?;
                ranges.insert(name, old_length..line.len());
            }

            self.buffer.clear();
        }

        Self::_absorb_line_ending(&mut reader, &record_spec.line_ending, &mut self.buffer).map_err(|e| (e, record_name.clone()))?;

        Ok(Record { data: Data { data: self.data_type.new_data_holder(line)?, ranges: ranges }, name: record_name })
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

    pub fn iter<'a, X: 'a + Read, Y: DataRanges + 'a>(&'a mut self, reader: &'a mut X) -> Iter<'a, X, T, U, V, W, Y> {
        Iter {
            source: reader,
            reader: self,
            marker: ::std::marker::PhantomData
        }
    }

    pub fn into_iter<X: Read, Y: DataRanges>(self, reader: X) -> IntoIter<X, T, U, V, W, Y> {
        IntoIter {
            source: reader,
            reader: self,
            marker: ::std::marker::PhantomData
        }
    }
}

pub struct ReaderBuilder<T: UnPadder<W>, U: LineRecordSpecRecognizer<W>, V: Borrow<HashMap<String, RecordSpec>>, W: ReadableDataType> {
    un_padder: T,
    recognizer: U,
    specs: Option<V>,
    data_type: W
}

impl<V: Borrow<HashMap<String, RecordSpec>>> ReaderBuilder<IdentityPadder, NoneRecognizer, V, BinaryType> {
    pub fn new() -> ReaderBuilder<IdentityPadder, NoneRecognizer, V, BinaryType> {
        ReaderBuilder {
            un_padder: IdentityPadder,
            recognizer: NoneRecognizer,
            specs: None,
            data_type: BinaryType
        }
    }
}

impl<T: UnPadder<W>, U: LineRecordSpecRecognizer<W>, V: Borrow<HashMap<String, RecordSpec>>, W: ReadableDataType> ReaderBuilder<T, U, V, W> {
    pub fn from_reader(reader: Reader<T, U, V, W>) -> Self {
        ReaderBuilder {
            un_padder: reader.un_padder,
            recognizer: reader.recognizer,
            data_type: reader.data_type,
            specs: Some(reader.specs)
        }
    }

    pub fn with_un_padder<X: UnPadder<W>>(self, un_padder: X) -> ReaderBuilder<X, U, V, W> {
        ReaderBuilder {
            un_padder: un_padder,
            recognizer: self.recognizer,
            specs: self.specs,
            data_type: self.data_type
        }
    }

    pub fn with_recognizer<X: LineRecordSpecRecognizer<W>>(self, recognizer: X) -> ReaderBuilder<T, X, V, W> {
        ReaderBuilder {
            un_padder: self.un_padder,
            recognizer: recognizer,
            specs: self.specs,
            data_type: self.data_type
        }
    }

    pub fn with_specs(mut self, specs: V) -> Self {
        self.specs = Some(specs);
        self
    }

    pub fn with_data_type<X: ReadableDataType>(self, data_type: X) -> ReaderBuilder<T, U, V, X>
        where T: UnPadder<X>,
              U: LineRecordSpecRecognizer<X>
    {
        ReaderBuilder {
            un_padder: self.un_padder,
            recognizer: self.recognizer,
            specs: self.specs,
            data_type: data_type
        }
    }

    pub fn build(self) -> Reader<T, U, V, W> {
        Reader {
            un_padder: self.un_padder,
            recognizer: self.recognizer,
            specs: self.specs.expect("specs is required to build a writer"),
            buffer: Vec::new(),
            data_type: self.data_type
        }
    }
}

pub struct Iter<'a, T: Read + 'a, U: UnPadder<X> + 'a, V: LineRecordSpecRecognizer<X> + 'a, W: Borrow<HashMap<String, RecordSpec>> + 'a, X: ReadableDataType + 'a, Y: DataRanges + 'a> {
    source: &'a mut T,
    reader: &'a mut Reader<U, V, W, X>,
    marker: ::std::marker::PhantomData<Y>
}

impl<'a, T: Read + 'a, U: UnPadder<X> + 'a, V: LineRecordSpecRecognizer<X> + 'a, W: Borrow<HashMap<String, RecordSpec>> + 'a, X: ReadableDataType + 'a, Y: DataRanges + 'a> Iterator for Iter<'a, T, U, V, W, X, Y> {
    type Item = PositionalResult<Record<Y, X::DataHolder>>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.reader.read_record(self.source, None, None) {
            Err(PositionalError { error: Error::CouldNotReadEnough(ref string), .. }) if string.len() == 0 => None,
            r => Some(r)
        }
    }
}

pub struct IntoIter<T: Read, U: UnPadder<X>, V: LineRecordSpecRecognizer<X>, W: Borrow<HashMap<String, RecordSpec>>, X: ReadableDataType, Y: DataRanges> {
    source: T,
    reader: Reader<U, V, W, X>,
    marker: ::std::marker::PhantomData<Y>
}

impl<T: Read, U: UnPadder<X>, V: LineRecordSpecRecognizer<X>, W: Borrow<HashMap<String, RecordSpec>>, X: ReadableDataType, Y: DataRanges> Iterator for IntoIter<T, U, V, W, X, Y> {
    type Item = PositionalResult<Record<Y, X::DataHolder>>;
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

    pub fn reset(&mut self) {
        self.buf = self.buf.split_off(self.pos);
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
            .with_specs(&spec.record_specs)
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
            reader.read_record::<_, _, _, BTreeMap<_, _>>(&mut buf, "record1", None)
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
            reader.read_record::<_, _, _, BTreeMap<_, _>>(&mut buf, "record5", Vec::new())
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
            reader.read_record::<_, _, _, BTreeMap<_, _>>(&mut buf, None, Vec::new())
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
            reader.read_record::<_, _, _, BTreeMap<_, _>>(&mut buf, "record1", None)
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
            reader.read_record::<_, _, _, BTreeMap<_, _>>(&mut buf, "record1", None)
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
        buf.reset();
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