use std::collections::{HashMap, BTreeMap};
use std::ops::Range;
use std::iter::repeat;

pub trait Builder<T> {
    fn build(self) -> T;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Spec {
    pub record_specs: HashMap<String, RecordSpec>
}

impl Builder<Spec> for Spec {
    fn build(self) -> Self {
        self
    }
}

#[derive(Clone)]
pub struct SpecBuilder {
    record_specs: HashMap<String, RecordSpec>
}

impl SpecBuilder {
    pub fn new() -> Self {
        SpecBuilder {
            record_specs: HashMap::new()
        }
    }

    pub fn with_record<T: Into<String>, U: Builder<RecordSpec>>(mut self, name: T, record: U) -> Self {
        self.record_specs.insert(name.into(), record.build());
        self
    }
}

impl Builder<Spec> for SpecBuilder {
    fn build(self) -> Spec {
        Spec {
            record_specs: self.record_specs
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordSpec {
    pub line_ending: Vec<u8>,
    pub field_specs: BTreeMap<String, FieldSpec>
}

impl RecordSpec {
    pub fn field_range<'a>(&self, name: &'a str) -> Option<Range<usize>> {
        let mut found_field_spec = None;
        let index = self.field_specs.iter().take_while(|&(field_name, field_spec)| {
            if field_name == name {
                found_field_spec = Some(field_spec);
            }
            found_field_spec.is_none()
        }).fold(0, |length, (_, field_spec)| length + field_spec.length);

        found_field_spec.map(|field_spec| index..index + field_spec.length)
    }

    pub fn len(&self) -> usize {
        self.field_specs.iter().fold(0, |length, (_, field_spec)| length + field_spec.length)
    }
}

impl Builder<RecordSpec> for RecordSpec {
    fn build(self) -> Self {
        self
    }
}

#[derive(Clone)]
pub struct RecordSpecBuilder {
    line_ending: Vec<u8>,
    field_specs: BTreeMap<String, FieldSpec>,
}

impl RecordSpecBuilder {
    pub fn new() -> Self {
        RecordSpecBuilder {
            line_ending: Vec::new(),
            field_specs: BTreeMap::new()
        }
    }

    pub fn with_field<T: Into<String>, U: Builder<FieldSpec>>(mut self, name: T, field: U) -> Self {
        self.field_specs.insert(name.into(), field.build());
        self
    }

    pub fn with_line_ending<T: Into<Vec<u8>>>(mut self, line_ending: T) -> Self {
        self.line_ending = line_ending.into();
        self
    }
}

impl Builder<RecordSpec> for RecordSpecBuilder {
    fn build(self) -> RecordSpec {
        RecordSpec {
            line_ending: self.line_ending,
            field_specs: self.field_specs
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum PaddingDirection {
    Left,
    Right
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldSpec {
    pub length: usize,
    pub padding_direction: PaddingDirection,
    pub padding: Vec<u8>,
    pub default: Option<Vec<u8>>,
    pub filler: bool
}

impl Builder<FieldSpec> for FieldSpec {
    fn build(self) -> Self {
        self
    }
}

#[derive(Clone)]
pub struct FieldSpecBuilder {
    length: Option<usize>,
    padding_direction: Option<PaddingDirection>,
    padding: Option<Vec<u8>>,
    default: Option<Vec<u8>>,
    filler: bool
}

impl FieldSpecBuilder {
    pub fn new() -> Self {
        FieldSpecBuilder {
            length: None,
            padding_direction: None,
            padding: None,
            default: None,
            filler: false
        }
    }

    pub fn new_number() -> Self {
        Self::new().with_padding("0").with_padding_direction(PaddingDirection::Left)
    }

    pub fn new_empty_number() -> Self {
        Self::new_number().with_default("0")
    }

    pub fn new_string() -> Self {
        Self::new().with_padding(" ").with_padding_direction(PaddingDirection::Right)
    }

    pub fn new_empty_string() -> Self {
        Self::new_string().with_default("")
    }

    pub fn new_filler(length: usize) -> Self {
        Self::new_string()
            .with_default(repeat(" ").take(length).collect::<String>())
            .with_length(length)
            .make_filler()
    }

    pub fn with_length(mut self, length: usize) -> Self {
        self.length = Some(length);
        self
    }

    pub fn with_padding_direction(mut self, padding_direction: PaddingDirection) -> Self {
        self.padding_direction = Some(padding_direction);
        self
    }

    pub fn with_padding<T: Into<Vec<u8>>>(mut self, padding: T) -> Self {
        self.padding = Some(padding.into());
        self
    }

    pub fn with_default<T: Into<Vec<u8>>>(mut self, default: T) -> Self {
        self.default = Some(default.into());
        self
    }

    pub fn make_filler(mut self) -> Self {
        self.filler = true;
        self
    }
}

impl Builder<FieldSpec> for FieldSpecBuilder {
    fn build(self) -> FieldSpec {
        FieldSpec {
            length: self.length.expect("length must be set in order to build"),
            padding_direction: self.padding_direction.expect("padding direction must be set in order to build"),
            padding: self.padding.expect("padding must be set in order to build"),
            default: self.default,
            filler: self.filler
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::{HashMap, BTreeMap};
    use super::super::test::test_spec;

    #[test]
    fn build() {
        let spec = test_spec();
        let mut record_specs = HashMap::new();
        let mut field_specs = BTreeMap::new();
        field_specs.insert("field1".to_string(), FieldSpec {
            length: 4,
            padding: "dsasd".as_bytes().to_owned(),
            padding_direction: PaddingDirection::Left,
            default: None,
            filler: true
        });
        field_specs.insert("field2".to_string(), FieldSpec {
            length: 5,
            padding: " ".as_bytes().to_owned(),
            padding_direction: PaddingDirection::Right,
            default: Some("def".as_bytes().to_owned()),
            filler: false
        });
        field_specs.insert("field3".to_string(), FieldSpec {
            length: 36,
            padding: "xcvcxv".as_bytes().to_owned(),
            padding_direction: PaddingDirection::Right,
            default: None,
            filler: false
        });
        record_specs.insert("record1".to_string(), RecordSpec {
            line_ending: "\n".as_bytes().to_owned(),
            field_specs: field_specs
        });
        let mut field_specs = BTreeMap::new();
        field_specs.insert("field1".to_string(), FieldSpec {
            length: 3,
            padding: "dsasd".as_bytes().to_owned(),
            padding_direction: PaddingDirection::Left,
            default: None,
            filler: false
        });
        field_specs.insert("field2".to_string(), FieldSpec {
            length: 4,
            padding: "sdf".as_bytes().to_owned(),
            padding_direction: PaddingDirection::Right,
            default: Some("defa".as_bytes().to_owned()),
            filler: false
        });
        field_specs.insert("field3".to_string(), FieldSpec {
            length: 27,
            padding: "xcvcxv".as_bytes().to_owned(),
            padding_direction: PaddingDirection::Right,
            default: None,
            filler: false
        });
        field_specs.insert("field4".to_string(), FieldSpec {
            length: 8,
            padding: "sdfsd".as_bytes().to_owned(),
            padding_direction: PaddingDirection::Left,
            default: None,
            filler: false
        });
        record_specs.insert("record2".to_string(), RecordSpec {
            line_ending: "\n".as_bytes().to_owned(),
            field_specs: field_specs
        });
        record_specs.insert("record3".to_string(), RecordSpec {
            line_ending: "\n".as_bytes().to_owned(),
            field_specs: BTreeMap::new()
        });
        assert_eq!(Spec {
            record_specs: record_specs
        }, spec);
        assert_eq!(FieldSpecBuilder::new()
            .with_padding("0".to_string())
            .with_padding_direction(PaddingDirection::Left)
            .with_length(0)
            .build()
        , FieldSpecBuilder::new_number().with_length(0).build());
        assert_eq!(FieldSpecBuilder::new()
            .with_padding(" ".as_bytes().to_owned())
            .with_padding_direction(PaddingDirection::Right)
            .with_length(0)
            .build()
        , FieldSpecBuilder::new_string().with_length(0).build());
    }

    #[test]
    fn field_range() {
        let spec = test_spec();
        let record_spec = spec.record_specs.get("record1").unwrap();
        assert_eq!(Some(0..4), record_spec.field_range("field1"));
        assert_eq!(Some(4..9), record_spec.field_range(&"field2".to_string()));
        assert_eq!(Some(9..45), record_spec.field_range(&"field3".to_string()));
        assert_eq!(None, record_spec.field_range(&"field4".to_string()));
    }

    #[test]
    fn len() {
        let spec = test_spec();
        assert_eq!(45, spec.record_specs.get("record1").unwrap().len());
        assert_eq!(42, spec.record_specs.get("record2").unwrap().len());
        assert_eq!(0, spec.record_specs.get("record3").unwrap().len());
    }
}
