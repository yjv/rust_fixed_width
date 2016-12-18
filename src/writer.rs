use common::{File, MutableFile};
use spec::{FileSpec, DataRecordSpecRecognizer, LineRecordSpecRecognizer, NoneRecognizer, Padder, IdentityPadder};
use std::collections::HashMap;

#[derive(Debug)]
pub enum Error<T: File, U: Padder> {
    FailedToAddLine(T::Error),
    RecordSpecNameRequired,
    FailedToSetData(T::Error),
    FailedToGetLine(T::Error),
    RecordSpecNotFound(String),
    PaddingFailed(U::Error)
}

pub struct FileWriter<'a, T: DataRecordSpecRecognizer, U: LineRecordSpecRecognizer, V: Padder> {
    spec: &'a FileSpec,
    data_recognizer: T,
    line_recognizer: U,
    padder: V
}

impl<'a, T: DataRecordSpecRecognizer, U: LineRecordSpecRecognizer, V: Padder> FileWriter<'a, T, U, V> {
    pub fn new(spec: &'a FileSpec) -> FileWriter<'a, NoneRecognizer, NoneRecognizer, IdentityPadder> {
        FileWriter { spec: spec, data_recognizer: NoneRecognizer, line_recognizer: NoneRecognizer, padder: IdentityPadder }
    }

    pub fn new_with_recognizers_and_padder(spec: &'a FileSpec, data_recognizer: T, line_recognizer: U, padder: V) -> Self {
        FileWriter { spec: spec, data_recognizer: data_recognizer, line_recognizer: line_recognizer, padder: padder }
    }

    pub fn set_fields<W: MutableFile>(&'a mut self, file: &mut W, index: usize, data: &HashMap<String, String>, spec_name: Option<String>) -> Result<&'a mut Self, Error<W, V>> {
        let record_spec_name = try!(
            spec_name
                .or_else(|| self.data_recognizer.recognize_for_data(data, &self.spec.record_specs))
                .or_else(|| self.line_recognizer.recognize_for_line(file, index, &self.spec.record_specs))
                .ok_or(Error::RecordSpecNameRequired)
        );
        let record_spec = try!(self.spec.record_specs.get(&record_spec_name).ok_or(Error::RecordSpecNotFound(record_spec_name)));

        for (name, field_spec) in &record_spec.field_specs {
            if let Some(value) = data.get(name).or(field_spec.default.as_ref()) {
                let value = try!(self.padder.pad(value, field_spec.range.end - field_spec.range.start, &field_spec.padding, field_spec.padding_direction).map_err(Error::PaddingFailed));
                try!(file.set(
                    index,
                    field_spec.range.start,
                    &value
                ).map_err(Error::FailedToSetData));
            }
        }

        Ok(self)
    }

    pub fn set_field<W: MutableFile>(&'a mut self, file: &mut W, index: usize, key: String, value: String, spec_name: Option<String>) -> Result<&'a mut Self, Error<W, V>> {
        let mut data = HashMap::new();
        data.insert(key, value);
        self.set_fields(file, index, &data, spec_name)
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