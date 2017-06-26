use spec::*;
use std::collections::HashMap;
use std::io::BufRead;
use record::{Data, DataRanges};
use data_type::{FieldReadSupport, WriteSupport};
use writer::formatter::FieldFormatter;
use reader::parser::FieldParser;
use reader::spec::{Resolver as ReaderResolver, RequiresBufRead};
use writer::spec::Resolver as WriterResolver;
use super::BoxedErrorResult as Result;

#[derive(Debug)]
pub struct MockResolver<'a> {
    line_recognize_calls: Vec<(&'a HashMap<String, RecordSpec>, Result<Option<&'a str>>)>,
    data_recognize_calls: Vec<(&'a HashMap<String, RecordSpec>, Result<Option<&'a str>>)>
}

impl<'a> MockResolver<'a> {
    pub fn new() -> Self {
        MockResolver {
            data_recognize_calls: Vec::new(),
            line_recognize_calls: Vec::new()
        }
    }

    pub fn add_line_recognize_call(&mut self, record_specs: &'a HashMap<String, RecordSpec>, return_value: Result<Option<&'a str>>) -> &mut Self {
        self.line_recognize_calls.push((record_specs, return_value));
        self
    }

    pub fn add_data_recognize_call(&mut self, record_specs: &'a HashMap<String, RecordSpec>, return_value: Result<Option<&'a str>>) -> &mut Self {
        self.data_recognize_calls.push((record_specs, return_value));
        self
    }
}

impl<'a, T: FieldReadSupport + 'a> RequiresBufRead<'a, T> for MockResolver<'a> {}

impl<'a, U: FieldReadSupport + 'a> ReaderResolver<'a, U> for MockResolver<'a> {
    fn resolve<'b, 'c, V: BufRead + 'b>(&self, _: &'b mut V, record_specs: &'c HashMap<String, RecordSpec>, _: &'b U) -> Result<Option<&'c str>> {
        for &(ref expected_record_specs, ref return_value) in &self.line_recognize_calls {
            if *expected_record_specs as *const HashMap<String, RecordSpec> == record_specs as *const HashMap<String, RecordSpec>
            {
                return match *return_value {
                    Ok(Some(ref v)) => {
                        for (key, _) in record_specs {
                            if key == v {
                                return Ok(Some(key as &str));
                            }
                        }

                        panic!("key {:?} not found in {:?}");
                    },
                    Ok(None) => Ok(None),
                    Err(_) => Err("".into())
                }
            }
        }

        panic!("Method recognize_for_line was not expected to be called with {:?}", record_specs)
    }
}

impl<'a, V: WriteSupport + 'a> WriterResolver<'a, V> for MockResolver<'a> {
    fn resolve<'b, 'c, W: DataRanges + 'b>(&self, _: &'b Data<W, V::DataHolder>, record_specs: &'c HashMap<String, RecordSpec>, _: &'b V) -> Result<Option<&'c str>> {
        for &(ref expected_record_specs, ref return_value) in &self.data_recognize_calls {
            if *expected_record_specs as *const HashMap<String, RecordSpec> == record_specs as *const HashMap<String, RecordSpec>
                {
                    return match *return_value {
                        Ok(Some(ref v)) => {
                            for (key, _) in record_specs {
                                if key == v {
                                    return Ok(Some(key as &str));
                                }
                            }

                            panic!("key {:?} not found in {:?}");
                        },
                        Ok(None) => Ok(None),
                        Err(_) => Err("".into())
                    }
                }
        }

        panic!("Method recognize_for_data was not expected to be called with {:?}", (record_specs))
    }
}

#[derive(Debug)]
pub struct MockFormatter {
    format_calls: Vec<(Vec<u8>, FieldSpec, Result<Vec<u8>>)>
}

impl MockFormatter {
    pub fn new() -> Self {
        MockFormatter {
            format_calls: Vec::new()
        }
    }

    pub fn add_format_call(&mut self, data: Vec<u8>, field_spec: FieldSpec, return_value: Result<Vec<u8>>) -> &mut Self {
        self.format_calls.push((data, field_spec, return_value));
        self
    }
}

