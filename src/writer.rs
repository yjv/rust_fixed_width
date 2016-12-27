use common::{File, MutableFile};
use spec::{FileSpec, DataRecordSpecRecognizer, LineRecordSpecRecognizer, NoneRecognizer, Padder, IdentityPadder};
use std::collections::HashMap;

#[derive(Debug)]
pub enum Error<T: File, U: Padder> {
    FailedToAddLine(T::Error),
    RecordSpecNameRequired,
    FailedToSetData(T::Error),
    FailedToGetLine(T::Error),
    RecordSpecNotFound(String),
    PaddingFailed(U::Error)
}

pub struct FileWriter<'a, T: DataRecordSpecRecognizer, U: LineRecordSpecRecognizer, V: Padder> {
    spec: &'a FileSpec,
    data_recognizer: T,
    line_recognizer: U,
    padder: V
}

impl<'a, T: DataRecordSpecRecognizer, U: LineRecordSpecRecognizer, V: Padder> FileWriter<'a, T, U, V> {
    pub fn new(spec: &'a FileSpec) -> FileWriter<'a, NoneRecognizer, NoneRecognizer, IdentityPadder> {
        FileWriter { spec: spec, data_recognizer: NoneRecognizer, line_recognizer: NoneRecognizer, padder: IdentityPadder }
    }

    pub fn new_with_recognizers_and_padder(spec: &'a FileSpec, data_recognizer: T, line_recognizer: U, padder: V) -> Self {
        FileWriter { spec: spec, data_recognizer: data_recognizer, line_recognizer: line_recognizer, padder: padder }
    }

    pub fn set_fields<W: MutableFile>(&'a mut self, file: &mut W, index: usize, data: &HashMap<String, String>, spec_name: Option<String>) -> Result<&'a mut Self, Error<W, V>> {
        let record_spec_name = try!(
            spec_name
                .or_else(|| self.data_recognizer.recognize_for_data(data, &self.spec.record_specs))
                .or_else(|| self.line_recognizer.recognize_for_line(file, index, &self.spec.record_specs))
                .ok_or(Error::RecordSpecNameRequired)
        );
        let record_spec = try!(self.spec.record_specs.get(&record_spec_name).ok_or(Error::RecordSpecNotFound(record_spec_name)));

        for (name, field_spec) in &record_spec.field_specs {
            if let Some(value) = data.get(name).or(field_spec.default.as_ref()) {
                let value = try!(self.padder.pad(value, field_spec.range.end - field_spec.range.start, &field_spec.padding, field_spec.padding_direction).map_err(Error::PaddingFailed));
                try!(file.set(
                    index,
                    field_spec.range.start,
                    &value
                ).map_err(Error::FailedToSetData));
            }
        }

        Ok(self)
    }

    pub fn set_field<W: MutableFile>(&'a mut self, file: &mut W, index: usize, key: String, value: String, spec_name: Option<String>) -> Result<&'a mut Self, Error<W, V>> {
        let mut data = HashMap::new();
        data.insert(key, value);
        self.set_fields(file, index, &data, spec_name)
    }
}
//
//pub struct FileWriterBuilder {
//    spec: Option<&'a FileSpec>,
//    data_recognizer: Option<U>,
//    line_recognizer: Option<V>,
//    padder: Option<W>
//}

#[cfg(test)]
mod test {

//    use super::*;
//    use super::super::spec::*;
//    use super::super::in_memory::*;
//    use std::collections::HashMap;
//
//    #[test]
//    fn building() {
//        let spec = test_spec();
//        let line1 = repeat("line1").take(9).collect();
//        let line2 = repeat("line2").take(9).collect();
//        let line3 = repeat("line3").take(9).collect();
//        let line4 = repeat("line4").take(9).collect();
//        let mut file = MockFile::new(45, Some(vec![
//        &line1,
//        &line2,
//        &line3,
//        &line4
//        ]));
//        file.add_read_error(1);
//        let mut recognizer = MockRecognizer::new();
//        let mut padder = MockPadder::new();
//        recognizer.add_line_recognize_call(&file, 0, &spec.record_specs, Some("record2".to_string()));
//        recognizer.add_line_recognize_call(&file, 1, &spec.record_specs, None);
//        recognizer.add_line_recognize_call(&file, 3, &spec.record_specs, Some("record1".to_string()));
//        padder.add_unpad_call(
//            line1[0..3].to_string(),
//            spec.record_specs.get(&"record2".to_string()).unwrap().field_specs.get(&"field1".to_string()).unwrap().padding.clone(),
//            spec.record_specs.get(&"record2".to_string()).unwrap().field_specs.get(&"field1".to_string()).unwrap().padding_direction,
//            Ok("field1_value".to_string())
//        );
//        padder.add_unpad_call(
//            line1[4..8].to_string(),
//            spec.record_specs.get(&"record2".to_string()).unwrap().field_specs.get(&"field2".to_string()).unwrap().padding.clone(),
//            spec.record_specs.get(&"record2".to_string()).unwrap().field_specs.get(&"field2".to_string()).unwrap().padding_direction,
//            Ok("field2_value".to_string())
//        );
//        padder.add_unpad_call(
//            line1[9..36].to_string(),
//            spec.record_specs.get(&"record2".to_string()).unwrap().field_specs.get(&"field3".to_string()).unwrap().padding.clone(),
//            spec.record_specs.get(&"record2".to_string()).unwrap().field_specs.get(&"field3".to_string()).unwrap().padding_direction,
//            Ok("field3_value".to_string())
//        );
//        padder.add_unpad_call(
//            line1[37..45].to_string(),
//            spec.record_specs.get(&"record2".to_string()).unwrap().field_specs.get(&"field4".to_string()).unwrap().padding.clone(),
//            spec.record_specs.get(&"record2".to_string()).unwrap().field_specs.get(&"field4".to_string()).unwrap().padding_direction,
//            Ok("field4_value".to_string())
//        );
//        padder.add_unpad_call(
//            line2[0..4].to_string(),
//            spec.record_specs.get(&"record1".to_string()).unwrap().field_specs.get(&"field1".to_string()).unwrap().padding.clone(),
//            spec.record_specs.get(&"record1".to_string()).unwrap().field_specs.get(&"field1".to_string()).unwrap().padding_direction,
//            Err(())
//        );
//        padder.add_unpad_call(
//            line2[4..9].to_string(),
//            spec.record_specs.get(&"record1".to_string()).unwrap().field_specs.get(&"field2".to_string()).unwrap().padding.clone(),
//            spec.record_specs.get(&"record1".to_string()).unwrap().field_specs.get(&"field2".to_string()).unwrap().padding_direction,
//            Err(())
//        );
//        padder.add_unpad_call(
//            line2[9..45].to_string(),
//            spec.record_specs.get(&"record1".to_string()).unwrap().field_specs.get(&"field3".to_string()).unwrap().padding.clone(),
//            spec.record_specs.get(&"record1".to_string()).unwrap().field_specs.get(&"field3".to_string()).unwrap().padding_direction,
//            Err(())
//        );
//        let writer = FileWriter::new_with_recognizers_and_padder(&spec, recognizer.clone(), recognizer, padder);
//        let mut data = HashMap::new();
//        data.insert("field1".to_string(), "field1_value".to_string());
//        data.insert("field2".to_string(), "field2_value".to_string());
//        data.insert("field3".to_string(), "field3_value".to_string());
//        data.insert("field4".to_string(), "field4_value".to_string());
//        assert_eq!(data, writer.fields(&file, 0, None).unwrap());
//        match writer.fields(&file, 1, None).unwrap_err() {
//            Error::RecordSpecNameRequired => (),
//            e => panic!("did not return correct error {:?}", e)
//        }
//        match writer.fields(&file, 1, Some("record5".to_string())).unwrap_err() {
//            Error::RecordSpecNotFound(name) => assert_eq!("record5".to_string(), name),
//            e => panic!("did not return correct error {:?}", e)
//        }
//        match writer.fields(&file, 1, Some("record2".to_string())).unwrap_err() {
//            Error::GetFailed(field, ()) => assert!(
//            field == "field1".to_string()
//                || field == "field2".to_string()
//                || field == "field3".to_string()
//                || field == "field4".to_string()
//            ),
//            e => panic!("did not return correct error {:?}", e)
//        }
//        match writer.fields(&file, 2, Some("record1".to_string())).unwrap_err() {
//            Error::UnPaddingFailed(()) => (),
//            e => panic!("did not return correct error {:?}", e)
//        }
//
//        assert_eq!("field2_value".to_string(), writer.field(&file, 0, "field2".to_string(), None).unwrap());
//        match writer.field(&file, 1, "field1".to_string(), None).unwrap_err() {
//            Error::RecordSpecNameRequired => (),
//            e => panic!("did not return correct error {:?}", e)
//        }
//        match writer.field(&file, 1, "field1".to_string(), Some("record5".to_string())).unwrap_err() {
//            Error::RecordSpecNotFound(name) => assert_eq!("record5".to_string(), name),
//            e => panic!("did not return correct error {:?}", e)
//        }
//        match writer.field(&file, 1, "field3".to_string(), Some("record2".to_string())).unwrap_err() {
//            Error::GetFailed(field, ()) => assert_eq!(field, "field3".to_string()),
//            e => panic!("did not return correct error {:?}", e)
//        }
//        match writer.field(&file, 2, "field2".to_string(), Some("record1".to_string())).unwrap_err() {
//            Error::UnPaddingFailed(()) => (),
//            e => panic!("did not return correct error {:?}", e)
//        }
//    }
}