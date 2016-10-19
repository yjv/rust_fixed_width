use common::{File, Range};
use spec::{FileSpec, RecordSpec, LineRecordSpecRecognizer, NoneRecognizer};
use std::collections::HashMap;

#[derive(Debug)]
pub enum Error<T: File> {
    FailedToGetLine(T::Error),
    RecordSpecNotFound(String),
    RecordSpecNameRequired,
    GetFailed(T::Error),
    FieldSpecNotFound(String)
}

pub struct FileReader<'a, T: 'a + File, U: 'a + Range, V: LineRecordSpecRecognizer> {
    spec: &'a FileSpec<U>,
    file: &'a T,
    recognizer: V
}

impl<'a, T: 'a + File, U: 'a + Range, V: 'a + LineRecordSpecRecognizer> FileReader<'a, T, U, V> {
    pub fn new(spec: &'a FileSpec<U>, file: &'a T) -> FileReader<'a, T, U, NoneRecognizer> {
        FileReader { spec: spec, file: file, recognizer: NoneRecognizer }
    }

    pub fn new_with_recognizer(spec: &'a FileSpec<U>, file: &'a T, recognizer: V) -> Self {
        FileReader {spec: spec, file: file, recognizer: recognizer}
    }

    pub fn field(&self, index: usize, name: String, spec_name: Option<String>) -> Result<Option<String>, Error<T>> {
        let record_spec_name = try!(spec_name.or_else(|| self.recognizer.recognize_for_line(self.file, index, &self.spec.record_specs)).ok_or(Error::RecordSpecNameRequired));
        let record_spec = try!(self.spec.record_specs.get(
            &record_spec_name
        ).ok_or(Error::RecordSpecNotFound(record_spec_name)));

        Ok(try!(self.file.get(
            index,
            try!(record_spec.field_specs.get(&name).ok_or(Error::FieldSpecNotFound(name))).range.clone()
        ).map_err(Error::GetFailed)))
    }

    pub fn fields(&self, index: usize, spec_name: Option<String>) -> Result<Option<HashMap<String, Result<String, Error<T>>>>, Error<T>> {
        let record_spec_name = try!(spec_name.or_else(|| self.recognizer.recognize_for_line(self.file, index, &self.spec.record_specs)).ok_or(Error::RecordSpecNameRequired));
        let record_spec = try!(self.spec.record_specs.get(
            &record_spec_name
        ).ok_or(Error::RecordSpecNotFound(record_spec_name)));
        Ok(Some(record_spec.field_specs.iter().map(|(name, field_spec)| (name.clone(), self.file.get(
            index,
            field_spec.range.clone()
        ).map_err(Error::GetFailed))).collect()))
    }
}

pub struct FileIterator<'a, T: 'a + File, U: 'a + Range, V: 'a + LineRecordSpecRecognizer> {
    position: usize,
    reader: &'a FileReader<'a, T, U, V>
}

impl<'a, T: 'a + File, U: 'a + Range, V: 'a + LineRecordSpecRecognizer> FileIterator<'a, T, U, V> {
    pub fn new(reader: &'a FileReader<'a, T, U, V>) -> Self {
        FileIterator {
            position: 0,
            reader: reader
        }
    }
}

impl<'a, T: 'a + File, U: 'a + Range, V: 'a + LineRecordSpecRecognizer> Iterator for FileIterator<'a, T, U, V> {
    type Item = Result<Option<HashMap<String, Result<String, Error<T>>>>, Error<T>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.position = self.position + 1;
        match self.reader.fields(self.position - 1, None) {
            Ok(Some(line)) => Some(Ok(line)),
            Err(error) => Some(Err(error)),
            Ok(None) => None
        }
    }
}

#[cfg(test)]
mod test {

    #[test]
    fn ranges_work() {
    }
}