use spec::{RecordSpec, FieldSpec};
use padders::UnPadder;
use std::collections::HashMap;
use std::io::{Read, Error as IoError, ErrorKind};

#[derive(Debug)]
pub enum Error<T: UnPadder> {
    RecordSpecNotFound(String),
    FieldSpecNotFound(String, String),
    RecordSpecNameRequired,
    UnPaddingFailed(T::Error),
    IoError(IoError),
    NotEnoughRead(usize, usize)
}

impl<T: UnPadder> From<IoError> for Error<T> {
    fn from(e: IoError) -> Self {
        Error::IoError(e)
    }
}

pub struct Reader<T: UnPadder> {
    un_padder: T,
    specs: HashMap<String, RecordSpec>
}

impl<T: UnPadder> Reader<T> {
    pub fn new(un_padder: T, specs: HashMap<String, RecordSpec>) -> Self {
        Reader {
            un_padder: un_padder,
            specs: specs
        }
    }

    pub fn read_field<'a, V: 'a + Read>(&self, reader: &'a mut V, record_name: String, name: String) -> Result<String, Error<T>> {
        let field_spec = self.specs
            .get(&record_name)
            .ok_or_else(|| Error::RecordSpecNotFound(record_name.clone()))?
            .field_specs.get(&name)
            .ok_or_else(|| Error::FieldSpecNotFound(record_name.clone(), name.clone()))?
        ;
        Ok(self._unpad_field(self._read_string(reader, field_spec.length)?, field_spec)?)
    }

    pub fn read_record<'a, V: 'a + Read>(&self, reader: &'a mut V, record_name: String) -> Result<HashMap<String, String>, Error<T>> {
        let record_spec = self.specs
            .get(&record_name)
            .ok_or_else(|| Error::RecordSpecNotFound(record_name.clone()))?
        ;
        let line = self._read_string(reader, record_spec.line_spec.length)?;
        let mut data: HashMap<String, String> = HashMap::new();
        for (name, field_spec) in &record_spec.field_specs {
            if field_spec.ignore {
                continue;
            }
            data.insert(name.clone(), self._unpad_field(
                line[field_spec.index..field_spec.index + field_spec.length].to_string(),
                &field_spec
            )?);
        }

        Ok(data)
    }

    fn _unpad_field<'a>(&self, field: String, field_spec: &'a FieldSpec) -> Result<String, Error<T>> {
        Ok(self.un_padder.unpad(
            field,
            &field_spec.padding, field_spec.padding_direction).map_err(|e| Error::UnPaddingFailed(e))?
        )
    }

    fn _read_string<'a, V: 'a + Read>(&self, reader: &'a mut V, length: usize) -> Result<String, IoError> {
        let mut data = vec![0; length];
        reader.read_exact(&mut data[..])?;
        String::from_utf8(data).map_err(|_| IoError::new(ErrorKind::InvalidData, "stream did not contain valid UTF-8"))
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use test::*;
    use std::iter::repeat;
    use std::collections::HashMap;
    use std::io::{Read, Seek, Cursor};
    use spec::PaddingDirection;

    #[test]
    fn read_record() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;zxcvbnm,./-=[];dfszbvvitwyotywt4trjkvvbjsbrgh4oq3njm,k.l/[p]";
        let mut buf = Cursor::new(string.as_bytes());
        let mut un_padder = MockPadder::new();
        un_padder.add_unpad_call(string[4..9].to_string(), " ".to_string(), PaddingDirection::Right, Ok("hello".to_string()));
        un_padder.add_unpad_call(string[9..45].to_string(), "xcvcxv".to_string(), PaddingDirection::Right, Ok("hello2".to_string()));
        let reader = Reader::new(&un_padder, spec.record_specs);
        assert_eq!([("field2".to_string(), "hello".to_string()),
            ("field3".to_string(), "hello2".to_string())]
            .iter().cloned().collect::<HashMap<String, String>>(), reader.read_record(&mut buf, "record1".to_string()).unwrap());
    }

    #[test]
    fn read_record_with_bad_record_name() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;zxcvbnm,./-=[];dfszbvvitwyotywt4trjkvvbjsbrgh4oq3njm,k.l/[p]";
        let mut buf = Cursor::new(string.as_bytes());
        let mut un_padder = MockPadder::new();
        let reader = Reader::new(&un_padder, spec.record_specs);
        match reader.read_record(&mut buf, "record5".to_string()) {
            Err(Error::RecordSpecNotFound(record_name)) => assert_eq!("record5".to_string(), record_name),
            _ => panic!("should have returned a record spec not found error")
        }
    }

    #[test]
    fn read_record_with_padding_error() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;zxcvbnm,./-=[];dfszbvvitwyotywt4trjkvvbjsbrgh4oq3njm,k.l/[p]";
        let mut buf = Cursor::new(string.as_bytes());
        let mut un_padder = MockPadder::new();
        un_padder.add_unpad_call(string[4..9].to_string(), " ".to_string(), PaddingDirection::Right, Err(()));
        let reader = Reader::new(&un_padder, spec.record_specs);
        reader.read_record(&mut buf, "record1".to_string()).unwrap_err();
    }

    #[test]
    fn read_record_with_read_error() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;";
        let mut buf = Cursor::new(string.as_bytes());
        let mut un_padder = MockPadder::new();
        un_padder.add_unpad_call(string[4..9].to_string(), " ".to_string(), PaddingDirection::Right, Ok("hello".to_string()));
        let reader = Reader::new(&un_padder, spec.record_specs);
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
        let reader = Reader::new(&un_padder, spec.record_specs);
        assert_eq!("hello".to_string(), reader.read_field(&mut buf, "record1".to_string(), "field1".to_string()).unwrap());
        assert_eq!("hello2".to_string(), reader.read_field(&mut buf, "record1".to_string(), "field2".to_string()).unwrap());
    }

    #[test]
    fn read_field_with_bad_record_name() {
        let spec = test_spec();
        let string = "1234567890qwertyuiopasdfghjkl;zxcvbnm,./-=[];dfszbvvitwyotywt4trjkvvbjsbrgh4oq3njm,k.l/[p]";
        let mut buf = Cursor::new(string.as_bytes());
        let mut un_padder = MockPadder::new();
        let reader = Reader::new(&un_padder, spec.record_specs);
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
        let mut un_padder = MockPadder::new();
        let reader = Reader::new(&un_padder, spec.record_specs);
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
        un_padder.add_unpad_call(string[0..4].to_string(), "dsasd".to_string(), PaddingDirection::Left, Err(()));
        let reader = Reader::new(&un_padder, spec.record_specs);
        reader.read_field(&mut buf, "record1".to_string(), "field1".to_string()).unwrap_err();
    }

    #[test]
    fn read_field_with_read_error() {
        let spec = test_spec();
        let string = "123";
        let mut buf = Cursor::new(string.as_bytes());
        let reader = Reader::new(MockPadder::new(), spec.record_specs);
        reader.read_field(&mut buf, "record1".to_string(), "field1".to_string()).unwrap_err();
    }
}