use common::File;
use spec::{FileSpec, LineRecordSpecRecognizer, NoneRecognizer, UnPadder, IdentityPadder};
use std::collections::HashMap;

#[derive(Debug)]
pub enum Error<T: File, U: UnPadder> {
    RecordSpecNotFound(String),
    RecordSpecNameRequired,
    GetFailed(String, T::Error),
    FieldSpecNotFound(String),
    UnPaddingFailed(U::Error)
}

pub struct FileReader<'a, T: 'a + LineRecordSpecRecognizer, U: 'a + UnPadder> {
    spec: &'a FileSpec,
    recognizer: T,
    un_padder: U
}

impl<'a, T: 'a + LineRecordSpecRecognizer, U: 'a + UnPadder> FileReader<'a, T, U> {
    pub fn new(spec: &'a FileSpec) -> FileReader<'a, NoneRecognizer, IdentityPadder> {
        FileReader { spec: spec, recognizer: NoneRecognizer, un_padder: IdentityPadder }
    }

    pub fn new_with_recognizer_and_un_padder(spec: &'a FileSpec, recognizer: T, un_padder: U) -> Self {
        FileReader {spec: spec, recognizer: recognizer, un_padder: un_padder}
    }

    pub fn field<W: File>(&self, file: &W, index: usize, name: String, spec_name: Option<String>) -> Result<String, Error<W, U>> {
        let record_spec_name = try!(spec_name.or_else(|| self.recognizer.recognize_for_line(file, index, &self.spec.record_specs)).ok_or(Error::RecordSpecNameRequired));
        let record_spec = try!(self.spec.record_specs.get(
            &record_spec_name
        ).ok_or(Error::RecordSpecNotFound(record_spec_name)));

        let field_spec = try!(record_spec.field_specs.get(&name).ok_or_else(|| Error::FieldSpecNotFound(name.clone())));

        let data = try!(file.get(
            index,
            field_spec.range.clone()
        ).map_err(|e| Error::GetFailed(name, e)));

        Ok(try!(self.un_padder.unpad(&data, &field_spec.padding, field_spec.padding_direction).map_err(Error::UnPaddingFailed)))
    }

    pub fn line<W: File>(&self, file: &W, index: usize, spec_name: Option<String>) -> Result<HashMap<String, String>, Error<W, U>> {
        let record_spec_name = try!(spec_name.or_else(|| self.recognizer.recognize_for_line(file, index, &self.spec.record_specs)).ok_or(Error::RecordSpecNameRequired));
        let record_spec = try!(self.spec.record_specs.get(
            &record_spec_name
        ).ok_or(Error::RecordSpecNotFound(record_spec_name)));

        let mut fields = HashMap::new();

        for (name, field_spec) in &record_spec.field_specs {
            let data = try!(file.get(index, field_spec.range.clone()).map_err(|e| Error::GetFailed(name.clone(), e)));
            fields.insert(
                name.clone(),
                try!(self.un_padder.unpad(&data, &field_spec.padding, field_spec.padding_direction).map_err(Error::UnPaddingFailed))
            );
        }
        Ok(fields)
    }
}

pub struct FileIterator<'a, T: 'a + File, U: 'a + LineRecordSpecRecognizer, V: 'a + UnPadder> {
    position: usize,
    file: &'a T,
    reader: &'a FileReader<'a, U, V>
}

impl<'a, T: File, U: LineRecordSpecRecognizer, V: UnPadder> FileIterator<'a, T, U, V> {
    pub fn new(reader: &'a FileReader<'a, U, V>, file: &'a T) -> Self {
        FileIterator {
            position: 0,
            file: file,
            reader: reader
        }
    }
}

impl<'a, T: File, U: LineRecordSpecRecognizer, V: UnPadder> Iterator for FileIterator<'a, T, U, V> {
    type Item = Result<HashMap<String, String>, Error<T, V>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.position = self.position + 1;
        if self.position > self.file.len() {
            None
        } else {
            Some(self.reader.line(self.file, self.position - 1, None))
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use super::super::test::*;
    use std::iter::repeat;
    use std::collections::HashMap;

    #[test]
    fn read() {
        let spec = test_spec();
        let line1 = repeat("line1").take(9).collect();
        let line2 = repeat("line2").take(9).collect();
        let line3 = repeat("line3").take(9).collect();
        let line4 = repeat("line4").take(9).collect();
        let mut file = MockFile::new(45, Some(vec![
            &line1,
            &line2,
            &line3,
            &line4
        ]));
        file.add_read_error(1);
        let mut recognizer = MockRecognizer::new();
        let mut padder = MockPadder::new();
        recognizer.add_line_recognize_call(&file, 0, &spec.record_specs, Some("record2".to_string()));
        recognizer.add_line_recognize_call(&file, 1, &spec.record_specs, None);
        recognizer.add_line_recognize_call(&file, 3, &spec.record_specs, Some("record1".to_string()));
        padder.add_unpad_call(
            line1[0..3].to_string(),
            spec.record_specs.get(&"record2".to_string()).unwrap().field_specs.get(&"field1".to_string()).unwrap().padding.clone(),
            spec.record_specs.get(&"record2".to_string()).unwrap().field_specs.get(&"field1".to_string()).unwrap().padding_direction,
            Ok("field1_value".to_string())
        );
        padder.add_unpad_call(
            line1[4..8].to_string(),
            spec.record_specs.get(&"record2".to_string()).unwrap().field_specs.get(&"field2".to_string()).unwrap().padding.clone(),
            spec.record_specs.get(&"record2".to_string()).unwrap().field_specs.get(&"field2".to_string()).unwrap().padding_direction,
            Ok("field2_value".to_string())
        );
        padder.add_unpad_call(
            line1[9..36].to_string(),
            spec.record_specs.get(&"record2".to_string()).unwrap().field_specs.get(&"field3".to_string()).unwrap().padding.clone(),
            spec.record_specs.get(&"record2".to_string()).unwrap().field_specs.get(&"field3".to_string()).unwrap().padding_direction,
            Ok("field3_value".to_string())
        );
        padder.add_unpad_call(
            line1[37..45].to_string(),
            spec.record_specs.get(&"record2".to_string()).unwrap().field_specs.get(&"field4".to_string()).unwrap().padding.clone(),
            spec.record_specs.get(&"record2".to_string()).unwrap().field_specs.get(&"field4".to_string()).unwrap().padding_direction,
            Ok("field4_value".to_string())
        );
        padder.add_unpad_call(
            line2[0..4].to_string(),
            spec.record_specs.get(&"record1".to_string()).unwrap().field_specs.get(&"field1".to_string()).unwrap().padding.clone(),
            spec.record_specs.get(&"record1".to_string()).unwrap().field_specs.get(&"field1".to_string()).unwrap().padding_direction,
            Err(())
        );
        padder.add_unpad_call(
            line2[4..9].to_string(),
            spec.record_specs.get(&"record1".to_string()).unwrap().field_specs.get(&"field2".to_string()).unwrap().padding.clone(),
            spec.record_specs.get(&"record1".to_string()).unwrap().field_specs.get(&"field2".to_string()).unwrap().padding_direction,
            Err(())
        );
        padder.add_unpad_call(
            line2[9..45].to_string(),
            spec.record_specs.get(&"record1".to_string()).unwrap().field_specs.get(&"field3".to_string()).unwrap().padding.clone(),
            spec.record_specs.get(&"record1".to_string()).unwrap().field_specs.get(&"field3".to_string()).unwrap().padding_direction,
            Err(())
        );
        let reader = FileReader::new_with_recognizer_and_un_padder(&spec, recognizer, padder);
        let mut data = HashMap::new();
        data.insert("field1".to_string(), "field1_value".to_string());
        data.insert("field2".to_string(), "field2_value".to_string());
        data.insert("field3".to_string(), "field3_value".to_string());
        data.insert("field4".to_string(), "field4_value".to_string());
        assert_eq!(data, reader.line(&file, 0, None).unwrap());
        match reader.line(&file, 1, None).unwrap_err() {
            Error::RecordSpecNameRequired => (),
            e => panic!("did not return correct error {:?}", e)
        }
        match reader.line(&file, 1, Some("record5".to_string())).unwrap_err() {
            Error::RecordSpecNotFound(name) => assert_eq!("record5".to_string(), name),
            e => panic!("did not return correct error {:?}", e)
        }
        match reader.line(&file, 1, Some("record2".to_string())).unwrap_err() {
            Error::GetFailed(field, ()) => assert!(
                field == "field1".to_string()
                || field == "field2".to_string()
                || field == "field3".to_string()
                || field == "field4".to_string()
            ),
            e => panic!("did not return correct error {:?}", e)
        }
        match reader.line(&file, 2, Some("record1".to_string())).unwrap_err() {
            Error::UnPaddingFailed(()) => (),
            e => panic!("did not return correct error {:?}", e)
        }

        assert_eq!("field2_value".to_string(), reader.field(&file, 0, "field2".to_string(), None).unwrap());
        match reader.field(&file, 1, "field1".to_string(), None).unwrap_err() {
            Error::RecordSpecNameRequired => (),
            e => panic!("did not return correct error {:?}", e)
        }
        match reader.field(&file, 1, "field1".to_string(), Some("record5".to_string())).unwrap_err() {
            Error::RecordSpecNotFound(name) => assert_eq!("record5".to_string(), name),
            e => panic!("did not return correct error {:?}", e)
        }
        match reader.field(&file, 1, "field3".to_string(), Some("record2".to_string())).unwrap_err() {
            Error::GetFailed(field, ()) => assert_eq!(field, "field3".to_string()),
            e => panic!("did not return correct error {:?}", e)
        }
        match reader.field(&file, 2, "field2".to_string(), Some("record1".to_string())).unwrap_err() {
            Error::UnPaddingFailed(()) => (),
            e => panic!("did not return correct error {:?}", e)
        }
    }

    #[test]
    fn iterate() {
        let spec = test_spec();
        let line1 = repeat("line1").take(9).collect();
        let line2 = repeat("line2").take(9).collect();
        let file = MockFile::new(45, Some(vec![
            &line1,
            &line2
        ]));
        let mut recognizer = MockRecognizer::new();
        let mut padder = MockPadder::new();
        recognizer.add_line_recognize_call(&file, 0, &spec.record_specs, Some("record2".to_string()));
        recognizer.add_line_recognize_call(&file, 1, &spec.record_specs, Some("record1".to_string()));
        padder.add_unpad_call(
            line1[0..3].to_string(),
            spec.record_specs.get(&"record2".to_string()).unwrap().field_specs.get(&"field1".to_string()).unwrap().padding.clone(),
            spec.record_specs.get(&"record2".to_string()).unwrap().field_specs.get(&"field1".to_string()).unwrap().padding_direction,
            Ok("field1_value".to_string())
        );
        padder.add_unpad_call(
            line1[4..8].to_string(),
            spec.record_specs.get(&"record2".to_string()).unwrap().field_specs.get(&"field2".to_string()).unwrap().padding.clone(),
            spec.record_specs.get(&"record2".to_string()).unwrap().field_specs.get(&"field2".to_string()).unwrap().padding_direction,
            Ok("field2_value".to_string())
        );
        padder.add_unpad_call(
            line1[9..36].to_string(),
            spec.record_specs.get(&"record2".to_string()).unwrap().field_specs.get(&"field3".to_string()).unwrap().padding.clone(),
            spec.record_specs.get(&"record2".to_string()).unwrap().field_specs.get(&"field3".to_string()).unwrap().padding_direction,
            Ok("field3_value".to_string())
        );
        padder.add_unpad_call(
            line1[37..45].to_string(),
            spec.record_specs.get(&"record2".to_string()).unwrap().field_specs.get(&"field4".to_string()).unwrap().padding.clone(),
            spec.record_specs.get(&"record2".to_string()).unwrap().field_specs.get(&"field4".to_string()).unwrap().padding_direction,
            Ok("field4_value".to_string())
        );
        padder.add_unpad_call(
            line2[0..4].to_string(),
            spec.record_specs.get(&"record1".to_string()).unwrap().field_specs.get(&"field1".to_string()).unwrap().padding.clone(),
            spec.record_specs.get(&"record1".to_string()).unwrap().field_specs.get(&"field1".to_string()).unwrap().padding_direction,
            Ok("field1_value2".to_string())
        );
        padder.add_unpad_call(
            line2[4..9].to_string(),
            spec.record_specs.get(&"record1".to_string()).unwrap().field_specs.get(&"field2".to_string()).unwrap().padding.clone(),
            spec.record_specs.get(&"record1".to_string()).unwrap().field_specs.get(&"field2".to_string()).unwrap().padding_direction,
            Ok("field2_value2".to_string())
        );
        padder.add_unpad_call(
            line2[9..45].to_string(),
            spec.record_specs.get(&"record1".to_string()).unwrap().field_specs.get(&"field3".to_string()).unwrap().padding.clone(),
            spec.record_specs.get(&"record1".to_string()).unwrap().field_specs.get(&"field3".to_string()).unwrap().padding_direction,
            Ok("field3_value2".to_string())
        );
        let reader = FileReader::new_with_recognizer_and_un_padder(&spec, recognizer, padder);
        let mut iterator = FileIterator::new(&reader, &file);
        let mut data = HashMap::new();
        data.insert("field1".to_string(), "field1_value".to_string());
        data.insert("field2".to_string(), "field2_value".to_string());
        data.insert("field3".to_string(), "field3_value".to_string());
        data.insert("field4".to_string(), "field4_value".to_string());
        assert_eq!(Some(data), iterator.next().map(|r| r.unwrap()));
        let mut data = HashMap::new();
        data.insert("field1".to_string(), "field1_value2".to_string());
        data.insert("field2".to_string(), "field2_value2".to_string());
        data.insert("field3".to_string(), "field3_value2".to_string());
        assert_eq!(Some(data), iterator.next().map(|r| r.unwrap()));
        assert!(iterator.next().is_none());
    }
}