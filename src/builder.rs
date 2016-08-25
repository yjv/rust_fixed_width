use common::{File, Line, ToField};
use spec::{FileSpec, RecordSpec, DataRecordSpecRecognizer};
use std::collections::HashMap;

#[derive(Debug)]
pub enum Error<T: File, U: DataRecordSpecRecognizer> {
    FailedToAddLine(T::Error),
    RecordSpecNotFound(String),
    FailedToRecognizeRecordSpec(U::Error),
    RecordSpecNameRequired,
}

pub struct FileBuilder<'a, T: File, U: 'a + DataRecordSpecRecognizer> {
    pub file: T,
    spec: &'a FileSpec,
    recognizer: Option<&'a U>
}

impl<'a, T: File, U: 'a + DataRecordSpecRecognizer> FileBuilder<'a, T, U> {
    pub fn new(file: T, spec: &'a FileSpec, recognizer: Option<&'a U>) -> Self {
        FileBuilder { file: file, spec: spec, recognizer: recognizer }
    }

    pub fn add_line<'b, V: AsRef<HashMap<String, String>>>(&'a mut self, data: V, spec_name: Option<String>) -> Result<usize, Error<T, U>> {
        let record_spec_name = try!(spec_name.map_or_else(
            || self.recognizer.ok_or(
                Error::RecordSpecNameRequired
            ).and_then(
                |recognizer| recognizer.recognize_for_data(data.as_ref(), self.spec).map_err(Error::FailedToRecognizeRecordSpec)
            ),
            |name| Ok(name))
        );
        let record_spec = try!(self.spec.record_specs.get(&record_spec_name).ok_or(Error::RecordSpecNotFound(record_spec_name)));
        Ok(try!(self.file.add_line().map_err(Error::FailedToAddLine)))
    }
}

#[derive(Debug)]
pub enum DataError<T: ToField> {
    ToFieldFail(T::Error)
}

pub struct DataBuilder {
    pub data: HashMap<String, String>
}

impl DataBuilder {
    pub fn new() -> Self {
        DataBuilder { data: HashMap::new() }
    }

    pub fn set_field<'b, T: 'b + ToField>(&mut self, name: String, value: &'b T) -> Result<(), DataError<T>> {
        self.data.insert(name, try!(value.to_field().map_err(DataError::ToFieldFail)));
        Ok(())
    }
}

impl AsRef<HashMap<String, String>> for DataBuilder {
    fn as_ref(&self) -> &HashMap<String, String> {
        &self.data
    }
}

#[cfg(test)]
mod test {

    #[test]
    fn ranges_work() {
    }
}