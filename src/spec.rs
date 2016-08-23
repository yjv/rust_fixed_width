use std::collections::HashMap;
use common::{Range, Line};
use std::ops::Range as RangeStruct;
use std::fmt::Debug;

pub struct FileSpec<T: Range = RangeStruct<usize>> {
    pub name: String,
    pub width: usize,
    pub line_seperator: String,
    pub record_specs: HashMap<String, RecordSpec<T>>
}

pub struct RecordSpec<T: Range = RangeStruct<usize>> {
    pub name: String,
    pub field_specs: HashMap<String, FieldSpec<T>>
}

pub struct FieldSpec<T: Range = RangeStruct<usize>> {
    pub name: String,
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
    fn recognize_for_line<T: Line, U: Range>(&self, line: &T, file_spec: &FileSpec<U>) -> Result<String, Self::Error>;
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
    fn recognize_for_line<T: Line, U: Range>(&self, line: &T, file_spec: &FileSpec<U>) -> Result<String, Self::Error> {
        for (name, record_spec) in file_spec.record_specs.iter() {
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