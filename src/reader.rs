use common::{File, Range};
use spec::{FileSpec, LineRecordSpecRecognizer, NoneRecognizer};
use std::collections::HashMap;

#[derive(Debug)]
pub enum Error<T: File> {
    FailedToGetLine(T::Error),
    RecordSpecNotFound(String),
    RecordSpecNameRequired,
    GetFailed(String, T::Error),
    FieldSpecNotFound(String)
}

pub struct FileReader<'a, T: File, U: 'a + Range, V: LineRecordSpecRecognizer> {
    spec: &'a FileSpec<U>,
    file: T,
    recognizer: V
}

impl<'a, T: 'a + File, U: 'a + Range, V: 'a + LineRecordSpecRecognizer> FileReader<'a, T, U, V> {
    pub fn new(spec: &'a FileSpec<U>, file: T) -> FileReader<'a, T, U, NoneRecognizer> {
        FileReader { spec: spec, file: file, recognizer: NoneRecognizer }
    }

    pub fn new_with_recognizer(spec: &'a FileSpec<U>, file: T, recognizer: V) -> Self {
        FileReader {spec: spec, file: file, recognizer: recognizer}
    }

    pub fn field(&self, index: usize, name: String, spec_name: Option<String>) -> Result<String, Error<T>> {
        let record_spec_name = try!(spec_name.or_else(|| self.recognizer.recognize_for_line(&self.file, index, &self.spec.record_specs)).ok_or(Error::RecordSpecNameRequired));
        let record_spec = try!(self.spec.record_specs.get(
            &record_spec_name
        ).ok_or(Error::RecordSpecNotFound(record_spec_name)));

        Ok(try!(self.file.get(
            index,
            try!(record_spec.field_specs.get(&name).ok_or_else(|| Error::FieldSpecNotFound(name.clone()))).range.clone()
        ).map_err(|e| Error::GetFailed(name, e))))
    }

    pub fn fields(&self, index: usize, spec_name: Option<String>) -> Result<HashMap<String, String>, Error<T>> {
        let record_spec_name = try!(spec_name.or_else(|| self.recognizer.recognize_for_line(&self.file, index, &self.spec.record_specs)).ok_or(Error::RecordSpecNameRequired));
        let record_spec = try!(self.spec.record_specs.get(
            &record_spec_name
        ).ok_or(Error::RecordSpecNotFound(record_spec_name)));

        let mut fields = HashMap::new();

        for (name, field_spec) in &record_spec.field_specs {
            fields.insert(name.clone(), try!(self.file.get(index, field_spec.range.clone()).map_err(|e| Error::GetFailed(name.clone(), e))));
        }
        Ok(fields)
    }

    pub fn file(&'a self) -> &'a T {
        &self.file
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
    type Item = Result<HashMap<String, String>, Error<T>>;

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

    #[test]
    fn ranges_work() {
    }
}