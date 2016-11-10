use std::collections::HashMap;
use common::{Range, File};
use std::ops::Range as RangeStruct;
use std::fmt::Debug;

#[derive(Debug, Eq, PartialEq)]
pub struct FileSpec<T: Range = RangeStruct<usize>> {
    pub width: usize,
    pub record_specs: HashMap<String, RecordSpec<T>>
}

impl <T: Range> SpecBuilder<FileSpec<T>> for FileSpec<T> {
    fn build(self) -> Self {
        self
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct RecordSpec<T: Range = RangeStruct<usize>> {
    pub field_specs: HashMap<String, FieldSpec<T>>
}

impl <T: Range> SpecBuilder<RecordSpec<T>> for RecordSpec<T> {
    fn build(self) -> Self {
        self
    }
}

#[derive(Debug, Eq, PartialEq)]
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

#[derive(Debug, Eq, PartialEq)]
pub enum PaddingDirection {
    Left,
    Right
}

pub trait Padder {
    type Error: Debug;
    fn pad(&self, data: String, length: usize, padding: &String, direction: PaddingDirection) -> Result<String, Self::Error>;
}

impl<'a, T> Padder for &'a T where T: 'a + Padder {
    type Error = T::Error;
    fn pad(&self, data: String, length: usize, padding: &String, direction: PaddingDirection) -> Result<String, Self::Error> {
        (**self).pad(data, length, padding, direction)
    }
}

pub trait UnPadder {
    type Error: Debug;
    fn unpad(&self, data: String, padding: &String, direction: PaddingDirection) -> Result<String, Self::Error>;
}

extern crate pad;
use self::pad::{PadStr, Alignment};

pub struct DefaultPadder;

#[derive(Debug)]
pub enum PaddingError {
    PaddingLongerThanOne
}

impl DefaultPadder {
    fn get_char(padding: &String) -> Result<char, PaddingError> {
        if padding.len() > 1 {
            Err(PaddingError::PaddingLongerThanOne)
        } else {
            Ok(padding.chars().next().or(Some(' ')).expect("should have a some no matter what"))
        }
    }
}

impl Padder for DefaultPadder {
    type Error = PaddingError;
    fn pad(&self, data: String, length: usize, padding: &String, direction: PaddingDirection) -> Result<String, Self::Error> {
        Ok(data.pad(
            length,
            try!(Self::get_char(padding)),
            match direction {
                PaddingDirection::Left => Alignment::Left,
                PaddingDirection::Right => Alignment::Right,
            },
            false
        ))
    }
}

impl UnPadder for DefaultPadder {
    type Error = PaddingError;
    fn unpad(&self, data: String, padding: &String, direction: PaddingDirection) -> Result<String, Self::Error> {
        Ok(match direction {
            PaddingDirection::Left => data.trim_left_matches(try!(Self::get_char(padding))).to_string(),
            PaddingDirection::Right => data.trim_right_matches(try!(Self::get_char(padding))).to_string(),
        })
    }
}

pub struct IdentityPadder;

impl Padder for IdentityPadder {
    type Error = ();
    fn pad(&self, data: String, _: usize, _: &String, _: PaddingDirection) -> Result<String, Self::Error> {
        Ok(data)
    }
}

impl UnPadder for IdentityPadder {
    type Error = ();
    fn unpad(&self, data: String, _: &String, _: PaddingDirection) -> Result<String, Self::Error> {
        Ok(data)
    }
}

pub trait LineRecordSpecRecognizer {
    fn recognize_for_line<T: File, U: Range>(&self, file: &T, index: usize, record_specs: &HashMap<String, RecordSpec<U>>) -> Option<String>;
}

impl<'a, V> LineRecordSpecRecognizer for &'a V where V: 'a + LineRecordSpecRecognizer {
    fn recognize_for_line<T: File, U: Range>(&self, file: &T, index: usize, record_specs: &HashMap<String, RecordSpec<U>>) -> Option<String> {
        (*self).recognize_for_line(file, index, record_specs)
    }
}

