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

    #[test]
    fn read() {
//        let un_padder = MockPadder::new();
//        let reader = Reader::new(&un_padder, test_spec());
//        let buf = "1234567890qwertyuiopasdfghjkl;zxcvbnm,./-=[];dfszbvvitwyotywt4trjkvvbjsbrgh4oq3njm,k.l/[p]".as_bytes();
//        un_padder.add_unpad_call("");
    }
}