impl<'a, T: WriteSupport + 'a> FieldFormatter<'a, T> for MockFormatter {
    fn format<'b>(&self, data: &'b [u8], field_spec: &'b FieldSpec, destination: &'b mut Vec<u8>, _: &'b T) -> Result<()> {
        for &(ref expected_data, ref expected_field_spec, ref return_value) in &self.format_calls {
            if *expected_data == data && expected_field_spec == field_spec {
                return match *return_value {
                    Ok(ref value) =>  {
                        destination.extend(value.iter());
                        Ok(())
                    },
                    Err(_) => Err("".into())
                };
            }
        }

        panic!("Method format was not expected to be called with {:?}", (data, field_spec))
    }
}

#[derive(Debug)]
pub struct MockParser {
    parse_calls: Vec<(Vec<u8>, FieldSpec, Result<Vec<u8>>)>
}

impl MockParser {
    pub fn new() -> Self {
        MockParser {
            parse_calls: Vec::new()
        }
    }

    pub fn add_parse_call(&mut self, data: Vec<u8>, field_spec: FieldSpec, return_value: Result<Vec<u8>>) -> &mut Self {
        self.parse_calls.push((data, field_spec, return_value));
        self
    }
}

impl<'a, T: FieldReadSupport + 'a> FieldParser<'a, T> for MockParser {
    fn parse<'b>(&self, data: &'b [u8], field_spec: &'b FieldSpec, destination: &'b mut Vec<u8>, _: &'b T) -> Result<()> {
        for &(ref expected_data, ref expected_field_spec, ref return_value) in &self.parse_calls {
            if *expected_data == data
                && expected_field_spec == field_spec {
                return match *return_value {
                    Ok(ref value) =>  {
                        destination.extend(value.iter());
                        Ok(())
                    },
                    Err(_) => Err("".into())
                };
            }
        }

        panic!("Method parse was not expected to be called with {:?}", (data, field_spec))
    }
}

pub fn test_spec() -> Spec {
    let record2field4 = FieldSpecBuilder::new()
        .with_length(8)
        .with_padding_direction(PaddingDirection::Left)
        .with_padding("sdfsd")
        .build()
        .unwrap()
    ;
    let record3 = RecordSpecBuilder::new()
        .with_line_ending("\n")
        .build()
        .unwrap()
    ;
    SpecBuilder::new()
        .with_record("record1")
            .with_line_ending("\n")
            .add_field(
                "field1".to_string(),
                FieldSpecBuilder::new()
                    .with_length(4)
                    .with_padding("dsasd")
                    .with_padding_direction(PaddingDirection::Left)
            )
            .add_field(
                "field2",
                FieldSpecBuilder::new()
                    .string()
                    .with_length(5)
                    .with_default("def")
            )
            .add_field(
                "field3".to_string(),
                FieldSpecBuilder::new()
                    .with_length(36)
                    .with_padding("xcvcxv".to_string())
                    .with_padding_direction(PaddingDirection::Right)
            )
        .end()
        .add_record(
            "record2".to_string(),
            RecordSpecBuilder::new()
                .with_line_ending("\n")
                .add_field(
                    "field1".to_string(),
                    FieldSpecBuilder::new()
                        .with_length(3)
                        .with_padding("dsasd".to_string())
                        .with_padding_direction(PaddingDirection::Left)
                )
                .with_field("field2".to_string())
                    .with_length(4)
                    .with_padding("sdf".to_string())
                    .with_padding_direction(PaddingDirection::Right)
                    .with_default("defa")
                .end()
                .add_field(
                    "field3",
                    FieldSpecBuilder::new()
                        .with_length(27)
                        .with_padding("xcvcxv".to_string())
                        .with_padding_direction(PaddingDirection::Right)
                )
                .add_field(
                    "field4".to_string(),
                    record2field4
                )
        )
        .add_record("record3".to_string(), record3)
        .build()
        .unwrap()
}

#[macro_export]
macro_rules! assert_result {
    (Ok($left:expr), $right:expr) => {
        match $right {
            Ok(v) => assert_eq!($left, v),
            e => panic!("Failed result returned was not the expected one {:?}", e)
        }
    };
    ($left:pat, $right:expr) => {
        match $right {
            $left => (),
            v => panic!("Failed result returned was not the expected one {:?}", v)
        }
    };
    ($left:pat if $leftif:expr, $right:expr) => {
        match $right {
            $left if $leftif => (),
            v => panic!("Failed result returned was not the expected one {:?}", v)
        }
    }
}

#[macro_export]
macro_rules! assert_option {
    ($left:pat, $right:expr) => {
        match $right {
            $left => (),
            e => panic!("Failed option returned was not the expected one {:?}", e)
        }
    }
}