pub trait DataRecordSpecRecognizer {
    fn recognize_for_data<T: Range>(&self, data: &HashMap<String, String>, record_specs: &HashMap<String, RecordSpec<T>>) -> Option<String>;
}

impl<'a, U> DataRecordSpecRecognizer for &'a U where U: 'a + DataRecordSpecRecognizer {
    fn recognize_for_data<T: Range>(&self, data: &HashMap<String, String>, record_specs: &HashMap<String, RecordSpec<T>>) -> Option<String> {
        (*self).recognize_for_data(data, record_specs)
    }
}

pub struct IdFieldRecognizer {
    id_field: String
}

impl IdFieldRecognizer {
    pub fn new() -> Self {
        Self::new_with_field("$id")
    }

    pub fn new_with_field<T: Into<String>>(id_field: T) -> Self {
        IdFieldRecognizer { id_field: id_field.into() }
    }
}

impl LineRecordSpecRecognizer for IdFieldRecognizer {
    fn recognize_for_line<T: File, U: Range>(&self, file: &T, index: usize, record_specs: &HashMap<String, RecordSpec<U>>) -> Option<String> {
        for (name, record_spec) in record_specs.iter() {
            if let Some(ref field_spec) = record_spec.field_specs.get(&self.id_field) {
                if let Some(ref default) = field_spec.default {
                    if let Ok(string) = file.get(index, field_spec.range.clone()) {
                        if &string == default {
                            return Some(name.clone());
                        }
                    }
                }
            }
        }

        None
    }
}

impl DataRecordSpecRecognizer for IdFieldRecognizer {
    fn recognize_for_data<T: Range>(&self, data: &HashMap<String, String>, record_specs: &HashMap<String, RecordSpec<T>>) -> Option<String> {
        for (name, record_spec) in record_specs.iter() {
            if let Some(ref field_spec) = record_spec.field_specs.get(&self.id_field) {
                if let Some(ref default) = field_spec.default {
                    if let Some(string) = data.get(&self.id_field) {
                        if string == default {
                            return Some(name.clone());
                        }
                    }
                }
            }
        }

        None
    }
}

pub struct NoneRecognizer;

impl LineRecordSpecRecognizer for NoneRecognizer {
    fn recognize_for_line<T: File, U: Range>(&self, _: &T, _: usize, _: &HashMap<String, RecordSpec<U>>) -> Option<String> {
        None
    }
}

impl DataRecordSpecRecognizer for NoneRecognizer {
    fn recognize_for_data<T: Range>(&self, _: &HashMap<String, String>, _: &HashMap<String, RecordSpec<T>>) -> Option<String> {
        None
    }
}

