use common::File;
use spec::{FileSpec, FieldSpec, LineRecordSpecRecognizer, NoneRecognizer, UnPadder, IdentityPadder};
use std::collections::HashMap;
use std::io::{Read, Write, Error as IoError};

#[derive(Debug)]
pub enum Error<T: UnPadder> {
    RecordSpecNotFound(String),
    FieldSpecNotFound(String, String),
    RecordSpecNameRequired,
    UnPaddingFailed(T::Error),
    IoError(IoError),
    NotEnoughLeftInReader(usize, usize)
}

impl<T: UnPadder> From<IoError> for Error<T> {
    fn from(e: IoError) -> Self {
        Error::IoError(e)
    }
}

pub struct Reader<T: UnPadder, U: LineRecordSpecRecognizer> {
    un_padder: T,
    recognizer: U,
    spec: FileSpec
}

impl<T: UnPadder, U: LineRecordSpecRecognizer> Reader<T, U> {
    pub fn read_field<'a, V: 'a + Read>(&self, reader: &'a mut V, record_name: String, name: String) -> Result<String, Error<T>> {
        let field_spec = self.spec.record_specs
            .get(&record_name)
            .ok_or_else(|| Error::RecordSpecNotFound(record_name.clone()))?
            .field_specs.get(&name)
            .ok_or_else(|| Error::FieldSpecNotFound(record_name.clone(), name.clone()))?
        ;
        Ok(self._read_field(reader, field_spec)?)
    }

    pub fn read_line<'a, V: 'a + Read>(&self, reader: &'a mut V, record_name: String) -> Result<HashMap<String, String>, Error<T>> {
        let record_spec = self.spec.record_specs
            .get(&record_name)
            .ok_or_else(|| Error::RecordSpecNotFound(record_name.clone()))?
        ;
        let mut data: HashMap<String, String> = HashMap::new();
        for (name, field_spec) in &record_spec.field_specs {
            data.insert(name.clone(), self._read_field(reader, field_spec)?);
        }

        Ok(data)
    }

    fn _read_field<'a, V: 'a + Read>(&self, reader: &'a mut V, field_spec: &FieldSpec) -> Result<String, Error<T>> {
        let length = field_spec.range.end - field_spec.range.start;
        let data = {
            let mut string = String::new();
            let amount = reader.by_ref().take(length as u64).read_to_string(&mut string)?;
            string
        };
        Ok(self.un_padder.unpad(&data, &field_spec.padding, field_spec.padding_direction).map_err(|e| Error::UnPaddingFailed(e))?)
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use super::super::test::*;
    use std::iter::repeat;
    use std::collections::HashMap;

    #[test]
    fn read() {

    }
}