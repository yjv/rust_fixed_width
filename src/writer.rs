use common::{File, MutableFile, validate_range, InvalidRangeError};
use spec::{FileSpec, DataRecordSpecRecognizer, LineRecordSpecRecognizer, NoneRecognizer, Padder, IdentityPadder};
use std::collections::HashMap;
use std::ops::Range;

#[derive(Debug)]
pub enum Error<T: File, U: Padder> {
    FailedToAddLine(T::Error),
    RecordSpecNameRequired,
    FailedToSetData(T::Error),
    FailedToGetLine(T::Error),
    RecordSpecNotFound(String),
    InvalidRange(InvalidRangeError),
    PaddingFailed(U::Error)
}

impl<T: File, U: Padder> From<InvalidRangeError> for Error<T, U> {
    fn from(e: InvalidRangeError) -> Self {
        Error::InvalidRange(e)
    }
}

pub struct FileWriter<'a, T: MutableFile, U: DataRecordSpecRecognizer, V: LineRecordSpecRecognizer, W: Padder> {
    file: T,
    spec: &'a FileSpec,
    data_recognizer: U,
    line_recognizer: V,
    padder: W
}

impl<'a, T: MutableFile, U: DataRecordSpecRecognizer, V: LineRecordSpecRecognizer, W: Padder> FileWriter<'a, T, U, V, W> {
    pub fn new(file: T, spec: &'a FileSpec) -> FileWriter<'a, T, NoneRecognizer, NoneRecognizer, IdentityPadder> {
        FileWriter { file: file, spec: spec, data_recognizer: NoneRecognizer, line_recognizer: NoneRecognizer, padder: IdentityPadder }
    }

    pub fn new_with_recognizers_and_padder(file: T, spec: &'a FileSpec, data_recognizer: U, line_recognizer: V, padder: W) -> Self {
        FileWriter { file: file, spec: spec, data_recognizer: data_recognizer, line_recognizer: line_recognizer, padder: padder }
    }

    pub fn add_line(&'a mut self) -> Result<usize, Error<T, W>> {
        Ok(try!(self.file.add_line().map_err(Error::FailedToAddLine)))
    }

    pub fn set_fields(&'a mut self, index: usize, data: &HashMap<String, String>, spec_name: Option<String>) -> Result<&'a mut Self, Error<T, W>> {
        let record_spec_name = try!(
            spec_name
                .or_else(|| self.data_recognizer.recognize_for_data(data, &self.spec.record_specs))
                .or_else(|| self.line_recognizer.recognize_for_line(&self.file, index, &self.spec.record_specs))
                .ok_or(Error::RecordSpecNameRequired)
        );
        let record_spec = try!(self.spec.record_specs.get(&record_spec_name).ok_or(Error::RecordSpecNotFound(record_spec_name)));

        for (name, field_spec) in &record_spec.field_specs {
            if let Some(value) = data.get(name).or(field_spec.default.as_ref()) {
                let (end, start) = try!(validate_range(field_spec.range.clone(), self.file.width(), Some(value)));
                try!(self.file.set(
                    index,
                    field_spec.range.clone(),
                    &try!(self.padder.pad(value, end - start, &field_spec.padding, field_spec.padding_direction).map_err(Error::PaddingFailed))
                ).map_err(Error::FailedToSetData));
            }
        }

        Ok(self)
    }

    pub fn set_field(&'a mut self, index: usize, key: String, value: String, spec_name: Option<String>) -> Result<&'a mut Self, Error<T, W>> {
        let mut data = HashMap::new();
        data.insert(key, value);
        self.set_fields(index, &data, spec_name)
    }

    pub fn file(&'a self) -> &'a T {
        &self.file
    }
}
//
//pub struct FileWriterBuilder {
//    file: T,
//    spec: &'a FileSpec,
//    data_recognizer: U,
//    line_recognizer: V,
//    padder: W
//}

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