use spec::{RecordSpec, FieldSpec};
use padder::{UnPadder, IdentityPadder};
use std::collections::{HashMap};
use std::io::Read;
use std::borrow::Borrow;
use recognizer::{LineBuffer, LineRecordSpecRecognizer, NoneRecognizer};
use super::{Error, Result, PositionalResult, Record, RecordData, PositionalError};

pub struct Reader<T: UnPadder, U: LineRecordSpecRecognizer, V: Borrow<HashMap<String, RecordSpec>>> {
    un_padder: T,
    recognizer: U,
    specs: V
}

impl<T: UnPadder, U: LineRecordSpecRecognizer, V: Borrow<HashMap<String, RecordSpec>>> Reader<T, U, V> {
    pub fn read_field<'a, W: 'a + Read>(&self, reader: &'a mut W, record_name: &'a str, name: &'a str) -> Result<String> {
        let record_spec = self.specs.borrow()
            .get(record_name)
            .ok_or_else(|| Error::RecordSpecNotFound(record_name.to_string()))?
        ;
        let field_spec = record_spec
            .field_specs.get(name)
            .ok_or_else(|| Error::FieldSpecNotFound(record_name.to_string(), name.to_string()))?
        ;
        let field = self._unpad_field(self._read_string(reader, field_spec.length, String::new())?, field_spec)?;

        Ok(field)
    }

    pub fn read_record<'a, W, X, Y>(&self, reader: &'a mut W, record_name: X) -> PositionalResult<Record<Y>>
        where W: 'a + Read,
              X: Into<Option<&'a str>>,
              Y: RecordData
    {
        let mut line = String::new();
        let record_name = record_name
            .into()
            .map_or_else(
                || self.recognizer.recognize_for_line(LineBuffer::new(reader, &mut line), self.specs.borrow()),
                |name| Ok(name.to_string())
            )?
        ;

        let record_spec = self.specs.borrow()
            .get(&record_name)
            .ok_or_else(|| Error::RecordSpecNotFound(record_name.clone()))?
        ;
        let line = self._read_string(reader, record_spec.len(), line).map_err(|e| (e, record_name.clone()))?;
        let mut data = Y::new();
        let mut current_index = 0;

        for (name, field_spec) in &record_spec.field_specs {
            if !field_spec.filler {
                data.insert(name.clone(), self._unpad_field(
                    line[current_index..current_index + field_spec.length].to_string(),
                    &field_spec
                ).map_err(|e| (e, record_name.clone(), name.clone()))?);
            }
            current_index += field_spec.length;
        }

        self.absorb_line_ending(reader, &record_spec.line_ending).map_err(|e| (e, record_name.clone()))?;

        Ok(Record { data: data, name: record_name })
    }

    fn _unpad_field<'a>(&self, field: String, field_spec: &'a FieldSpec) -> Result<String> {
        Ok(self.un_padder.unpad(
            field,
            &field_spec.padding, field_spec.padding_direction)?
        )
    }

    pub fn absorb_line_ending<'a, W: 'a + Read>(&self, reader: &'a mut W, line_ending: &String) -> Result<()> {
        let mut ending = String::new();
        reader.by_ref().take(line_ending.len() as u64).read_to_string(&mut ending)?;
        if ending.len() != 0 && ending != *line_ending {
            return Err(Error::StringDoesNotMatchLineEnding(
                line_ending.clone(),
                ending
            ));
        }

        Ok(())
    }

    fn _read_string<'a, W: 'a + Read>(&self, reader: &'a mut W, length: usize, mut string: String) -> Result<String> {
        let _ = reader.by_ref().take((length - string.len()) as u64).read_to_string(&mut string)?;

        if string.len() < length {
            Err(Error::CouldNotReadEnough(string))
        } else {
            Ok(string)
        }
    }

    pub fn iter<'a, W: 'a + Read, X: RecordData>(&'a self, reader: &'a mut W) -> Iter<'a, T, U, V, W, X> {
        Iter {
            source: reader,
            reader: self,
            marker: ::std::marker::PhantomData
        }
    }

    pub fn into_iter<W: Read, X: RecordData>(self, reader: W) -> IntoIter<T, U, V, W, X> {
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
            specs: self.specs.expect("specs is required to build a writer")
        }
    }
}

