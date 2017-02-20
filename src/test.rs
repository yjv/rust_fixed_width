use spec::*;
use padder::{Padder, UnPadder, Error as PaddingError};
use recognizer::{DataRecordSpecRecognizer, LineRecordSpecRecognizer, LineBuffer};
use std::collections::{HashMap, BTreeMap};
use std::io::Read;
use record::{Data, DataRanges, WriteDataHolder, DataType};

#[derive(Debug)]
pub struct MockRecognizer<'a, T: DataRanges + 'a = (), U: WriteDataHolder + 'a = Vec<u8>> {
    line_recognize_calls: Vec<(&'a HashMap<String, RecordSpec>, Result<String, ::recognizer::Error>)>,
    data_recognize_calls: Vec<(&'a Data<T, U>, &'a HashMap<String, RecordSpec>, Result<String, ::recognizer::Error>)>
}

impl<'a, T: DataRanges + 'a, U: WriteDataHolder + 'a> MockRecognizer<'a, T, U> {
    pub fn new() -> Self {
        MockRecognizer {
            data_recognize_calls: Vec::new(),
            line_recognize_calls: Vec::new()
        }
    }

    pub fn add_line_recognize_call(&mut self, record_specs: &'a HashMap<String, RecordSpec>, return_value: Result<String, ::recognizer::Error>) -> &mut Self {
        self.line_recognize_calls.push((record_specs, return_value));
        self
    }

    pub fn add_data_recognize_call(&mut self, data: &'a Data<T, U>, record_specs: &'a HashMap<String, RecordSpec>, return_value: Result<String, ::recognizer::Error>) -> &mut Self {
        self.data_recognize_calls.push((data, record_specs, return_value));
        self
    }
}

impl<'a, T: DataRanges + 'a, U: DataType> LineRecordSpecRecognizer<U> for MockRecognizer<'a, T> {
    fn recognize_for_line<'b, V: Read + 'b>(&self, _: LineBuffer<'b, V>, record_specs: &HashMap<String, RecordSpec>) -> Result<String, ::recognizer::Error> {
        for &(ref expected_record_specs, ref return_value) in &self.line_recognize_calls {
            if *expected_record_specs as *const HashMap<String, RecordSpec> == record_specs as *const HashMap<String, RecordSpec>
            {
                return return_value.clone();
            }
        }

        panic!("Method recognize_for_line was not expected to be called with {:?}", record_specs)
    }
}

impl<'a, T: DataRanges + 'a, U: WriteDataHolder + 'a, V: DataType> DataRecordSpecRecognizer<V> for MockRecognizer<'a, T, U> {
    fn recognize_for_data<'b, W: DataRanges + 'b, X: WriteDataHolder + 'b>(&self, data: &Data<W, X>, record_specs: &HashMap<String, RecordSpec>) -> Result<String, ::recognizer::Error> {
        for &(ref expected_data, ref expected_record_specs, ref return_value) in &self.data_recognize_calls {
            if *expected_data as *const Data<T, U> == data as *const Data<W, X> as *const Data<T, U>
                && *expected_record_specs as *const HashMap<String, RecordSpec> == record_specs as *const HashMap<String, RecordSpec>
                {
                    return (*return_value).clone();
                }
        }

        panic!("Method recognize_for_data was not expected to be called with {:?}", (record_specs))
    }
}

#[derive(Debug)]
pub struct MockPadder {
    pad_calls: Vec<(Vec<u8>, usize, Vec<u8>, PaddingDirection, Result<Vec<u8>, PaddingError>)>,
    unpad_calls: Vec<(Vec<u8>, Vec<u8>, PaddingDirection, Result<Vec<u8>, PaddingError>)>
}

impl MockPadder {
    pub fn new() -> Self {
        MockPadder {
            pad_calls: Vec::new(),
            unpad_calls: Vec::new()
        }
    }

