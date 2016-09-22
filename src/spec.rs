use std::collections::HashMap;
use common::{Range, Line};
use std::ops::Range as RangeStruct;
use std::fmt::Debug;

pub struct FileSpec<T: Range = RangeStruct<usize>> {
    pub width: usize,
    pub record_specs: HashMap<String, RecordSpec<T>>
}

pub struct RecordSpec<T: Range = RangeStruct<usize>> {
    pub field_specs: HashMap<String, FieldSpec<T>>
}

pub struct FieldSpec<T: Range = RangeStruct<usize>> {
    pub range: T,
    pub padding_direction: PaddingDirection,
    pub padding_char: char,
    pub default: Option<String>
}

pub enum PaddingDirection {
    Left,
    Right
}

pub trait LineRecordSpecRecognizer {
    type Error: Debug;
    fn recognize_for_line<T: Line, U: Range>(&self, line: &T, record_specs: &HashMap<String, RecordSpec<U>>) -> Result<String, Self::Error>;
}

pub trait DataRecordSpecRecognizer {
    type Error: Debug;
    fn recognize_for_data<T: AsRef<HashMap<String, String>>, U: Range>(&self, data: T, record_specs: &HashMap<String, RecordSpec<U>>) -> Result<String, Self::Error>;
}

pub struct IdFieldRecognizer {
    id_field: String
}

impl IdFieldRecognizer {
    pub fn new() -> Self {
        Self::new_with_field("$id".to_string())
    }

    pub fn new_with_field(id_field: String) -> Self {
        IdFieldRecognizer { id_field: id_field }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum IdFieldRecognizerError {
    NoRecordSpecMatchingIdField(String)
}

impl LineRecordSpecRecognizer for IdFieldRecognizer {
    type Error = IdFieldRecognizerError;
    fn recognize_for_line<T: Line, U: Range>(&self, line: &T, record_specs: &HashMap<String, RecordSpec<U>>) -> Result<String, Self::Error> {
        for (name, record_spec) in record_specs.iter() {
            if let Some(ref field_spec) = record_spec.field_specs.get(&self.id_field) {
                if let Some(ref default) = field_spec.default {
                    if let Ok(string) = line.get(field_spec.range.clone()) {
                        if &string == default {
                            return Ok(name.clone());
                        }
                    }
                }
            }
        }

        Err(IdFieldRecognizerError::NoRecordSpecMatchingIdField(self.id_field.clone()))
    }
}

impl DataRecordSpecRecognizer for IdFieldRecognizer {
    type Error = IdFieldRecognizerError;
    fn recognize_for_data<T: AsRef<HashMap<String, String>>, U: Range>(&self, data: T, record_specs: &HashMap<String, RecordSpec<U>>) -> Result<String, Self::Error> {
        for (name, record_spec) in record_specs.iter() {
            if let Some(ref field_spec) = record_spec.field_specs.get(&self.id_field) {
                if let Some(ref default) = field_spec.default {
                    if let Some(string) = data.as_ref().get(&self.id_field) {
                        if string == default {
                            return Ok(name.clone());
                        }
                    }
                }
            }
        }

        Err(IdFieldRecognizerError::NoRecordSpecMatchingIdField(self.id_field.clone()))
    }
}

pub struct ErrorFieldRecognizer;

#[derive(Debug, Eq, PartialEq)]
pub enum ErrorFieldRecognizerError {
    NoRecordSpecFound
}

impl LineRecordSpecRecognizer for ErrorFieldRecognizer {
    type Error = ErrorFieldRecognizerError;
    fn recognize_for_line<T: Line, U: Range>(&self, line: &T, record_specs: &HashMap<String, RecordSpec<U>>) -> Result<String, Self::Error> {
        Err(ErrorFieldRecognizerError::NoRecordSpecFound)
    }
}

impl DataRecordSpecRecognizer for ErrorFieldRecognizer {
    type Error = ErrorFieldRecognizerError;
    fn recognize_for_data<T: AsRef<HashMap<String, String>>, U: Range>(&self, data: T, record_specs: &HashMap<String, RecordSpec<U>>) -> Result<String, Self::Error> {
        Err(ErrorFieldRecognizerError::NoRecordSpecFound)
    }
}

#[cfg(test)]
mod test {
    use super::{IdFieldRecognizer, DataRecordSpecRecognizer, LineRecordSpecRecognizer, RecordSpec, FieldSpec};
    use std::collections::HashMap;

    #[test]
    fn id_spec_recognizer() {
        let mut record_specs: HashMap<String, RecordSpec> = HashMap::new();
        record_specs.insert("record1".to_string(), RecordSpec {
            field_specs: HashMap::new()
        });
    }
}