pub struct Iter<'a, T: UnPadder + 'a, U: LineRecordSpecRecognizer + 'a, V: Borrow<HashMap<String, RecordSpec>> + 'a, W: Read + 'a, X: RecordData> {
    source: &'a mut W,
    reader: &'a Reader<T, U, V>,
    marker: ::std::marker::PhantomData<X>
}

impl<'a, T: UnPadder + 'a, U: LineRecordSpecRecognizer + 'a, V: Borrow<HashMap<String, RecordSpec>> + 'a, W: Read + 'a, X: RecordData> Iterator for Iter<'a, T, U, V, W, X> {
    type Item = PositionalResult<Record<X>>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.reader.read_record(self.source, None) {
            Err(PositionalError { error: Error::CouldNotReadEnough(ref string), .. }) if string.len() == 0 => None,
            r => Some(r)
        }
    }
}

pub struct IntoIter<T: UnPadder, U: LineRecordSpecRecognizer, V: Borrow<HashMap<String, RecordSpec>>, W: Read, X: RecordData> {
    source: W,
    reader: Reader<T, U, V>,
    marker: ::std::marker::PhantomData<X>,
}

impl<T: UnPadder, U: LineRecordSpecRecognizer, V: Borrow<HashMap<String, RecordSpec>>, W: Read, X: RecordData> Iterator for IntoIter<T, U, V, W, X> {
    type Item = PositionalResult<Record<X>>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.reader.read_record(&mut self.source, None) {
            Err(PositionalError { error: Error::CouldNotReadEnough(ref string), .. }) if string.len() == 0 => None,
            r => Some(r)
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use super::super::{Error, PositionalError, Position, Record};
    use test::*;
    use std::io::Cursor;
    use spec::PaddingDirection;
    use padder::Error as PaddingError;
    use std::collections::{HashMap, BTreeMap};
    use std::io::{Seek, SeekFrom};

    #[test]
    fn read_record() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;zxcvbnm,./-=[];\ndfszbvvitwyotywt4trjkvvbjsbrgh4oq3njm,k.l/[p]";
        let mut buf = Cursor::new(string.as_bytes());
        let mut un_padder = MockPadder::new();
        un_padder.add_unpad_call(string[4..9].to_string(), " ".to_string(), PaddingDirection::Right, Ok("hello".to_string()));
        un_padder.add_unpad_call(string[9..45].to_string(), "xcvcxv".to_string(), PaddingDirection::Right, Ok("hello2".to_string()));
        un_padder.add_unpad_call(string[50..55].to_string(), " ".to_string(), PaddingDirection::Right, Ok("hello3".to_string()));
        un_padder.add_unpad_call(string[55..91].to_string(), "xcvcxv".to_string(), PaddingDirection::Right, Ok("hello4".to_string()));
        let reader = ReaderBuilder::new()
            .with_un_padder(&un_padder)
            .with_specs(spec.record_specs)
            .build()
        ;
        assert_result!(Ok(Record { data: [("field2".to_string(), "hello".to_string()),
            ("field3".to_string(), "hello2".to_string())]
            .iter().cloned().collect::<HashMap<String, String>>(), name: "record1".to_string() }),
            reader.read_record(&mut buf, "record1")
        );
        assert_result!(Ok(Record { data: [("field2".to_string(), "hello3".to_string()),
            ("field3".to_string(), "hello4".to_string())]
            .iter().cloned().collect::<BTreeMap<String, String>>(), name: "record1".to_string() }),
            reader.read_record(&mut buf, "record1")
        );
    }