pub trait SpecBuilder<T> {
    fn build(self) -> T;
}

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

    pub fn with_record<U: Into<String>, V: SpecBuilder<RecordSpec<T>>>(mut self, name: U, record: V) -> Self {
        self.record_specs.insert(name.into(), record.build());
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

    pub fn with_field<U: Into<String>, V: SpecBuilder<FieldSpec<T>>>(mut self, name: U, field: V) -> Self {
        self.field_specs.insert(name.into(), field.build());
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

    pub fn with_padding<U: Into<String>>(self, padding: U) -> Self {
        FieldSpecBuilder {
            range: self.range,
            padding_direction: self.padding_direction,
            padding: Some(padding.into()),
            default: self.default
        }
    }

    pub fn with_default<U: Into<String>>(self, default: U) -> Self {
        FieldSpecBuilder {
            range: self.range,
            padding_direction: self.padding_direction,
            padding: self.padding,
            default: Some(default.into())
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
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn id_spec_recognizer() {
        let mut record_specs: HashMap<String, RecordSpec> = HashMap::new();
        record_specs.insert("record1".to_string(), RecordSpec {
            field_specs: HashMap::new()
        });
    }

    #[test]
    fn build() {
        let spec = FileSpecBuilder::new()
            .with_width(10)
            .with_record(
                "record1".to_string(),
                RecordSpecBuilder::new()
                    .with_field(
                        "field1".to_string(),
                        FieldSpecBuilder::new()
                            .with_range(0..4)
                            .with_padding("dsasd")
                            .with_padding_direction(PaddingDirection::Left)
                    )
                    .with_field(
                        "field2".to_string(),
                        FieldSpecBuilder::new()
                            .with_range(5..9)
                            .with_padding("sdf".to_string())
                            .with_padding_direction(PaddingDirection::Right)
                            .with_default("def")
                    )
                    .with_field(
                        "field3".to_string(),
                        FieldSpecBuilder::new()
                            .with_range(10..45)
                            .with_padding("xcvcxv".to_string())
                            .with_padding_direction(PaddingDirection::Right)
                    )
            )
            .with_record(
                "record2".to_string(),
                RecordSpecBuilder::new()
                    .with_field(
                        "field1".to_string(),
                        FieldSpecBuilder::new()
                            .with_range((0..3))
                            .with_padding("dsasd".to_string())
                            .with_padding_direction(PaddingDirection::Left)
                    )
                    .with_field(
                        "field2".to_string(),
                        FieldSpecBuilder::new()
                            .with_range((4..8))
                            .with_padding("sdf".to_string())
                            .with_padding_direction(PaddingDirection::Right)
                    )
                    .with_field(
                        "field3",
                        FieldSpecBuilder::new()
                            .with_range((9..36))
                            .with_padding("xcvcxv".to_string())
                            .with_padding_direction(PaddingDirection::Right)
                    )
                    .with_field(
                        "field4".to_string(),
                        FieldSpec {
                            range: (37..45),
                            padding: "sdfsd".to_string(),
                            padding_direction: PaddingDirection::Left,
                            default: None
                        }
                    )
            )
            .with_record("record3".to_string(), RecordSpec {
                field_specs: HashMap::new()
            })
            .build()
        ;
        let mut record_specs = HashMap::new();
        let mut field_specs = HashMap::new();
        field_specs.insert("field1".to_string(), FieldSpec {
            range: (0..4),
            padding: "dsasd".to_string(),
            padding_direction: PaddingDirection::Left,
            default: None
        });
        field_specs.insert("field2".to_string(), FieldSpec {
            range: (5..9),
            padding: "sdf".to_string(),
            padding_direction: PaddingDirection::Right,
            default: Some("def".to_string())
        });
        field_specs.insert("field3".to_string(), FieldSpec {
            range: (10..45),
            padding: "xcvcxv".to_string(),
            padding_direction: PaddingDirection::Right,
            default: None
        });
        record_specs.insert("record1".to_string(), RecordSpec {
            field_specs: field_specs
        });
        let mut field_specs = HashMap::new();
        field_specs.insert("field1".to_string(), FieldSpec {
            range: (0..3),
            padding: "dsasd".to_string(),
            padding_direction: PaddingDirection::Left,
            default: None
        });
        field_specs.insert("field2".to_string(), FieldSpec {
            range: (4..8),
            padding: "sdf".to_string(),
            padding_direction: PaddingDirection::Right,
            default: None
        });
        field_specs.insert("field3".to_string(), FieldSpec {
            range: (9..36),
            padding: "xcvcxv".to_string(),
            padding_direction: PaddingDirection::Right,
            default: None
        });
        field_specs.insert("field4".to_string(), FieldSpec {
            range: (37..45),
            padding: "sdfsd".to_string(),
            padding_direction: PaddingDirection::Left,
            default: None
        });
        record_specs.insert("record2".to_string(), RecordSpec {
            field_specs: field_specs
        });
        record_specs.insert("record3".to_string(), RecordSpec {
            field_specs: HashMap::new()
        });
        assert_eq!(FileSpec {
            width: 10,
            record_specs: record_specs
        }, spec);
    }
}
