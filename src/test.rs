use super::common::{File, MutableFile, FileError};
use super::in_memory::File as InMemoryFile;
use std::ops::Range;
use super::spec::*;
use std::collections::HashMap;

#[derive(Debug)]
pub struct MockFile {
    inner_file: InMemoryFile,
    read_errors: HashMap<usize, ()>,
    write_errors: HashMap<usize, ()>
}

impl MockFile {
    pub fn new(width: usize, initial_lines: Option<Vec<&String>>) -> Self {
        let mut file = MockFile {
            inner_file: InMemoryFile::new(width),
            read_errors: HashMap::new(),
            write_errors: HashMap::new()
        };

        if let Some(lines) = initial_lines {
            for line in lines {
                let index = file.add_line().unwrap();
                file.set(index, 0, line);
            }
        }

        file
    }

    pub fn add_read_error(&mut self, index: usize) -> &mut Self {
        if self.read_errors.get(&index).is_none() && self.write_errors.get(&index).is_none() {
            self.inner_file.insert_line(index);
        }
        self.read_errors.insert(index, ());
        self
    }

    pub fn add_write_error(&mut self, index: usize) -> &mut Self {
        if self.read_errors.get(&index).is_none() && self.write_errors.get(&index).is_none() {
            self.inner_file.insert_line(index);
        }
        self.write_errors.insert(index, ());
        self
    }
}

impl FileError for () {
    fn is_invalid_index(&self) -> bool {
        unimplemented!()
    }

    fn is_invalid_range(&self) -> bool {
        unimplemented!()
    }
}

impl File for MockFile {
    type Error = ();

    fn width(&self) -> usize {
        self.inner_file.width()
    }

    fn get(&self, index: usize, range: Range<usize>) -> Result<String, Self::Error> {
        if let Some(e) = self.read_errors.get(&index) {
            return Err(e.clone());
        }
        Ok(self.inner_file.get(index, range).unwrap())
    }

    fn len(&self) -> usize {
        self.inner_file.len()
    }
}

impl MutableFile for MockFile {
    fn set(&mut self, index: usize, column_index: usize, string: &String) -> Result<&mut Self, Self::Error> {
        if let Some(e) = self.write_errors.get(&index) {
            return Err(e.clone());
        }

        self.inner_file.set(index, column_index, string).unwrap();
        Ok(self)
    }

    fn clear(&mut self, index: usize, range: Range<usize>) -> Result<&mut Self, Self::Error> {
        if let Some(e) = self.write_errors.get(&index) {
            return Err(e.clone());
        }

        self.inner_file.clear(index, range).unwrap();
        Ok(self)
    }

    fn add_line(&mut self) -> Result<usize, Self::Error> {
        Ok(self.inner_file.add_line().unwrap())
    }

    fn remove_line(&mut self) -> Result<usize, Self::Error> {
        Ok(self.inner_file.remove_line().unwrap())
    }

    fn insert_line(&mut self, index: usize) -> Result<usize, Self::Error> {
        Ok(self.inner_file.insert_line(index).unwrap())
    }
}

pub struct TestRecognizer<'a, T: 'a + File> {
    line_recognize_calls: Vec<(&'a T, usize, &'a HashMap<String, RecordSpec>, Option<String>)>,
    data_recognize_calls: Vec<(&'a HashMap<String, String>, &'a HashMap<String, RecordSpec>, Option<String>)>
}

impl<'a, T: 'a + File> TestRecognizer<'a, T> {
    pub fn add_line_recognize_call(&mut self, file: &'a T, index: usize, record_spec: &'a HashMap<String, RecordSpec>, return_value: Option<String>) -> &mut Self {
        self.line_recognize_calls.push((file, index, record_spec, return_value));
        self
    }

    pub fn add_data_recognize_call(&mut self, data: &'a HashMap<String, String>, record_spec: &'a HashMap<String, RecordSpec>, return_value: Option<String>) -> &mut Self {
        self.data_recognize_calls.push((data, record_spec, return_value));
        self
    }
}

impl<'a, T: 'a + File> LineRecordSpecRecognizer for TestRecognizer<'a, T> {
    fn recognize_for_line<W: File>(&self, file: &W, index: usize, record_specs: &HashMap<String, RecordSpec>) -> Option<String> {
        for &(ref expected_file, expected_index, ref expected_record_specs, ref return_value) in &self.line_recognize_calls {
            if *expected_file as *const T as *const W == file as *const W
                && expected_index == index
                && *expected_record_specs as *const HashMap<String, RecordSpec> == record_specs as *const HashMap<String, RecordSpec>
            {
                return return_value.clone();
            }
        }

        panic!("Method was not expected to be called with {:?}", (index, record_specs))
    }
}

impl<'a, T: 'a + File> DataRecordSpecRecognizer for TestRecognizer<'a, T> {
    fn recognize_for_data(&self, data: &HashMap<String, String>, record_specs: &HashMap<String, RecordSpec>) -> Option<String> {
        for &(ref expected_data, ref expected_record_specs, ref return_value) in &self.data_recognize_calls {
            if *expected_data as *const HashMap<String, String> == data as *const HashMap<String, String>
                && *expected_record_specs as *const HashMap<String, RecordSpec> == record_specs as *const HashMap<String, RecordSpec>
                {
                    return return_value.clone();
                }
        }

        panic!("Method was not expected to be called with {:?}", (data, record_specs))
    }
}

pub fn test_spec() -> FileSpec {
    FileSpecBuilder::new()
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
}
