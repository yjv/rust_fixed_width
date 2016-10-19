use common::File;
use spec::{FileSpec, DataRecordSpecRecognizer, LineRecordSpecRecognizer, NoneRecognizer};
use std::collections::HashMap;

#[derive(Debug)]
pub enum Error<T: File> {
    FailedToAddLine(T::Error),
    RecordSpecNameRequired,
    FailedToSetData(T::Error),
    FailedToGetLine(T::Error),
    RecordSpecNotFound(String),
}

pub struct FileWriter<'a, T: 'a + File, U: DataRecordSpecRecognizer, V: LineRecordSpecRecognizer> {
    file: &'a mut T,
    spec: &'a FileSpec,
    data_recognizer: U,
    line_recognizer: V
}

impl<'a, T: File, U: DataRecordSpecRecognizer, V: LineRecordSpecRecognizer> FileWriter<'a, T, U, V> {
    pub fn new(file: &'a mut T, spec: &'a FileSpec) -> FileWriter<'a, T, NoneRecognizer, NoneRecognizer> {
        FileWriter { file: file, spec: spec, data_recognizer: NoneRecognizer, line_recognizer: NoneRecognizer }
    }

    pub fn new_with_recognizers(file: &'a mut T, spec: &'a FileSpec, data_recognizer: U, line_recognizer: V) -> Self {
        FileWriter { file: file, spec: spec, data_recognizer: data_recognizer, line_recognizer: line_recognizer }
    }

    pub fn add_line(&'a mut self, data: &HashMap<String, String>, spec_name: Option<String>) -> Result<usize, Error<T>> {
        try!(self.file.add_line().map_err(Error::FailedToAddLine))
    }

    pub fn set_fields(&'a mut self, index: usize, data: &HashMap<String, String>, spec_name: Option<String>) -> Result<&'a mut Self, Error<T>> {
        let line = match self.file.line_mut(index).map_err(Error::FailedToGetLine) {
            Ok(Some(line)) => line,
            Err(error) => return Err(error),
            Ok(None) => return Ok(None)
        };

        let record_spec_name = try!(
            spec_name
                .or_else(|| self.data_recognizer.recognize_for_data(data, &self.spec.record_specs))
                .or_else(|| self.line_recognizer.recognize_for_line(line, &self.spec.record_specs))
                .ok_or(Error::RecordSpecNameRequired)
        );
        let record_spec = try!(self.spec.record_specs.get(&record_spec_name).ok_or(Error::RecordSpecNotFound(record_spec_name)));

        for (name, field_spec) in &record_spec.field_specs {
            if let Some(value) = data.get(name).or(field_spec.default.as_ref()) {
                try!(self.line.set(field_spec.range.clone(), value).map_err(Error::FailedToSetData));
            }
        }

        Ok(self)
    }

    pub fn set_field(&'a mut self, index: usize, key: String, value: String, spec_name: Option<String>) -> Result<&'a mut Self, Error<T>> {
        let mut data = HashMap::new();
        data.insert(key, value);
        self.set_fields(index, &data, spec_name)
    }
}

#[cfg(test)]
mod test {

//    use super::*;
//    use super::super::spec::*;
//    use super::super::in_memory::*;
//    use std::collections::HashMap;
//
//    #[test]
//    fn building() {
//        let mut spec = FileSpec {
//            width: 10,
//            record_specs: HashMap::new()
//        };
//        spec.record_specs.insert("record1".to_string(), RecordSpec {
//            field_specs: HashMap::new()
//        });
//        spec.record_specs.get("record1".to_string()).field_specs.insert("field1", FieldSpec {
//            default: None,
//
//        });
//        spec.record_specs.insert("record2".to_string(), RecordSpec {
//            field_specs: HashMap::new()
//        });
//        let mut builder = FileBuilder::new(File::new(spec.width), &spec);
//    }
}