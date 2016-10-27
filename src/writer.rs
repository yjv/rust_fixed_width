use common::{File, MutableFile};
use spec::{FileSpec, DataRecordSpecRecognizer, LineRecordSpecRecognizer, NoneRecognizer};
use std::collections::HashMap;

#[derive(Debug)]
pub enum Error<T: File> {
    FailedToAddLine(T::Error),
    RecordSpecNameRequired,
    FailedToSetData(T::Error),
    FailedToGetLine(T::Error),
    RecordSpecNotFound(String)
}

pub struct FileWriter<'a, T: MutableFile, U: DataRecordSpecRecognizer, V: LineRecordSpecRecognizer> {
    file: T,
    spec: &'a FileSpec,
    data_recognizer: U,
    line_recognizer: V
}

impl<'a, T: MutableFile, U: DataRecordSpecRecognizer, V: LineRecordSpecRecognizer> FileWriter<'a, T, U, V> {
    pub fn new(file: T, spec: &'a FileSpec) -> FileWriter<'a, T, NoneRecognizer, NoneRecognizer> {
        FileWriter { file: file, spec: spec, data_recognizer: NoneRecognizer, line_recognizer: NoneRecognizer }
    }

    pub fn new_with_recognizers(file: T, spec: &'a FileSpec, data_recognizer: U, line_recognizer: V) -> Self {
        FileWriter { file: file, spec: spec, data_recognizer: data_recognizer, line_recognizer: line_recognizer }
    }

    pub fn add_line(&'a mut self) -> Result<usize, Error<T>> {
        Ok(try!(self.file.add_line().map_err(Error::FailedToAddLine)))
    }

    pub fn set_fields(&'a mut self, index: usize, data: &HashMap<String, String>, spec_name: Option<String>) -> Result<&'a mut Self, Error<T>> {
        let record_spec_name = try!(
            spec_name
                .or_else(|| self.data_recognizer.recognize_for_data(data, &self.spec.record_specs))
                .or_else(|| self.line_recognizer.recognize_for_line(&self.file, index, &self.spec.record_specs))
                .ok_or(Error::RecordSpecNameRequired)
        );
        let record_spec = try!(self.spec.record_specs.get(&record_spec_name).ok_or(Error::RecordSpecNotFound(record_spec_name)));

        for (name, field_spec) in &record_spec.field_specs {
            if let Some(value) = data.get(name).or(field_spec.default.as_ref()) {
                try!(self.file.set(index, field_spec.range.clone(), value).map_err(Error::FailedToSetData));
            }
        }

        Ok(self)
    }

    pub fn set_field(&'a mut self, index: usize, key: String, value: String, spec_name: Option<String>) -> Result<&'a mut Self, Error<T>> {
        let mut data = HashMap::new();
        data.insert(key, value);
        self.set_fields(index, &data, spec_name)
    }

    pub fn file(&'a self) -> &'a T {
        &self.file
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