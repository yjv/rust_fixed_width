use spec::{RecordSpec, FieldSpec};
use padders::{UnPadder, IdentityPadder};
use std::collections::HashMap;
use std::io::{Read, Error as IoError, ErrorKind};
use std::borrow::Borrow;
use super::recognizers::{LineBuffer, LineRecordSpecRecognizer, NoneRecognizer};
use super::{Error, Result};

pub struct Reader<T: UnPadder, U: LineRecordSpecRecognizer, V: Borrow<HashMap<String, RecordSpec>>> {
    un_padder: T,
    recognizer: U,
    specs: V
}

impl<T: UnPadder, U: LineRecordSpecRecognizer, V: Borrow<HashMap<String, RecordSpec>>> Reader<T, U, V> {
    pub fn read_field<'a, W: 'a + Read>(&self, reader: &'a mut W, record_name: String, name: String) -> Result<String> {
        let record_spec = self.specs.borrow()
            .get(&record_name)
            .ok_or_else(|| Error::RecordSpecNotFound(record_name.clone()))?
        ;
        let field_spec = record_spec
            .field_specs.get(&name)
            .ok_or_else(|| Error::FieldSpecNotFound(record_name.clone(), name.clone()))?
        ;
        let field = self._unpad_field(self._read_string(reader, field_spec.length, String::new())?, field_spec)?;

        if record_spec.field_range(&name).expect("Should never be none").end == record_spec.len() {
            self._absorb_line_ending(reader, record_spec)?;
        }

        Ok(field)
    }

    pub fn read_record<'a, W: 'a + Read, X>(&self, reader: &'a mut W, record_name: X) -> Result<HashMap<String, String>>
        where X: Into<Option<String>>
    {
        let mut line = String::new();
        let record_name = record_name
            .into()
            .map_or_else(
                || self.recognizer.recognize_for_line(LineBuffer::new(reader, &mut line), self.specs.borrow()),
                |name| Ok(name)
            )?
        ;

        let record_spec = self.specs.borrow()
            .get(&record_name)
            .ok_or_else(|| Error::RecordSpecNotFound(record_name.clone()))?
        ;
        let line = self._read_string(reader, record_spec.len(), line)?;
        let mut data: HashMap<String, String> = HashMap::new();
        let mut current_index = 0;

        for (name, field_spec) in &record_spec.field_specs {
            if !field_spec.ignore {
                data.insert(name.clone(), self._unpad_field(
                    line[current_index..current_index + field_spec.length].to_string(),
                    &field_spec
                )?);
            }
            current_index += field_spec.length;
        }

        self._absorb_line_ending(reader, record_spec)?;

        Ok(data)
    }

    fn _unpad_field<'a>(&self, field: String, field_spec: &'a FieldSpec) -> Result<String> {
        Ok(self.un_padder.unpad(
            field,
            &field_spec.padding, field_spec.padding_direction)?
        )
    }

    fn _read_string<'a, W: 'a + Read>(&self, reader: &'a mut W, length: usize, string: String) -> ::std::result::Result<String, IoError> {
        let original_length = string.len();
        let mut data = string.into_bytes();
        data.resize(length, 0);
        reader.read_exact(&mut data[original_length..])?;
        String::from_utf8(data).map_err(|e| IoError::new(ErrorKind::InvalidData, e))
    }

    fn _absorb_line_ending<'a, W: 'a + Read>(&self, reader: &'a mut W, record_spec: &RecordSpec) -> Result<()> {
        let mut ending = String::new();
        reader.by_ref().take(record_spec.line_ending.len() as u64).read_to_string(&mut ending)?;
        if ending.len() != 0 && ending != record_spec.line_ending {
            return Err(Error::StringDoesNotMatchLineEnding(
                record_spec.line_ending.clone(),
                ending
            ));
        }

        Ok(())
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

#[cfg(test)]
mod test {

    use super::*;
    use super::super::*;
    use test::*;
    use std::collections::HashMap;
    use std::io::Cursor;
    use spec::PaddingDirection;
    use padders::Error as PaddingError;

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
        assert_eq!([("field2".to_string(), "hello".to_string()),
            ("field3".to_string(), "hello2".to_string())]
            .iter().cloned().collect::<HashMap<String, String>>(), reader.read_record(&mut buf, "record1".to_string()).unwrap())
        ;
        assert_eq!([("field2".to_string(), "hello3".to_string()),
            ("field3".to_string(), "hello4".to_string())]
            .iter().cloned().collect::<HashMap<String, String>>(), reader.read_record(&mut buf, "record1".to_string()).unwrap())
        ;
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
        assert_result!(Err(Error::StringDoesNotMatchLineEnding(_, _)), reader.read_record(&mut buf, "record1".to_string()));
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
        match reader.read_record(&mut buf, "record5".to_string()) {
            Err(Error::RecordSpecNotFound(record_name)) => assert_eq!("record5".to_string(), record_name),
            _ => panic!("should have returned a record spec not found error")
        }
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
        match reader.read_record(&mut buf, None) {
            Err(Error::RecordSpecNameRequired) => (),
            _ => panic!("should have returned a record spec name required error")
        }
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
        assert_eq!([("field2".to_string(), "hello".to_string()),
            ("field3".to_string(), "hello2".to_string())]
            .iter().cloned().collect::<HashMap<String, String>>(), reader.read_record(&mut buf, None).unwrap());
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
        reader.read_record(&mut buf, "record1".to_string()).unwrap_err();
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
        reader.read_record(&mut buf, "record1".to_string()).unwrap_err();
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
        assert_eq!("hello".to_string(), reader.read_field(&mut buf, "record1".to_string(), "field1".to_string()).unwrap());
        assert_eq!("hello2".to_string(), reader.read_field(&mut buf, "record1".to_string(), "field2".to_string()).unwrap());
    }

    #[test]
    fn read_field_with_bad_line_ending() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;zxcvbnm,./-=[];dfszbvvitwyotywt4trjkvvbjsbrgh4oq3njm,k.l/[p]";
        let mut buf = Cursor::new(string.as_bytes());
        let mut un_padder = MockPadder::new();
        un_padder.add_unpad_call(string[0..36].to_string(), "xcvcxv".to_string(), PaddingDirection::Right, Ok("hello2".to_string()));
        let reader = ReaderBuilder::new()
            .with_un_padder(&un_padder)
            .with_specs(&spec.record_specs)
            .build()
        ;
        assert_result!(Err(Error::StringDoesNotMatchLineEnding(_, _)), reader.read_field(&mut buf, "record1".to_string(), "field3".to_string()));
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
        match reader.read_field(&mut buf, "record5".to_string(), "field1".to_string()) {
            Err(Error::RecordSpecNotFound(record_name)) => assert_eq!("record5".to_string(), record_name),
            _ => panic!("should have returned a record spec not found error")
        }
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
        match reader.read_field(&mut buf, "record1".to_string(), "field5".to_string()) {
            Err(Error::FieldSpecNotFound(record_name, field_name)) => {
                assert_eq!("record1".to_string(), record_name);
                assert_eq!("field5".to_string(), field_name);
            },
            _ => panic!("should have returned a field spec not found error")
        }
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
        reader.read_field(&mut buf, "record1".to_string(), "field1".to_string()).unwrap_err();
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
        reader.read_field(&mut buf, "record1".to_string(), "field1".to_string()).unwrap_err();
    }
}