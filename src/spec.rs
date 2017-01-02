extern crate pad;
use std::collections::{HashMap, BTreeMap};
use std::ops::Range;

#[derive(Debug, Eq, PartialEq)]
pub struct FileSpec {
    pub line_spec: LineSpec,
    pub record_specs: HashMap<String, RecordSpec>
}

impl SpecBuilder<FileSpec> for FileSpec {
    fn build(self) -> Self {
        self
    }
}

pub struct FileSpecBuilder {
    line_spec: Option<LineSpec>,
    record_specs: HashMap<String, RecordSpec>
}

impl FileSpecBuilder {
    pub fn new() -> Self {
        FileSpecBuilder {
            line_spec: None,
            record_specs: HashMap::new()
        }
    }

    pub fn with_record<T: Into<String>, U: SpecBuilder<RecordSpec>>(mut self, name: T, record: U) -> Self {
        self.record_specs.insert(name.into(), record.build());
        self
    }

    pub fn with_line_spec<T: SpecBuilder<LineSpec>>(self, line_spec: T) -> Self {
        FileSpecBuilder {
            line_spec: Some(line_spec.build()),
            record_specs: self.record_specs
        }
    }
}

impl SpecBuilder<FileSpec> for FileSpecBuilder {
    fn build(self) -> FileSpec {
        FileSpec {
            line_spec: self.line_spec.expect("line spec must be set in order to build"),
            record_specs: self.record_specs
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct LineSpec {
    pub length: usize,
    pub separator: String,
}

impl SpecBuilder<LineSpec> for LineSpec {
    fn build(self) -> LineSpec {
        self
    }
}

pub struct LineSpecBuilder {
    length: Option<usize>,
    separator: Option<String>
}

impl LineSpecBuilder {
    pub fn new() -> Self {
        LineSpecBuilder {
            length: None,
            separator: None
        }
    }
    pub fn with_length(self, length: usize) -> Self {
        LineSpecBuilder {
            length: Some(length),
            separator: self.separator
        }
    }

    pub fn with_separator<T: Into<String>>(self, separator: T) -> Self {
        LineSpecBuilder {
            length: self.length,
            separator: Some(separator.into())
        }
    }
}

impl SpecBuilder<LineSpec> for LineSpecBuilder {
    fn build(self) -> LineSpec {
        LineSpec {
            length: self.length.expect("length is required to create the line spec"),
            separator: self.separator.unwrap_or_default()
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct RecordSpec {
    pub field_specs: BTreeMap<String, FieldSpec>
}

impl SpecBuilder<RecordSpec> for RecordSpec {
    fn build(self) -> Self {
        self
    }
}

pub struct RecordSpecBuilder {
    field_specs: BTreeMap<String, FieldSpec>,
}

impl RecordSpecBuilder {
    pub fn new() -> Self {
        RecordSpecBuilder {
            field_specs: BTreeMap::new()
        }
    }

    pub fn with_field<T: Into<String>, U: SpecBuilder<FieldSpec>>(mut self, name: T, field: U) -> Self {
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

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum PaddingDirection {
    Left,
    Right
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

pub trait SpecBuilder<T> {
    fn build(self) -> T;
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

    pub fn new_empty_number() -> Self {
        Self::new_number().with_default("0")
    }

    pub fn new_string() -> Self {
        FieldSpecBuilder {
            range: None,
            padding_direction: Some(PaddingDirection::Right),
            padding: Some(" ".to_string()),
            default: None
        }
    }

    pub fn new_empty_string() -> Self {
        Self::new_string().with_default("")
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

    pub fn with_padding<T: Into<String>>(self, padding: T) -> Self {
        FieldSpecBuilder {
            range: self.range,
            padding_direction: self.padding_direction,
            padding: Some(padding.into()),
            default: self.default
        }
    }

    pub fn with_default<T: Into<String>>(self, default: T) -> Self {
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
    use std::collections::{HashMap, BTreeMap};
    use super::super::test::test_spec;

    #[test]
    fn build() {
        let spec = test_spec();
        let mut record_specs = HashMap::new();
        let mut field_specs = BTreeMap::new();
        field_specs.insert("field1".to_string(), FieldSpec {
            range: (0..4),
            padding: "dsasd".to_string(),
            padding_direction: PaddingDirection::Left,
            default: None
        });
        field_specs.insert("field2".to_string(), FieldSpec {
            range: (4..9),
            padding: " ".to_string(),
            padding_direction: PaddingDirection::Right,
            default: Some("def".to_string())
        });
        field_specs.insert("field3".to_string(), FieldSpec {
            range: (9..45),
            padding: "xcvcxv".to_string(),
            padding_direction: PaddingDirection::Right,
            default: None
        });
        record_specs.insert("record1".to_string(), RecordSpec {
            field_specs: field_specs
        });
        let mut field_specs = BTreeMap::new();
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
            default: Some("defa".to_string())
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
            field_specs: BTreeMap::new()
        });
        assert_eq!(FileSpec {
            line_spec: LineSpec {
                length: 45,
                separator: "\n".to_string()
            },
            record_specs: record_specs
        }, spec);
    }
}