    #[test]
    fn read_record_with_bad_line_ending() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;zxcvbnm,./-=[];bla";
        let mut buf = Cursor::new(string.as_bytes());
        let mut un_padder = MockPadder::new();
        un_padder.add_unpad_call(string[4..9].to_string(), " ".to_string(), PaddingDirection::Right, Ok("hello".to_string()));
        un_padder.add_unpad_call(string[9..45].to_string(), "xcvcxv".to_string(), PaddingDirection::Right, Ok("hello2".to_string()));
        let reader = ReaderBuilder::new()
            .with_un_padder(&un_padder)
            .with_specs(spec.record_specs)
            .build()
        ;
        assert_result!(
            Err(PositionalError {
                error: Error::StringDoesNotMatchLineEnding(_, _),
                position: Some(Position { ref record, field: None })
            }) if record == "record1",
            reader.read_record::<_, _, HashMap<String, String>>(&mut buf, "record1")
        );
    }

    #[test]
    fn read_record_with_bad_record_name() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;zxcvbnm,./-=[];\ndfszbvvitwyotywt4trjkvvbjsbrgh4oq3njm,k.l/[p]";
        let mut buf = Cursor::new(string.as_bytes());
        let un_padder = MockPadder::new();
        let reader = ReaderBuilder::new()
            .with_un_padder(&un_padder)
            .with_specs(&spec.record_specs)
            .build()
        ;
        assert_result!(
            Err(PositionalError { error: Error::RecordSpecNotFound(ref record_name), .. }) if record_name == "record5",
            reader.read_record::<_, _, HashMap<String, String>>(&mut buf, "record5")
        );
    }

    #[test]
    fn read_record_with_no_record_name() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;zxcvbnm,./-=[];\ndfszbvvitwyotywt4trjkvvbjsbrgh4oq3njm,k.l/[p]";
        let mut buf = Cursor::new(string.as_bytes());
        let un_padder = MockPadder::new();
        let reader = ReaderBuilder::new()
            .with_un_padder(&un_padder)
            .with_specs(&spec.record_specs)
            .build()
        ;
        assert_result!(
            Err(PositionalError { error: Error::RecordSpecNameRequired, .. }),
            reader.read_record::<_, _, HashMap<String, String>>(&mut buf, None)
        );
    }

    #[test]
    fn read_record_with_no_record_name_but_guessable() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;zxcvbnm,./-=[];\ndfszbvvitwyotywt4trjkvvbjsbrgh4oq3njm,k.l/[p]";
        let mut buf = Cursor::new(string.as_bytes());
        let mut un_padder = MockPadder::new();
        un_padder.add_unpad_call(string[4..9].to_string(), " ".to_string(), PaddingDirection::Right, Ok("hello".to_string()));
        un_padder.add_unpad_call(string[9..45].to_string(), "xcvcxv".to_string(), PaddingDirection::Right, Ok("hello2".to_string()));
        let mut recognizer = MockRecognizer::new();
        recognizer.add_line_recognize_call(&spec.record_specs, Ok("record1".to_string()));
        let reader = ReaderBuilder::new()
            .with_un_padder(&un_padder)
            .with_specs(&spec.record_specs)
            .with_recognizer(recognizer)
            .build()
        ;
        assert_result!(Ok(Record { data: [("field2".to_string(), "hello".to_string()),
            ("field3".to_string(), "hello2".to_string())]
            .iter().cloned().collect::<HashMap<String, String>>(), name: "record1".to_string() }),
            reader.read_record(&mut buf, None)
        );
    }

    #[test]
    fn read_record_with_padding_error() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;zxcvbnm,./-=[];\ndfszbvvitwyotywt4trjkvvbjsbrgh4oq3njm,k.l/[p]";
        let mut buf = Cursor::new(string.as_bytes());
        let mut un_padder = MockPadder::new();
        un_padder.add_unpad_call(string[4..9].to_string(), " ".to_string(), PaddingDirection::Right, Err(PaddingError::new("")));
        let reader = ReaderBuilder::new()
            .with_un_padder(&un_padder)
            .with_specs(&spec.record_specs)
            .build()
        ;
        assert_result!(
            Err(PositionalError {
                error: Error::PadderFailure(_),
                position: Some(Position { ref record, field: Some(ref field) })
            }) if record == "record1" && field == "field2",
            reader.read_record::<_, _, HashMap<String, String>>(&mut buf, "record1")
        );
    }

    #[test]
    fn read_record_with_read_error() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;";
        let mut buf = Cursor::new(string.as_bytes());
        let mut un_padder = MockPadder::new();
        un_padder.add_unpad_call(string[4..9].to_string(), " ".to_string(), PaddingDirection::Right, Ok("hello".to_string()));
        let reader = ReaderBuilder::new()
            .with_un_padder(&un_padder)
            .with_specs(&spec.record_specs)
            .build()
        ;
        assert_result!(
            Err(PositionalError {
                error: Error::CouldNotReadEnough(_),
                position: Some(Position { ref record, field: None })
            }) if record == "record1",
            reader.read_record::<_, _, HashMap<String, String>>(&mut buf, "record1")
        );
    }

    #[test]
    fn read_field() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;zxcvbnm,./-=[];dfszbvvitwyotywt4trjkvvbjsbrgh4oq3njm,k.l/[p]";
        let mut buf = Cursor::new(string.as_bytes());
        let mut un_padder = MockPadder::new();
        un_padder.add_unpad_call(string[0..4].to_string(), "dsasd".to_string(), PaddingDirection::Left, Ok("hello".to_string()));
        un_padder.add_unpad_call(string[4..9].to_string(), " ".to_string(), PaddingDirection::Right, Ok("hello2".to_string()));
        let reader = ReaderBuilder::new()
            .with_un_padder(&un_padder)
            .with_specs(&spec.record_specs)
            .build()
        ;
        assert_result!(Ok("hello".to_string()), reader.read_field(&mut buf, "record1", "field1"));
        assert_result!(Ok("hello2".to_string()), reader.read_field(&mut buf, "record1", "field2"));
    }

    #[test]
    fn read_field_with_bad_record_name() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;zxcvbnm,./-=[];dfszbvvitwyotywt4trjkvvbjsbrgh4oq3njm,k.l/[p]";
        let mut buf = Cursor::new(string.as_bytes());
        let un_padder = MockPadder::new();
        let reader = ReaderBuilder::new()
            .with_un_padder(&un_padder)
            .with_specs(&spec.record_specs)
            .build()
        ;
        assert_result!(
            Err(Error::RecordSpecNotFound(ref record_name)) if record_name == "record5",
            reader.read_field(&mut buf, "record5", "field1")
        );
    }

    #[test]
    fn read_field_with_bad_field_name() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;zxcvbnm,./-=[];dfszbvvitwyotywt4trjkvvbjsbrgh4oq3njm,k.l/[p]";
        let mut buf = Cursor::new(string.as_bytes());
        let un_padder = MockPadder::new();
        let reader = ReaderBuilder::new()
            .with_un_padder(&un_padder)
            .with_specs(&spec.record_specs)
            .build()
        ;
        assert_result!(
            Err(Error::FieldSpecNotFound(ref record_name, ref field_name)) if record_name == "record1" && field_name == "field5",
            reader.read_field(&mut buf, "record1", "field5")
        );
    }

    #[test]
    fn read_field_with_padding_error() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;zxcvbnm,./-=[];dfszbvvitwyotywt4trjkvvbjsbrgh4oq3njm,k.l/[p]";
        let mut buf = Cursor::new(string.as_bytes());
        let mut un_padder = MockPadder::new();
        un_padder.add_unpad_call(string[0..4].to_string(), "dsasd".to_string(), PaddingDirection::Left, Err(PaddingError::new("")));
        let reader = ReaderBuilder::new()
            .with_un_padder(&un_padder)
            .with_specs(&spec.record_specs)
            .build()
        ;
        assert_result!(
            Err(Error::PadderFailure(_)),
            reader.read_field(&mut buf, "record1", "field1")
        );
    }

    #[test]
    fn read_field_with_read_error() {
        let spec = test_spec();
        let string = "123";
        let mut buf = Cursor::new(string.as_bytes());
        let reader = ReaderBuilder::new()
            .with_un_padder(MockPadder::new())
            .with_specs(&spec.record_specs)
            .build()
        ;
        assert_result!(
            Err(Error::CouldNotReadEnough(_)),
            reader.read_field(&mut buf, "record1", "field1")
        );
    }

    #[test]
    fn iterator() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;zxcvbnm,./-=[];\ndfszbvvitwyotywt4trjkvvbjsbrgh4oq3njm,k.l/[p]";
        let mut buf = Cursor::new(string.as_bytes());
        let mut un_padder = MockPadder::new();
        un_padder.add_unpad_call(string[4..9].to_string(), " ".to_string(), PaddingDirection::Right, Ok("hello".to_string()));
        un_padder.add_unpad_call(string[9..45].to_string(), "xcvcxv".to_string(), PaddingDirection::Right, Ok("hello2".to_string()));
        un_padder.add_unpad_call(string[50..55].to_string(), " ".to_string(), PaddingDirection::Right, Ok("hello3".to_string()));
        un_padder.add_unpad_call(string[55..91].to_string(), "xcvcxv".to_string(), PaddingDirection::Right, Ok("hello4".to_string()));
        let mut recognizer = MockRecognizer::new();
        recognizer.add_line_recognize_call(&spec.record_specs, Ok("record1".to_string()));
        let reader = ReaderBuilder::new()
            .with_un_padder(&un_padder)
            .with_recognizer(recognizer)
            .with_specs(&spec.record_specs)
            .build()
        ;
        let mut vec = Vec::new();
        vec.push(Record { data: [("field2".to_string(), "hello".to_string()),
            ("field3".to_string(), "hello2".to_string())]
            .iter().cloned().collect::<HashMap<String, String>>(), name: "record1".to_string() });
        vec.push(Record { data: [("field2".to_string(), "hello3".to_string()),
            ("field3".to_string(), "hello4".to_string())]
            .iter().cloned().collect::<HashMap<String, String>>(), name: "record1".to_string() });
        assert_eq!(vec, reader.iter(&mut buf).map(|r| r.unwrap()).collect::<Vec<Record<HashMap<String, String>>>>());
        let _ = buf.seek(SeekFrom::Start(0)).unwrap();

        let mut vec = Vec::new();
        vec.push(Record { data: [("field2".to_string(), "hello".to_string()),
            ("field3".to_string(), "hello2".to_string())]
            .iter().cloned().collect::<BTreeMap<String, String>>(), name: "record1".to_string() });
        vec.push(Record { data: [("field2".to_string(), "hello3".to_string()),
            ("field3".to_string(), "hello4".to_string())]
            .iter().cloned().collect::<BTreeMap<String, String>>(), name: "record1".to_string() });
        assert_eq!(vec, reader.iter(&mut buf).map(|r| r.unwrap()).collect::<Vec<Record<BTreeMap<String, String>>>>());let _ = buf.seek(SeekFrom::Start(0)).unwrap();
        let _ = buf.seek(SeekFrom::Start(0)).unwrap();

        let mut vec = Vec::new();
        vec.push(Record { data: [("field2".to_string(), "hello".to_string()),
            ("field3".to_string(), "hello2".to_string())]
            .iter().cloned().collect::<BTreeMap<String, String>>(), name: "record1".to_string() });
        vec.push(Record { data: [("field2".to_string(), "hello3".to_string()),
            ("field3".to_string(), "hello4".to_string())]
            .iter().cloned().collect::<BTreeMap<String, String>>(), name: "record1".to_string() });
        assert_eq!(vec, reader.into_iter(buf).map(|r| r.unwrap()).collect::<Vec<Record<BTreeMap<String, String>>>>());
    }
}