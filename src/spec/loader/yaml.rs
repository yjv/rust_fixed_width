extern crate yaml_rust;
use self::yaml_rust::{Yaml};
use std::io::prelude::*;
use std::collections::{BTreeMap, HashMap};
use spec::{FieldSpec, RecordSpec, Spec, PaddingDirection};
use error::BoxedError;
use std::fmt::{Display, Formatter, Error as FmtError};

pub struct YamlLoader;

impl<'a, T: 'a + Read> super::Loader<&'a mut T> for YamlLoader {
    fn load(&self, resource: &'a mut T) -> ::Result<Spec> {
        let mut docs = Self::read_reader(resource).map_err(::error::Error::SpecLoaderError)?;

        if docs.len() == 0 {
            return Err(::error::Error::SpecLoaderError(Box::new(Error::NoDocumentsFound)));
        }

        Self::read_spec(docs.remove(0)).map_err(::error::Error::SpecLoaderError)
    }
}

impl<'a, T: 'a + Read> super::MultiLoader<&'a mut T> for YamlLoader {
    fn multi_load(&self, resource: &'a mut T) -> ::Result<Vec<Spec>> {
        let docs = Self::read_reader(resource).map_err(::error::Error::SpecLoaderError)?;

        let mut specs = Vec::new();

        for doc in docs {
            specs.push(Self::read_spec(doc).map_err(::error::Error::SpecLoaderError)?);
        }

        Ok(specs)
    }
}

impl YamlLoader {
    fn read_spec(doc: Yaml) -> Result<Spec, BoxedError> {
        let mut record_specs = HashMap::new();

        let records = Self::get_hash(Self::get_hash(doc, None)?.remove(&Yaml::String("records".to_string())).ok_or(Error::missing_key("records", None))?, Some(&["records"]))?;

        for (name, record_spec_data) in records {
            let path = &["records"];
            let name = Self::get_string(name, Some(path))?;
            let record_spec = Self::get_record_spec(record_spec_data, &name)?;
            record_specs.insert(
                name,
                record_spec
            );
        }

        Ok(Spec { record_specs: record_specs })
    }

    fn read_reader<'a, T: 'a + Read>(resource: &'a mut T) -> Result<Vec<Yaml>, BoxedError> {
        let mut contents = String::new();
        resource.read_to_string(&mut contents)?;
        Ok(yaml_rust::YamlLoader::load_from_str(&contents)?)
    }

    fn get_field_spec<'a>(field_spec_data: Yaml, name: &'a str, field_name: &'a str) -> Result<FieldSpec, Error> {
        let path = &["records", name, "fields", &field_name];
        let mut field_spec_map = Self::get_hash(field_spec_data, Some(path))?;
        Ok(FieldSpec {
            default: match field_spec_map.remove(&Yaml::String("default".to_string())) {
                Some(v) => Some(Self::get_bytes(v, Some(path))?),
                None => None
            },
            length: field_spec_map.remove(&Yaml::String("length".to_string())).map(|v| Self::get_usize(v, Some(path))).unwrap_or_else(|| Err(Error::missing_key("length", Some(path))))?,
            padding: field_spec_map.remove(&Yaml::String("padding".to_string())).map(|v| Self::get_bytes(v, Some(path))).unwrap_or_else(|| Ok(Vec::new()))?,
            padding_direction: field_spec_map.remove(&Yaml::String("padding_direction".to_string())).map(|v| Self::get_padding_direction(v, Some(path))).unwrap_or_else(|| Err(Error::missing_key("padding_direction", Some(path))))?
        })
    }

    fn get_record_spec<'a>(record_spec_data: Yaml, name: &'a str) -> Result<RecordSpec, Error> {
        let path = &["records", &name];
        let mut record_spec_data = Self::get_hash(record_spec_data, Some(path))?;
        let mut field_specs = BTreeMap::new();
        let path = &["records", &name, "fields"];
        let fields = Self::get_hash(record_spec_data.remove(&Yaml::String("fields".to_string())).ok_or(Error::missing_key("records", Some(path)))?, Some(path))?;

        for (field_name, field_spec_data) in fields {
            let field_name = Self::get_string(field_name, Some(path))?;
            let field_spec = Self::get_field_spec(field_spec_data, &name, &field_name)?;
            field_specs.insert(
                field_name,
                field_spec
            );
        }

        Ok(RecordSpec {
            line_ending: record_spec_data.remove(&Yaml::String("line_ending".to_string())).map(|v| Self::get_bytes(v, Some(path))).unwrap_or_else(|| Ok(Vec::new()))?,
            field_specs: field_specs
        })
    }

    fn get_hash<'a, 'b>(value: Yaml, path: Option<&'a [&'b str]>) -> Result<BTreeMap<Yaml, Yaml>, Error> {
        match value {
            Yaml::Hash(v) => Ok(v),
            _ => Err(Error::invalid_type(value, "Hash", path))
        }
    }

    fn get_string<'a, 'b>(value: Yaml, path: Option<&'a [&'b str]>) -> Result<String, Error> {
        match value {
            Yaml::String(v) => Ok(v),
            Yaml::Integer(v) => Ok(v.to_string()),
            _ => Err(Error::invalid_type(value, "String", path))
        }
    }

    fn get_bytes<'a, 'b>(value: Yaml, path: Option<&'a [&'b str]>) -> Result<Vec<u8>, Error> {
        Self::get_string(value, path).map(String::into_bytes)
    }

    fn get_usize<'a, 'b>(value: Yaml, path: Option<&'a [&'a str]>) -> Result<usize, Error> {
        match value {
            Yaml::Integer(v) => Ok(v as usize),
            _ => Err(Error::invalid_type(value, "Integer", path))
        }
    }

    fn get_padding_direction<'a, 'b>(value: Yaml, path: Option<&'a [&'b str]>) -> Result<PaddingDirection, Error> {
        match value {
            Yaml::String(ref v) if v == "right" => Ok(PaddingDirection::Right),
            Yaml::String(ref v) if v == "Right" => Ok(PaddingDirection::Right),
            Yaml::String(ref v) if v == "left" => Ok(PaddingDirection::Left),
            Yaml::String(ref v) if v == "Left" => Ok(PaddingDirection::Left),
            _ => Err(Error::invalid_type(value, "String: right, Right, left, Left", path))
        }
    }
}

