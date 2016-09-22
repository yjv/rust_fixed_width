use common::{File, Line};
use spec::{FileSpec, DataRecordSpecRecognizer};
use std::collections::HashMap;

#[derive(Debug)]
pub enum Error<T: File, U: DataRecordSpecRecognizer> {
    FailedToAddLine(T::Error),
    RecordSpecNotFound(String),
    FailedToRecognizeRecordSpec(U::Error),
    RecordSpecNameRequired,
    FailedToSetDataOnLine(<T::Line as Line>::Error)
}

pub struct FileBuilder<'a, T: File, U: 'a + DataRecordSpecRecognizer> {
    pub file: T,
    spec: &'a FileSpec,
    recognizer: &'a U
}

impl<'a, T: File, U: 'a + DataRecordSpecRecognizer> FileBuilder<'a, T, U> {
    pub fn new(file: T, spec: &'a FileSpec) -> Self {
        FileBuilder { file: file, spec: spec, recognizer: ErrorFieldRecognizer }
    }

    pub fn new_with_recognizer(file: T, spec: &'a FileSpec, recognizer: &'a U) -> Self {
        FileBuilder { file: file, spec: spec, recognizer: recognizer }
    }

    pub fn add_line<V: AsRef<HashMap<String, String>>>(&'a mut self, data: V, spec_name: Option<String>) -> Result<usize, Error<T, U>> {
        let record_spec_name = try!(spec_name.map_or_else(
            || self.recognizer.ok_or(
                Error::RecordSpecNameRequired
            ).and_then(
                |recognizer| recognizer.recognize_for_data(&data, &self.spec.record_specs).map_err(Error::FailedToRecognizeRecordSpec)
            ),
            |name| Ok(name))
        );
        let record_spec = try!(self.spec.record_specs.get(&record_spec_name).ok_or(Error::RecordSpecNotFound(record_spec_name)));
        let index = try!(self.file.add_line().map_err(Error::FailedToAddLine));
        let line = try!(self.file.line_mut(index).map_err(Error::FailedToAddLine)).expect("line just added doesn't exist this shouldn't happen");
        let data = data.as_ref();

        for (name, field_spec) in &record_spec.field_specs {
            if let Some(value) = data.get(name) {
                try!(line.set(field_spec.range.clone(), value).map_err(Error::FailedToSetDataOnLine));
            }
        }

        Ok(index)
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use super::super::spec::*;
    use super::super::in_memory::*;
    use std::collections::HashMap;

    #[test]
    fn building() {
        let mut spec = FileSpec {
            width: 10,
            record_specs: HashMap::new()
        };
        spec.record_specs.insert("record1".to_string(), RecordSpec {
            field_specs: HashMap::new()
        });
        let mut builder = FileBuilder::new(File::new(spec.width), &spec);
    }
}