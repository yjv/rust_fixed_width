pub mod resolver;
pub mod stream;
pub mod loader;

use std::collections::{HashMap, BTreeMap};
use std::ops::Range;
use std::iter::repeat;
use ::std::fmt::{Display, Error as FmtError, Formatter};

type Result<T> = ::std::result::Result<T, Error>;

pub trait Builder<T> {
    fn build(self) -> Result<T>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Spec {
    pub record_specs: HashMap<String, RecordSpec>,
    __no_construct: ()
}

impl Builder<Spec> for Spec {
    fn build(self) -> Result<Self> {
        Ok(self)
    }
}

pub struct SpecBuilder {
    record_specs: HashMap<String, Result<RecordSpec>>,
    sub_builder_error: bool
}

impl SpecBuilder {
    pub fn new() -> Self {
        SpecBuilder {
            record_specs: HashMap::new(),
            sub_builder_error: false
        }
    }

    pub fn add_record<T: Into<String>, U: Builder<RecordSpec>>(mut self, name: T, record: U) -> Self {
        let record = record.build();
        self.sub_builder_error = self.sub_builder_error || record.is_err();
        self.record_specs.insert(name.into(), record);
        self
    }

    pub fn with_record<T: Into<String>>(self, name: T) -> RecordSpecBuilder {
        RecordSpecBuilder::new_with_spec_builder(name, self)
    }
}

impl Builder<Spec> for SpecBuilder {
    fn build(self) -> Result<Spec> {
        if self.sub_builder_error {
            Err(Error::SubBuilderErrors(self.record_specs.into_iter()
                .filter(|&(_, ref result)| result.is_err())
                .map(|(name, result)| (name, result.unwrap_err()))
                .collect()
            ))
        } else {
            Ok(Spec {
                record_specs: self.record_specs.into_iter().map(|(name, result)| (name, result.expect("no errors should be in here"))).collect(),
                __no_construct: ()
            })
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordSpec {
    pub line_ending: Vec<u8>,
    pub field_specs: BTreeMap<String, FieldSpec>,
    __no_construct: ()
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
    fn build(self) -> Result<Self> {
        Ok(self)
    }
}

pub struct RecordSpecBuilder {
    line_ending: Vec<u8>,
    field_specs: BTreeMap<String, Result<FieldSpec>>,
    sub_builder_error: bool,
    spec_builder: Option<SpecBuilder>,
    name: Option<String>
}

impl RecordSpecBuilder {
    pub fn new() -> Self {
        RecordSpecBuilder {
            line_ending: Vec::new(),
            field_specs: BTreeMap::new(),
            sub_builder_error: false,
            spec_builder: None,
            name: None,
        }
    }

    pub fn new_with_spec_builder<T: Into<String>>(name: T, spec_builder: SpecBuilder) -> Self {
        RecordSpecBuilder {
            line_ending: Vec::new(),
            field_specs: BTreeMap::new(),
            sub_builder_error: false,
            spec_builder: Some(spec_builder),
            name: Some(name.into())
        }
    }

    pub fn add_field<T: Into<String>, U: Builder<FieldSpec>>(mut self, name: T, field: U) -> Self {
        let field = field.build();
        self.sub_builder_error = self.sub_builder_error || field.is_err();
        self.field_specs.insert(name.into(), field);
        self
    }

    pub fn with_field<T: Into<String>>(self, name: T) -> FieldSpecBuilder {
        FieldSpecBuilder::new_with_record_spec_builder(name, self)
    }

    pub fn with_line_ending<T: Into<Vec<u8>>>(mut self, line_ending: T) -> Self {
        self.line_ending = line_ending.into();
        self
    }

    pub fn end(mut self) -> SpecBuilder {
        let name = self.name.take().expect("calling end infers that this was created with the name connected");
        self.spec_builder.take()
            .expect("calling end infers that this was created with the parent spec builder connected")
            .add_record(name, self)
    }
}

impl Builder<RecordSpec> for RecordSpecBuilder {
    fn build(self) -> Result<RecordSpec> {
        if self.sub_builder_error {
            Err(Error::SubBuilderErrors(self.field_specs.into_iter()
                .filter(|&(_, ref result)| result.is_err())
                .map(|(name, result)| (name, result.unwrap_err()))
                .collect()
            ))
        } else {
            Ok(RecordSpec {
                line_ending: self.line_ending,
                field_specs: self.field_specs.into_iter().map(|(name, result)| (name, result.expect("no errors should be in here"))).collect(),
                __no_construct: ()
            })
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
    __no_construct: ()
}

impl Builder<FieldSpec> for FieldSpec {
    fn build(self) -> Result<Self> {
        Ok(self)
    }
}

pub struct FieldSpecBuilder {
    length: Option<usize>,
    padding_direction: Option<PaddingDirection>,
    padding: Option<Vec<u8>>,
    default: Option<Vec<u8>>,
    record_spec_builder: Option<RecordSpecBuilder>,
    name: Option<String>
}

impl Clone for FieldSpecBuilder {
    fn clone(&self) -> Self {
        FieldSpecBuilder {
            length: self.length.clone(),
            padding_direction: self.padding_direction.clone(),
            padding: self.padding.clone(),
            default: self.default.clone(),
            record_spec_builder: None,
            name: None
        }
    }
}

impl FieldSpecBuilder {
    pub fn new() -> Self {
        FieldSpecBuilder {
            length: None,
            padding_direction: None,
            padding: None,
            default: None,
            record_spec_builder: None,
            name: None,
        }
    }

    pub fn new_with_record_spec_builder<T: Into<String>>(name: T, record_spec_builder: RecordSpecBuilder) -> Self {
        FieldSpecBuilder {
            length: None,
            padding_direction: None,
            padding: None,
            default: None,
            record_spec_builder: Some(record_spec_builder),
            name: Some(name.into()),
        }
    }

    pub fn number(self) -> Self {
        self.with_padding("0").with_padding_direction(PaddingDirection::Left)
    }

    pub fn empty_number(self) -> Self {
        self.with_default("0")
    }

    pub fn string(self) -> Self {
        self.with_padding(" ").with_padding_direction(PaddingDirection::Right)
    }

    pub fn empty_string(self) -> Self {
        self.string().with_default("")
    }

    pub fn filler(self, length: usize) -> Self {
        self.string()
            .with_default(repeat(" ").take(length).collect::<String>())
            .with_length(length)
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

    pub fn end(mut self) -> RecordSpecBuilder {
        let name = self.name.take().expect("calling end infers that this was created with the name connected");
        self.record_spec_builder.take()
            .expect("calling end infers that this was created with the parent record spec builder connected")
            .add_field(name, self)
    }
}

impl Builder<FieldSpec> for FieldSpecBuilder {
    fn build(self) -> Result<FieldSpec> {
        Ok(FieldSpec {
            length: self.length.ok_or(Error::FieldRequiredToBuild("length"))?,
            padding_direction: self.padding_direction.ok_or(Error::FieldRequiredToBuild("padding"))?,
            padding: self.padding.unwrap_or_default(),
            default: self.default,
            __no_construct: (),
        })
    }
}

#[derive(Debug)]
pub enum Error {
    FieldRequiredToBuild(&'static str),
    SubBuilderErrors(HashMap<String, Error>)
}

impl ::std::error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::FieldRequiredToBuild(_) => "There is a required field missing",
            Error::SubBuilderErrors(_) => "Some sub builders had errors"
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> ::std::result::Result<(), FmtError> {
        match *self {
            Error::FieldRequiredToBuild(ref field) => write!(f, "{} must be set in order to build", field),
            Error::SubBuilderErrors(ref errors) => {
                write!(f, "Some sub builders had errors: ")?;
                for (name, error) in errors {
                    write!(f, "\n {}: {}", name, error)?;
                }

                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::{HashMap, BTreeMap};
    use test::test_spec;

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
            __no_construct: ()
        });
        field_specs.insert("field2".to_string(), FieldSpec {
            length: 5,
            padding: " ".as_bytes().to_owned(),
            padding_direction: PaddingDirection::Right,
            default: Some("def".as_bytes().to_owned()),
            __no_construct: ()
        });
        field_specs.insert("field3".to_string(), FieldSpec {
            length: 36,
            padding: "xcvcxv".as_bytes().to_owned(),
            padding_direction: PaddingDirection::Right,
            default: None,
            __no_construct: ()
        });
        record_specs.insert("record1".to_string(), RecordSpec {
            line_ending: "\n".as_bytes().to_owned(),
            field_specs: field_specs,
            __no_construct: ()
        });
        let mut field_specs = BTreeMap::new();
        field_specs.insert("field1".to_string(), FieldSpec {
            length: 3,
            padding: "dsasd".as_bytes().to_owned(),
            padding_direction: PaddingDirection::Left,
            default: None,
            __no_construct: ()
        });
        field_specs.insert("field2".to_string(), FieldSpec {
            length: 4,
            padding: "sdf".as_bytes().to_owned(),
            padding_direction: PaddingDirection::Right,
            default: Some("defa".as_bytes().to_owned()),
            __no_construct: (),
        });
        field_specs.insert("field3".to_string(), FieldSpec {
            length: 27,
            padding: "xcvcxv".as_bytes().to_owned(),
            padding_direction: PaddingDirection::Right,
            default: None,
            __no_construct: (),
        });
        field_specs.insert("field4".to_string(), FieldSpec {
            length: 8,
            padding: "sdfsd".as_bytes().to_owned(),
            padding_direction: PaddingDirection::Left,
            default: None,
            __no_construct: (),
        });
        record_specs.insert("record2".to_string(), RecordSpec {
            line_ending: "\n".as_bytes().to_owned(),
            field_specs: field_specs,
            __no_construct: (),
        });
        record_specs.insert("record3".to_string(), RecordSpec {
            line_ending: "\n".as_bytes().to_owned(),
            field_specs: BTreeMap::new(),
            __no_construct: (),
        });
        assert_eq!(Spec {
            record_specs: record_specs,
            __no_construct: (),
        }, spec);
        assert_eq!(FieldSpecBuilder::new()
            .with_padding("0".to_string())
            .with_padding_direction(PaddingDirection::Left)
            .with_length(0)
            .build()
            .unwrap()
        , FieldSpecBuilder::new().number().with_length(0).build().unwrap());
        assert_eq!(FieldSpecBuilder::new()
            .with_padding(" ".as_bytes().to_owned())
            .with_padding_direction(PaddingDirection::Right)
            .with_length(0)
            .build()
            .unwrap()
        , FieldSpecBuilder::new().string().with_length(0).build().unwrap());
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