#[derive(Debug)]
pub enum Error {
    NoDocumentsFound,
    MissingKey { key: &'static str, path: Option<String> },
    InvalidType { value: Yaml, expected_type: &'static str, path: Option<String> }
}

impl Error {
    fn missing_key<'a, 'b>(key: &'static str, path: Option<&'a [&'b str]>) -> Self {
        Error::MissingKey {
            key: key,
            path: path.map(Self::normalize_path)
        }
    }

    fn invalid_type<'a, 'b>(value: Yaml, expected_type: &'static str, path: Option<&'a [&'b str]>) -> Self {
        Error::InvalidType {
            value: value,
            expected_type: expected_type,
            path: path.map(Self::normalize_path)
        }
    }

    fn normalize_path<'a, 'b>(path: &'a [&'b str]) -> String {
        let mut string = String::new();
        for element in path {
            string.push_str(element);
        }

        string
    }
}

impl ::std::error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::NoDocumentsFound => "The resource at the given path has no documents in it",
            Error::MissingKey { .. } => "There is a key missing",
            Error::InvalidType { .. } => "The type is wrong"
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> ::std::result::Result<(), FmtError> {
        match *self {
            Error::NoDocumentsFound => write!(f, "The resource at the given path has no documents in it"),
            Error::MissingKey { ref key, path: Some(ref path) } => write!(f, "There is a key {} missing under key {}", key, path),
            Error::MissingKey { ref key, path: None } => write!(f, "There is a key {} missing", key),
            Error::InvalidType { ref value, ref expected_type, path: Some(ref path) } => write!(f, "The value {:?} at path {} has the wrong type. The expected type was {}", value, path, expected_type),
            Error::InvalidType { ref value, ref expected_type, path: None } => write!(f, "The value {:?} has the wrong type. The expected type was {}", value, expected_type)
        }
    }
}

#[cfg(test)]
mod test {
    use super::YamlLoader;
    use spec::loader::{Loader, MultiLoader};
    use spec::{RecordSpecBuilder, SpecBuilder, FieldSpecBuilder, PaddingDirection, Builder};
    use std::fs::File;

    #[test]
    fn read_record() {
        let loader = YamlLoader;
        let spec = SpecBuilder::new()
            .with_record(
                "record1",
                RecordSpecBuilder::new()
                    .with_line_ending([92, 110].as_ref())
                    .with_field(
                        "$id",
                        FieldSpecBuilder::new()
                            .with_length(2)
                            .with_padding_direction(PaddingDirection::Right)
                            .with_default([51, 52].as_ref())
                    )
                    .with_field(
                        "field1",
                        FieldSpecBuilder::new()
                            .with_length(10)
                            .with_padding_direction(PaddingDirection::Left)
                            .with_padding([32].as_ref())
                            .with_default([104, 101, 108, 108, 111].as_ref())
                    )
                    .with_field(
                        "field2",
                        FieldSpecBuilder::new()
                            .with_length(23)
                            .with_padding_direction(PaddingDirection::Right)
                            .with_default([103, 111, 111, 100, 98, 121, 101].as_ref())
                    )
            )
            .with_record(
                "record2",
                RecordSpecBuilder::new()
                    .with_line_ending([92, 110].as_ref())
                    .with_field(
                        "$id",
                        FieldSpecBuilder::new()
                            .with_length(5)
                            .with_padding_direction(PaddingDirection::Right)
                            .with_default([51, 52].as_ref())
                    )
                    .with_field(
                        "field1",
                        FieldSpecBuilder::new()
                            .with_length(12)
                            .with_padding_direction(PaddingDirection::Left)
                            .with_padding([32].as_ref())
                            .with_default([104, 101, 108, 108, 111].as_ref())
                    )
                    .with_field(
                        "field2",
                        FieldSpecBuilder::new()
                            .with_length(67)
                            .with_padding_direction(PaddingDirection::Right)
                            .with_default([103, 111, 111, 100, 98, 121, 101].as_ref())
                    )
            )
            .build()
            .unwrap()
        ;
        assert_eq!(spec, loader.load(&mut File::open("src/spec/loader/spec.yml").unwrap()).unwrap());
        assert_eq!(vec![spec], loader.multi_load(&mut File::open("src/spec/loader/spec.yml").unwrap()).unwrap());
    }
}
