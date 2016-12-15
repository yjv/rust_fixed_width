use common::File;
use spec::{FileSpec, LineRecordSpecRecognizer, NoneRecognizer, UnPadder, IdentityPadder};
use std::collections::HashMap;

#[derive(Debug)]
pub enum Error<T: File, U: UnPadder> {
    FailedToGetLine(T::Error),
    RecordSpecNotFound(String),
    RecordSpecNameRequired,
    GetFailed(String, T::Error),
    FieldSpecNotFound(String),
    UnPaddingFailed(U::Error)
}

pub struct FileReader<'a, T: 'a + File, U: 'a + LineRecordSpecRecognizer, V: 'a + UnPadder> {
    spec: &'a FileSpec,
    file: T,
    recognizer: U,
    un_padder: V
}

impl<'a, T: 'a + File, U: 'a + LineRecordSpecRecognizer, V: 'a + UnPadder> FileReader<'a, T, U, V> {
    pub fn new(spec: &'a FileSpec, file: T) -> FileReader<'a, T, NoneRecognizer, IdentityPadder> {
        FileReader { spec: spec, file: file, recognizer: NoneRecognizer, un_padder: IdentityPadder }
    }

    pub fn new_with_recognizer_and_un_padder(spec: &'a FileSpec, file: T, recognizer: U, un_padder: V) -> Self {
        FileReader {spec: spec, file: file, recognizer: recognizer, un_padder: un_padder}
    }

    pub fn field(&self, index: usize, name: String, spec_name: Option<String>) -> Result<String, Error<T, V>> {
        let record_spec_name = try!(spec_name.or_else(|| self.recognizer.recognize_for_line(&self.file, index, &self.spec.record_specs)).ok_or(Error::RecordSpecNameRequired));
        let record_spec = try!(self.spec.record_specs.get(
            &record_spec_name
        ).ok_or(Error::RecordSpecNotFound(record_spec_name)));

        let field_spec = try!(record_spec.field_specs.get(&name).ok_or_else(|| Error::FieldSpecNotFound(name.clone())));

        let data = try!(self.file.get(
            index,
            field_spec.range.clone()
        ).map_err(|e| Error::GetFailed(name, e)));

        Ok(try!(self.un_padder.unpad(&data, &field_spec.padding, field_spec.padding_direction).map_err(Error::UnPaddingFailed)))
    }

    pub fn fields(&self, index: usize, spec_name: Option<String>) -> Result<HashMap<String, String>, Error<T, V>> {
        let record_spec_name = try!(spec_name.or_else(|| self.recognizer.recognize_for_line(&self.file, index, &self.spec.record_specs)).ok_or(Error::RecordSpecNameRequired));
        let record_spec = try!(self.spec.record_specs.get(
            &record_spec_name
        ).ok_or(Error::RecordSpecNotFound(record_spec_name)));

        let mut fields = HashMap::new();

        for (name, field_spec) in &record_spec.field_specs {
            let data = try!(self.file.get(index, field_spec.range.clone()).map_err(|e| Error::GetFailed(name.clone(), e)));
            fields.insert(
                name.clone(),
                try!(self.un_padder.unpad(&data, &field_spec.padding, field_spec.padding_direction).map_err(Error::UnPaddingFailed))
            );
        }
        Ok(fields)
    }

    pub fn file(&'a self) -> &'a T {
        &self.file
    }
}

pub struct FileIterator<'a, T: 'a + File, U: 'a + LineRecordSpecRecognizer, V: 'a + UnPadder> {
    position: usize,
    reader: &'a FileReader<'a, T, U, V>
}

impl<'a, T: File, U: LineRecordSpecRecognizer, V: UnPadder> FileIterator<'a, T, U, V> {
    pub fn new(reader: &'a FileReader<'a, T, U, V>) -> Self {
        FileIterator {
            position: 0,
            reader: reader
        }
    }
}

impl<'a, T: File, U: LineRecordSpecRecognizer, V: UnPadder> Iterator for FileIterator<'a, T, U, V> {
    type Item = Result<HashMap<String, String>, Error<T, V>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.position = self.position + 1;
        if self.position >= self.reader.file().len() {
            None
        } else {
            Some(self.reader.fields(self.position - 1, None))
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use super::super::spec::*;
    use super::super::test::*;

    #[test]
    fn read() {
        let spec = FileSpecBuilder::new()
            .with_width(10)
            .with_record("record1", RecordSpecBuilder::new()
                .with_field("field1", FieldSpecBuilder::new()
                    .with_range(0..4)
                    .with_default("hello".to_string())
                    .with_padding("0")
                    .with_padding_direction(PaddingDirection::Left)
                )
            )
        ;
    }
}