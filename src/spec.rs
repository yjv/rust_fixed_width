extern crate pad;
use self::pad::{PadStr, Alignment};
use std::collections::HashMap;
use common::File;
use std::ops::Range;
use std::fmt::Debug;

#[derive(Debug, Eq, PartialEq)]
pub struct FileSpec {
    pub width: usize,
    pub record_specs: HashMap<String, RecordSpec>
}

impl SpecBuilder<FileSpec> for FileSpec {
    fn build(self) -> Self {
        self
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct RecordSpec {
    pub field_specs: HashMap<String, FieldSpec>
}

impl SpecBuilder<RecordSpec> for RecordSpec {
    fn build(self) -> Self {
        self
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct FieldSpec {
    pub range: Range<usize>,
    pub padding_direction: PaddingDirection,
    pub padding: String,
    pub default: Option<String>
}

impl SpecBuilder<FieldSpec> for FieldSpec {
    fn build(self) -> Self {
        self
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum PaddingDirection {
    Left,
    Right
}

pub trait Padder {
    type Error: Debug;
    fn pad(&self, data: &String, length: usize, padding: &String, direction: PaddingDirection) -> Result<String, Self::Error>;
}

impl<'a, T> Padder for &'a T where T: 'a + Padder {
    type Error = T::Error;
    fn pad(&self, data: &String, length: usize, padding: &String, direction: PaddingDirection) -> Result<String, Self::Error> {
        (**self).pad(data, length, padding, direction)
    }
}

pub trait UnPadder {
    type Error: Debug;
    fn unpad(&self, data: &String, padding: &String, direction: PaddingDirection) -> Result<String, Self::Error>;
}

impl<'a, T> UnPadder for &'a T where T: 'a + UnPadder {
    type Error = T::Error;
    fn unpad(&self, data: &String, padding: &String, direction: PaddingDirection) -> Result<String, Self::Error> {
        (**self).unpad(data, padding, direction)
    }
}

pub struct DefaultPadder;

#[derive(Debug, PartialEq, Eq)]
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
    fn pad(&self, data: &String, length: usize, padding: &String, direction: PaddingDirection) -> Result<String, Self::Error> {
        Ok(data.pad(
            length,
            try!(Self::get_char(padding)),
            match direction {
                PaddingDirection::Left => Alignment::Right,
                PaddingDirection::Right => Alignment::Left,
            },
            false
        ))
    }
}

impl UnPadder for DefaultPadder {
    type Error = PaddingError;
    fn unpad(&self, data: &String, padding: &String, direction: PaddingDirection) -> Result<String, Self::Error> {
        Ok(match direction {
            PaddingDirection::Left => data.trim_left_matches(try!(Self::get_char(padding))).to_string(),
            PaddingDirection::Right => data.trim_right_matches(try!(Self::get_char(padding))).to_string(),
        })
    }
}

pub struct IdentityPadder;

impl Padder for IdentityPadder {
    type Error = ();
    fn pad(&self, data: &String, _: usize, _: &String, _: PaddingDirection) -> Result<String, Self::Error> {
        Ok(data.clone())
    }
}

impl UnPadder for IdentityPadder {
    type Error = ();
    fn unpad(&self, data: &String, _: &String, _: PaddingDirection) -> Result<String, Self::Error> {
        Ok(data.clone())
    }
}

pub trait LineRecordSpecRecognizer {
    fn recognize_for_line<T: File>(&self, file: &T, index: usize, record_specs: &HashMap<String, RecordSpec>) -> Option<String>;
}

impl<'a, V> LineRecordSpecRecognizer for &'a V where V: 'a + LineRecordSpecRecognizer {
    fn recognize_for_line<T: File>(&self, file: &T, index: usize, record_specs: &HashMap<String, RecordSpec>) -> Option<String> {
        (**self).recognize_for_line(file, index, record_specs)
    }
}

pub trait DataRecordSpecRecognizer {
    fn recognize_for_data(&self, data: &HashMap<String, String>, record_specs: &HashMap<String, RecordSpec>) -> Option<String>;
}

impl<'a, U> DataRecordSpecRecognizer for &'a U where U: 'a + DataRecordSpecRecognizer {
    fn recognize_for_data(&self, data: &HashMap<String, String>, record_specs: &HashMap<String, RecordSpec>) -> Option<String> {
        (**self).recognize_for_data(data, record_specs)
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
    fn recognize_for_line<T: File>(&self, file: &T, index: usize, record_specs: &HashMap<String, RecordSpec>) -> Option<String> {
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
    fn recognize_for_data(&self, data: &HashMap<String, String>, record_specs: &HashMap<String, RecordSpec>) -> Option<String> {
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
    fn recognize_for_line<T: File>(&self, _: &T, _: usize, _: &HashMap<String, RecordSpec>) -> Option<String> {
        None
    }
}

impl DataRecordSpecRecognizer for NoneRecognizer {
    fn recognize_for_data(&self, _: &HashMap<String, String>, _: &HashMap<String, RecordSpec>) -> Option<String> {
        None
    }
}

pub trait SpecBuilder<T> {
    fn build(self) -> T;
}

pub struct FileSpecBuilder {
    width: Option<usize>,
    record_specs: HashMap<String, RecordSpec>
}

impl FileSpecBuilder {
    pub fn new() -> Self {
        FileSpecBuilder {
            width: None,
            record_specs: HashMap::new()
        }
    }

    pub fn with_record<U: Into<String>, V: SpecBuilder<RecordSpec>>(mut self, name: U, record: V) -> Self {
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

impl SpecBuilder<FileSpec> for FileSpecBuilder {
    fn build(self) -> FileSpec {
        FileSpec {
            width: self.width.expect("width must be set in order to build"),
            record_specs: self.record_specs
        }
    }
}

pub struct RecordSpecBuilder {
    field_specs: HashMap<String, FieldSpec>,
}

impl RecordSpecBuilder {
    pub fn new() -> Self {
        RecordSpecBuilder {
            field_specs: HashMap::new()
        }
    }

    pub fn with_field<U: Into<String>, V: SpecBuilder<FieldSpec>>(mut self, name: U, field: V) -> Self {
        self.field_specs.insert(name.into(), field.build());
        self
    }
}

impl SpecBuilder<RecordSpec> for RecordSpecBuilder {
    fn build(self) -> RecordSpec {
        RecordSpec {
            field_specs: self.field_specs
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct FieldSpecBuilder {
    range: Option<Range<usize>>,
    padding_direction: Option<PaddingDirection>,
    padding: Option<String>,
    default: Option<String>
}

impl FieldSpecBuilder {
    pub fn new() -> Self {
        FieldSpecBuilder {
            range: None,
            padding_direction: None,
            padding: None,
            default: None
        }
    }

    pub fn new_number() -> Self {
        FieldSpecBuilder {
            range: None,
            padding_direction: Some(PaddingDirection::Left),
            padding: Some("0".to_string()),
            default: None
        }
    }

    pub fn new_string() -> Self {
        FieldSpecBuilder {
            range: None,
            padding_direction: Some(PaddingDirection::Right),
            padding: Some(" ".to_string()),
            default: None
        }
    }

    pub fn with_range(self, range: Range<usize>) -> Self {
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

impl SpecBuilder<FieldSpec> for FieldSpecBuilder {
    fn build(self) -> FieldSpec {
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
    use super::super::test::{File as TestFile};

    #[test]
    fn none_recognizer() {
        let recognizer = NoneRecognizer;
        assert_eq!(None, recognizer.recognize_for_data(&HashMap::new(), &HashMap::new()));
        assert_eq!(None, recognizer.recognize_for_line(
            &TestFile {width: 10, line_seperator: "".to_string(), lines: vec![]},
            2,
            &HashMap::new()
        ));
    }

    #[test]
    fn id_spec_recognizer() {
        let specs = FileSpecBuilder::new()
            .with_width(10)
            .with_record(
                "record1",
                RecordSpecBuilder::new()
                    .with_field(
                        "field1",
                        FieldSpecBuilder::new()
                            .with_default("foo")
                            .with_range(0..3)
                            .with_padding("dsasd")
                            .with_padding_direction(PaddingDirection::Left)
                    )
                    .with_field(
                        "field2",
                        FieldSpecBuilder::new()
                            .with_range(4..9)
                            .with_padding("sdf".to_string())
                            .with_padding_direction(PaddingDirection::Right)
                    )
            )
            .with_record(
                "record2",
                RecordSpecBuilder::new()
                    .with_field(
                        "$id",
                        FieldSpecBuilder::new_string()
                            .with_default("bar")
                            .with_range(0..3)
                    )
                    .with_field(
                        "field2".to_string(),
                        FieldSpecBuilder::new_string()
                            .with_range(4..9)
                    )
            ).with_record(
                "record3",
                RecordSpecBuilder::new()
                    .with_field(
                        "field1",
                        FieldSpecBuilder::new_string()
                            .with_default("bar")
                            .with_range(0..3)
                    )
                    .with_field(
                        "field2",
                        FieldSpecBuilder::new_string()
                            .with_range(4..9)
                    )
            )
            .with_record(
                "record4",
                RecordSpecBuilder::new()
                    .with_field(
                        "$id",
                        FieldSpecBuilder::new_string()
                            .with_default("foo")
                            .with_range(0..3)
                    )
                    .with_field(
                        "field2".to_string(),
                        FieldSpecBuilder::new_string()
                            .with_range(4..9)
                    )
            )
            .build()
            .record_specs
        ;
        let recognizer = IdFieldRecognizer::new();
        let recognizer_with_field = IdFieldRecognizer::new_with_field("field1");
        let mut data = HashMap::new();

        data.insert("$id".to_string(), "bar".to_string());
        assert_eq!(Some("record2".to_string()), recognizer.recognize_for_data(&data, &specs));
        assert_eq!(None, recognizer_with_field.recognize_for_data(&data, &specs));

        data.insert("$id".to_string(), "foo".to_string());
        assert_eq!(Some("record4".to_string()), recognizer.recognize_for_data(&data, &specs));
        assert_eq!(None, recognizer_with_field.recognize_for_data(&data, &specs));

        data.insert("$id".to_string(), "foobar".to_string());
        assert_eq!(None, recognizer.recognize_for_data(&data, &specs));
        assert_eq!(None, recognizer_with_field.recognize_for_data(&data, &specs));

        data.insert("field1".to_string(), "bar".to_string());
        assert_eq!(None, recognizer.recognize_for_data(&data, &specs));
        assert_eq!(Some("record3".to_string()), recognizer_with_field.recognize_for_data(&data, &specs));
        data.remove(&"$id".to_string());

        assert_eq!(None, recognizer.recognize_for_data(&data, &specs));
        assert_eq!(Some("record3".to_string()), recognizer_with_field.recognize_for_data(&data, &specs));

        data.insert("field1".to_string(), "foo".to_string());
        assert_eq!(None, recognizer.recognize_for_data(&data, &specs));
        assert_eq!(Some("record1".to_string()), recognizer_with_field.recognize_for_data(&data, &specs));

        data.insert("field1".to_string(), "foobar".to_string());
        assert_eq!(None, recognizer.recognize_for_data(&data, &specs));
        assert_eq!(None, recognizer_with_field.recognize_for_data(&data, &specs));

        let mut file = TestFile {
            width: 10,
            line_seperator: String::new(),
            lines: vec![
                Ok("dsfdsfsdfd".to_string()),
                Ok("barasdasdd".to_string()),
                Ok("foodsfsdfd".to_string())
            ]
        };

        assert_eq!(None, recognizer.recognize_for_line(&file, 0, &specs));
        assert_eq!(None, recognizer_with_field.recognize_for_line(&file, 0, &specs));
        assert_eq!(Some("record2".to_string()), recognizer.recognize_for_line(&file, 1, &specs));
        assert_eq!(Some("record3".to_string()), recognizer_with_field.recognize_for_line(&file, 1, &specs));
        assert_eq!(Some("record4".to_string()), recognizer.recognize_for_line(&file, 2, &specs));
        assert_eq!(Some("record1".to_string()), recognizer_with_field.recognize_for_line(&file, 2, &specs));
        assert_eq!(FieldSpecBuilder {
            default: None,
            padding: Some("0".to_string()),
            padding_direction: Some(PaddingDirection::Left),
            range: None
        }, FieldSpecBuilder::new_number());
        assert_eq!(FieldSpecBuilder {
            default: None,
            padding: Some(" ".to_string()),
            padding_direction: Some(PaddingDirection::Right),
            range: None
        }, FieldSpecBuilder::new_string());
    }

    #[test]
    fn recognizer_reference() {
        let recognizer = NoneRecognizer;
        assert_eq!(None, DataRecordSpecRecognizer::recognize_for_data(&&recognizer, &HashMap::new(), &HashMap::new()));
        assert_eq!(None, LineRecordSpecRecognizer::recognize_for_line(
            &&recognizer,
            &TestFile {width: 10, line_seperator: "".to_string(), lines: vec![]},
            2,
            &HashMap::new()
        ));
    }

    #[test]
    fn build() {
        let spec = FileSpecBuilder::new()
            .with_width(10)
            .with_record(
                "record1",
                RecordSpecBuilder::new()
                    .with_field(
                        "field1".to_string(),
                        FieldSpecBuilder::new()
                            .with_range(0..4)
                            .with_padding("dsasd")
                            .with_padding_direction(PaddingDirection::Left)
                    )
                    .with_field(
                        "field2",
                        FieldSpecBuilder::new_string()
                            .with_range(5..9)
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
            padding: " ".to_string(),
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

    #[test]
    fn default_padder() {
        let padder = DefaultPadder;
        let data = "qwer".to_string();
        assert_eq!(Ok("qwer333333".to_string()), padder.pad(&data, 10, &"3".to_string(), PaddingDirection::Right));
        let data = "qwer".to_string();
        assert_eq!(Ok("333333qwer".to_string()), padder.pad(&data, 10, &"3".to_string(), PaddingDirection::Left));
        assert_eq!(Err(PaddingError::PaddingLongerThanOne), padder.pad(&data, 10, &"33".to_string(), PaddingDirection::Left));
        let data = "qwer333333".to_string();
        assert_eq!(Ok("qwer".to_string()), padder.unpad(&data, &"3".to_string(), PaddingDirection::Right));
        let data = "333333qwer".to_string();
        assert_eq!(Ok("qwer".to_string()), padder.unpad(&data, &"3".to_string(), PaddingDirection::Left));
        assert_eq!(Err(PaddingError::PaddingLongerThanOne), padder.unpad(&data, &"33".to_string(), PaddingDirection::Left));
    }

    #[test]
    fn identity_padder() {
        let padder = IdentityPadder;
        let data = "qwer".to_string();
        assert_eq!(Ok(data.clone()), padder.pad(&data, 10, &"3".to_string(), PaddingDirection::Right));
        assert_eq!(Ok(data.clone()), padder.pad(&data, 10, &"3".to_string(), PaddingDirection::Left));
        assert_eq!(Ok(data.clone()), padder.unpad(&data, &"3".to_string(), PaddingDirection::Right));
        assert_eq!(Ok(data.clone()), padder.unpad(&data, &"3".to_string(), PaddingDirection::Left));
    }

    #[test]
    fn padder_reference() {
        let padder = IdentityPadder;
        let data = "qwer".to_string();
        assert_eq!(Ok(data.clone()), Padder::pad(&&padder, &data, 10, &"3".to_string(), PaddingDirection::Right));
        assert_eq!(Ok(data.clone()), UnPadder::unpad(&&padder, &data, &"3".to_string(), PaddingDirection::Right));
    }
}
