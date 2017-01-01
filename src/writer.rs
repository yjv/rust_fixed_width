use spec::{FileSpec, FieldSpec, Padder};
use std::collections::HashMap;
use std::io::{Write, Error as IoError};

#[derive(Debug)]
pub enum Error<T: Padder> {
    RecordSpecNameRequired,
    RecordSpecNotFound(String),
    FieldSpecNotFound(String, String),
    PaddingFailed(T::Error),
    IoError(IoError),
    NotEnoughWritten(usize, usize),
    FieldValueRequired(String, String)
}

impl<T: Padder> From<IoError> for Error<T> {
    fn from(e: IoError) -> Self {
        Error::IoError(e)
    }
}

pub struct Writer<T: Padder> {
    padder: T,
    spec: FileSpec
}

impl<T: Padder> Writer<T> {
    pub fn write_field<'a, V: 'a + Write>(&self, writer: &'a mut V, record_name: String, name: String, value: String) -> Result<(), Error<T>> {
        let field_spec = self.spec.record_specs
            .get(&record_name)
            .ok_or_else(|| Error::RecordSpecNotFound(record_name.clone()))?
            .field_specs.get(&name)
            .ok_or_else(|| Error::FieldSpecNotFound(record_name.clone(), name.clone()))?
        ;
        Ok(self._write_field(writer, field_spec, value)?)
    }

    pub fn write_record<'a, V: 'a + Write>(&self, writer: &'a mut V, record_name: String, data: HashMap<String, String>) -> Result<(), Error<T>> {
        let record_spec = self.spec.record_specs
            .get(&record_name)
            .ok_or_else(|| Error::RecordSpecNotFound(record_name.clone()))?
        ;
        let mut end = 0;

        for (name, field_spec) in &record_spec.field_specs {
            if field_spec.range.start > end {
                writer.write_all(&mut vec![0; field_spec.range.start - end][..])?;
            }

            end = field_spec.range.end;
            self._write_field(writer, field_spec, data.get(name).or_else(|| field_spec.default.as_ref().clone()).ok_or_else(|| Error::FieldValueRequired(record_name.clone(), name.clone()))?.clone())?;
        }

        if end < self.spec.line_length {
            writer.write_all(&mut vec![0; self.spec.line_length - end][..])?;
        }

        Ok(())
    }

    fn _write_field<'a, V: 'a + Write>(&self, writer: &'a mut V, field_spec: &FieldSpec, value: String) -> Result<(), Error<T>> {
        let length = field_spec.range.end - field_spec.range.start;
        let value = self.padder.pad(value, length, &field_spec.padding, field_spec.padding_direction).map_err(|e| Error::PaddingFailed(e))?;
        Ok(writer.write_all(value.as_bytes())?)
    }
}
#[cfg(test)]
mod test {

    use super::*;
    use test::*;
    use std::iter::repeat;
    use std::collections::HashMap;

    #[test]
    fn writing() {

    }
}