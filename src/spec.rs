use std::collections::HashMap;
use common::{Range, Line};
use std::ops::Range as RangeStruct;


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

pub trait RecordSpecRecognizer {
    fn recognize<T: Line, U: Range>(&self, line: &T, file_spec: &FileSpec<U>) -> Result<String, String>;
}

pub struct SpecFieldRecognizer {
    field: String
}

impl SpecFieldRecognizer {
    pub fn new() -> Self {
        Self::new_with_field("$id".to_string())
    }

    pub fn new_with_field(field: String) -> Self {
        SpecFieldRecognizer { field: field }
    }
}

impl RecordSpecRecognizer for SpecFieldRecognizer {
    fn recognize<T: Line, U: Range>(&self, line: &T, file_spec: &FileSpec<U>) -> Result<String, String> {
        for (name, record_spec) in file_spec.record_specs.iter() {
            if let Some(ref field_spec) = record_spec.field_specs.get(&self.field) {
                if let Some(ref default) = field_spec.default {
                    if let Ok(string) = line.get(field_spec.range.clone()) {
                        if &string == default {
                            return Ok(name.clone());
                        }
                    }
                }
            }
        }

        Err(format!("no record spec has a field {} matching the value of that field in the line. Either the field value is wrong or a record spec is missing the $id field spec.", self.field))
    }
}