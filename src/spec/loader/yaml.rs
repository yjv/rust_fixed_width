extern crate yaml_rust;
use self::yaml_rust::{Yaml};
use std::fs::File;
use std::io::prelude::*;
use std::collections::{BTreeMap, HashMap};
use spec::{FieldSpec, RecordSpec, Spec, PaddingDirection};
use error::BoxedError;
use std::fmt::{Display, Formatter, Error as FmtError};
use std::path::Path;

pub struct YamlLoader;

impl<T: AsRef<Path>> super::Loader<T> for YamlLoader {
    fn load(&self, resource: T) -> ::Result<Spec> {
        let mut file = File::open(resource.as_ref()).map_err(|e| ::error::Error::SpecLoaderError(Box::new(e)))?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).map_err(|e| ::error::Error::SpecLoaderError(Box::new(e)))?;
        let docs = yaml_rust::YamlLoader::load_from_str(&contents).map_err(|e| ::error::Error::SpecLoaderError(Box::new(e)))?;
        self.read_spec(docs).map_err(::error::Error::SpecLoaderError)
    }
}

impl YamlLoader {
    fn read_spec(&self, docs: Vec<Yaml>) -> Result<Spec, BoxedError> {
        let mut record_specs = HashMap::new();

        for mut doc in docs {
            let mut records = Self::get_hash(Self::get_hash(doc, None)?.remove(&Yaml::String("records".to_string())).ok_or(Error::missing_key("records", None))?, Some(&["records"]))?;

            for (name, record_spec_data) in records {
                let path = &["records"];
                let name = Self::get_string(name, Some(path))?;
                let record_spec = Self::get_record_spec(record_spec_data, &name)?;
                record_specs.insert(
                    name,
                    record_spec
                );
            }
        }

        Ok(Spec { record_specs: record_specs })
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
        let mut fields = Self::get_hash(record_spec_data.remove(&Yaml::String("fields".to_string())).ok_or(Error::missing_key("records", Some(path)))?, Some(path))?;

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
            Error::MissingKey { .. } => "There is a key missing",
            Error::InvalidType { .. } => "The type is wrong"
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> ::std::result::Result<(), FmtError> {
        match *self {
            Error::MissingKey { key: ref key, path: Some(ref path) } => write!(f, "There is a key {} missing under key {}", key, path),
            Error::MissingKey { key: ref key, path: None } => write!(f, "There is a key {} missing", key),
            Error::InvalidType { value: ref value, expected_type: ref expected_type, path: Some(ref path) } => write!(f, "The value {:?} at path {} has the wrong type. The expected type was {}", value, path, expected_type),
            Error::InvalidType { value: ref value, expected_type: ref expected_type, path: None } => write!(f, "The value {:?} has the wrong type. The expected type was {}", value, expected_type)
        }
    }
}

#[cfg(test)]
mod test {
    use super::YamlLoader;
    use spec::loader::Loader;

    #[test]
    fn read_record() {
        let loader = YamlLoader;
        println!("{:?}", loader.load("src/spec/loader/spec.yml"));
    }
}
