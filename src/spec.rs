use std::collections::HashMap;
use common::{Range, Line};
use std::ops::Range as RangeStruct;
use std::fmt::Debug;

pub struct FileSpec<T: Range = RangeStruct<usize>> {
    pub width: usize,
    pub record_specs: HashMap<String, RecordSpec<T>>
}

impl <T: Range> SpecBuilder<FileSpec<T>> for FileSpec<T> {
    fn build(self) -> Self {
        self
    }
}

#[derive(Clone)]
pub struct RecordSpec<T: Range = RangeStruct<usize>> {
    pub field_specs: HashMap<String, FieldSpec<T>>
}

impl <T: Range> SpecBuilder<RecordSpec<T>> for RecordSpec<T> {
    fn build(self) -> Self {
        self
    }
}

#[derive(Clone)]
pub struct FieldSpec<T: Range = RangeStruct<usize>> {
    pub range: T,
    pub padding_direction: PaddingDirection,
    pub padding: String,
    pub default: Option<String>
}

impl <T: Range> SpecBuilder<FieldSpec<T>> for FieldSpec<T> {
    fn build(self) -> Self {
        self
    }
}

#[derive(Clone)]
pub enum PaddingDirection {
    Left,
    Right
}

pub trait LineRecordSpecRecognizer {
    type Error: Debug;
    fn recognize_for_line<T: Line, U: Range>(&self, line: &T, record_specs: &HashMap<String, RecordSpec<U>>) -> Result<String, Self::Error>;
}

impl<'a, V> LineRecordSpecRecognizer for &'a V where V: 'a + LineRecordSpecRecognizer {
    type Error = V::Error;
    fn recognize_for_line<T: Line, U: Range>(&self, line: &T, record_specs: &HashMap<String, RecordSpec<U>>) -> Result<String, Self::Error> {
        (*self).recognize_for_line(line, record_specs)
    }
}

pub trait DataRecordSpecRecognizer {
    type Error: Debug;
    fn recognize_for_data<T: AsRef<HashMap<String, String>>, U: Range>(&self, data: T, record_specs: &HashMap<String, RecordSpec<U>>) -> Result<String, Self::Error>;
}

impl<'a, V> DataRecordSpecRecognizer for &'a V where V: 'a + DataRecordSpecRecognizer {
    type Error = V::Error;
    fn recognize_for_data<T: AsRef<HashMap<String, String>>, U: Range>(&self, data: T, record_specs: &HashMap<String, RecordSpec<U>>) -> Result<String, Self::Error> {
        (*self).recognize_for_data(data, record_specs)
    }
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
    fn recognize_for_line<T: Line, U: Range>(&self, _: &T, _: &HashMap<String, RecordSpec<U>>) -> Result<String, Self::Error> {
        Err(ErrorFieldRecognizerError::NoRecordSpecFound)
    }
}

impl DataRecordSpecRecognizer for ErrorFieldRecognizer {
    type Error = ErrorFieldRecognizerError;
    fn recognize_for_data<T: AsRef<HashMap<String, String>>, U: Range>(&self, _: T, _: &HashMap<String, RecordSpec<U>>) -> Result<String, Self::Error> {
        Err(ErrorFieldRecognizerError::NoRecordSpecFound)
    }
}

pub trait SpecBuilder<T> {
    fn build(self) -> T;
}

#[derive(Clone)]
pub struct FileSpecBuilder<T: Range = RangeStruct<usize>> {
    width: Option<usize>,
    record_specs: HashMap<String, RecordSpec<T>>
}

impl <T: Range> FileSpecBuilder<T> {
    pub fn new() -> Self {
        FileSpecBuilder {
            width: None,
            record_specs: HashMap::new()
        }
    }

    pub fn with_record<U: SpecBuilder<RecordSpec<T>>>(mut self, name: String, record: U) -> Self {
        self.record_specs.insert(name, record.build());
        self
    }

    pub fn with_width(self, width: usize) -> Self {
        FileSpecBuilder {
            width: Some(width),
            record_specs: self.record_specs
        }
    }
}

impl <T: Range> SpecBuilder<FileSpec<T>> for FileSpecBuilder<T> {
    fn build(self) -> FileSpec<T> {
        FileSpec {
            width: self.width.expect("width must be set in order to build"),
            record_specs: self.record_specs
        }
    }
}

pub struct RecordSpecBuilder<T: Range = RangeStruct<usize>> {
    field_specs: HashMap<String, FieldSpec<T>>,
}

impl <T: Range> RecordSpecBuilder<T> {
    pub fn new() -> Self {
        RecordSpecBuilder {
            field_specs: HashMap::new()
        }
    }

    pub fn with_field<U: SpecBuilder<FieldSpec<T>>>(mut self, name: String, field: U) -> Self {
        self.field_specs.insert(name, field.build());
        self
    }
}

impl <T: Range> SpecBuilder<RecordSpec<T>> for RecordSpecBuilder<T> {
    fn build(self) -> RecordSpec<T> {
        RecordSpec {
            field_specs: self.field_specs
        }
    }
}

pub struct FieldSpecBuilder<T: Range = RangeStruct<usize>> {
    range: Option<T>,
    padding_direction: Option<PaddingDirection>,
    padding: Option<String>,
    default: Option<String>
}

impl <T: Range> FieldSpecBuilder<T> {
    pub fn new() -> Self {
        FieldSpecBuilder {
            range: None,
            padding_direction: None,
            padding: None,
            default: None
        }
    }

    pub fn with_range(self, range: T) -> Self {
        FieldSpecBuilder {
            range: Some(range),
            padding_direction: self.padding_direction,
            padding: self.padding,
            default: self.default
        }
    }

    pub fn with_padding_direction(self, padding_direction: PaddingDirection) -> Self {
        FieldSpecBuilder {
            range: self.range,
            padding_direction: Some(padding_direction),
            padding: self.padding,
            default: self.default
        }
    }

    pub fn with_padding(self, padding: String) -> Self {
        FieldSpecBuilder {
            range: self.range,
            padding_direction: self.padding_direction,
            padding: Some(padding),
            default: self.default
        }
    }

    pub fn with_default(self, default: String) -> Self {
        FieldSpecBuilder {
            range: self.range,
            padding_direction: self.padding_direction,
            padding: self.padding,
            default: Some(default)
        }
    }
}

impl <T: Range> SpecBuilder<FieldSpec<T>> for FieldSpecBuilder<T> {
    fn build(self) -> FieldSpec<T> {
        FieldSpec {
            range: self.range.expect("range must be set in order to build"),
            padding_direction: self.padding_direction.expect("padding direction must be set in order to build"),
            padding: self.padding.expect("padding must be set in order to build"),
            default: self.default
        }
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