    pub fn add_pad_call(&mut self, data: Vec<u8>, length: usize, padding: Vec<u8>, direction: PaddingDirection, return_value: Result<Vec<u8>, PaddingError>) -> &mut Self {
        self.pad_calls.push((data, length, padding, direction, return_value));
        self
    }

    pub fn add_unpad_call(&mut self, data: Vec<u8>, padding: Vec<u8>, direction: PaddingDirection, return_value: Result<Vec<u8>, PaddingError>) -> &mut Self {
        self.unpad_calls.push((data, padding, direction, return_value));
        self
    }
}

impl<T: DataType> Padder<T> for MockPadder {
    fn pad<'a>(&self, data: &[u8], length: usize, padding: &[u8], direction: PaddingDirection, destination: &'a mut Vec<u8>) -> Result<(), PaddingError> {
        for &(ref expected_data, expected_length, ref expected_padding, expected_direction, ref return_value) in &self.pad_calls {
            if *expected_data == data
                && expected_length == length
                && &expected_padding[..] == padding
                && expected_direction == direction {
                return match return_value.clone() {
                    Ok(value) =>  {
                        destination.extend(value.iter());
                        Ok(())
                    },
                    Err(e) => Err(e)
                };
            }
        }

        panic!("Method pad was not expected to be called with {:?}", (data, length, padding, direction))
    }
}

impl<T: DataType> UnPadder<T> for MockPadder {
    fn unpad<'a>(&self, data: &[u8], padding: &[u8], direction: PaddingDirection, destination: &'a mut Vec<u8>) -> Result<(), PaddingError> {
        for &(ref expected_data, ref expected_padding, expected_direction, ref return_value) in &self.unpad_calls {
            if *expected_data == data
                && &expected_padding[..] == padding
                && expected_direction == direction {
                return match return_value.clone() {
                    Ok(value) =>  {
                        destination.extend(value.iter());
                        Ok(())
                    },
                    Err(e) => Err(e)
                };
            }
        }

        panic!("Method unpad was not expected to be called with {:?}", (data, padding, direction))
    }
}

pub fn test_spec() -> Spec {
    SpecBuilder::new()
        .with_record(
            "record1",
            RecordSpecBuilder::new()
                .with_line_ending("\n")
                .with_field(
                    "field1".to_string(),
                    FieldSpecBuilder::new()
                        .with_length(4)
                        .with_padding("dsasd")
                        .with_padding_direction(PaddingDirection::Left)
                        .make_filler()
                )
                .with_field(
                    "field2",
                    FieldSpecBuilder::new_string()
                        .with_length(5)
                        .with_default("def")
                )
                .with_field(
                    "field3".to_string(),
                    FieldSpecBuilder::new()
                        .with_length(36)
                        .with_padding("xcvcxv".to_string())
                        .with_padding_direction(PaddingDirection::Right)
                )
        )
        .with_record(
            "record2".to_string(),
            RecordSpecBuilder::new()
                .with_line_ending("\n")
                .with_field(
                    "field1".to_string(),
                    FieldSpecBuilder::new()
                        .with_length(3)
                        .with_padding("dsasd".to_string())
                        .with_padding_direction(PaddingDirection::Left)
                )
                .with_field(
                    "field2".to_string(),
                    FieldSpecBuilder::new()
                        .with_length(4)
                        .with_padding("sdf".to_string())
                        .with_padding_direction(PaddingDirection::Right)
                        .with_default("defa")
                )
                .with_field(
                    "field3",
                    FieldSpecBuilder::new()
                        .with_length(27)
                        .with_padding("xcvcxv".to_string())
                        .with_padding_direction(PaddingDirection::Right)
                )
                .with_field(
                    "field4".to_string(),
                    FieldSpec {
                        length: 8,
                        padding: "sdfsd".as_bytes().to_owned(),
                        padding_direction: PaddingDirection::Left,
                        default: None,
                        filler: false
                    }
                )
        )
        .with_record("record3".to_string(), RecordSpec {
            line_ending: "\n".as_bytes().to_owned(),
            field_specs: BTreeMap::new()
        })
        .build()
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
