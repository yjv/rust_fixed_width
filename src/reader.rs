use spec::{FileSpec, FieldSpec, UnPadder};
use std::collections::HashMap;
use std::io::{Read, Error as IoError};

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
    spec: FileSpec
}

impl<T: UnPadder> Reader<T> {
    pub fn read_field<'a, V: 'a + Read>(&self, reader: &'a mut V, record_name: String, name: String) -> Result<String, Error<T>> {
        let field_spec = self.spec.record_specs
            .get(&record_name)
            .ok_or_else(|| Error::RecordSpecNotFound(record_name.clone()))?
            .field_specs.get(&name)
            .ok_or_else(|| Error::FieldSpecNotFound(record_name.clone(), name.clone()))?
        ;
        Ok(self._read_field(reader, field_spec)?)
    }

    pub fn read_record<'a, V: 'a + Read>(&self, reader: &'a mut V, record_name: String) -> Result<HashMap<String, String>, Error<T>> {
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
            if amount != length {
                return Err(Error::NotEnoughRead(length, amount))
            }
            string
        };
        Ok(self.un_padder.unpad(data, &field_spec.padding, field_spec.padding_direction).map_err(|e| Error::UnPaddingFailed(e))?)
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