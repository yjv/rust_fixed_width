use std::collections::HashMap;
use spec::RecordSpec;

pub trait LineRecordSpecRecognizer {
    fn recognize_for_line(&self, line: &String, record_specs: &HashMap<String, RecordSpec>) -> Option<String>;
}

impl<'a, V> LineRecordSpecRecognizer for &'a V where V: 'a + LineRecordSpecRecognizer {
    fn recognize_for_line(&self, line: &String, record_specs: &HashMap<String, RecordSpec>) -> Option<String> {
        (**self).recognize_for_line(line, record_specs)
    }
}

pub trait DataRecordSpecRecognizer {
    fn recognize_for_data(&self, data: &HashMap<String, String>, record_specs: &HashMap<String, RecordSpec>) -> Option<String>;
}

impl<'a, U> DataRecordSpecRecognizer for &'a U where U: 'a + DataRecordSpecRecognizer {
    fn recognize_for_data(&self, data: &HashMap<String, String>, record_specs: &HashMap<String, RecordSpec>) -> Option<String> {
        (**self).recognize_for_data(data, record_specs)
    }
}

pub struct IdFieldRecognizer {
    id_field: String
}

impl IdFieldRecognizer {
    pub fn new() -> Self {
        Self::new_with_field("$id")
    }

    pub fn new_with_field<T: Into<String>>(id_field: T) -> Self {
        IdFieldRecognizer { id_field: id_field.into() }
    }
}

impl LineRecordSpecRecognizer for IdFieldRecognizer {
    fn recognize_for_line(&self, line: &String, record_specs: &HashMap<String, RecordSpec>) -> Option<String> {
        for (name, record_spec) in record_specs.iter() {
            if let Some(ref field_spec) = record_spec.field_specs.get(&self.id_field) {
                if let Some(ref default) = field_spec.default {
                    if &line[field_spec.index..field_spec.index + field_spec.length] == default {
                        return Some(name.clone());
                    }
                }
            }
        }

        None
    }
}

impl DataRecordSpecRecognizer for IdFieldRecognizer {
    fn recognize_for_data(&self, data: &HashMap<String, String>, record_specs: &HashMap<String, RecordSpec>) -> Option<String> {
        for (name, record_spec) in record_specs.iter() {
            if let Some(ref field_spec) = record_spec.field_specs.get(&self.id_field) {
                if let Some(ref default) = field_spec.default {
                    if let Some(string) = data.get(&self.id_field) {
                        if string == default {
                            return Some(name.clone());
                        }
                    }
                }
            }
        }

        None
    }
}

pub struct NoneRecognizer;

impl LineRecordSpecRecognizer for NoneRecognizer {
    fn recognize_for_line(&self, _: &String, _: &HashMap<String, RecordSpec>) -> Option<String> {
        None
    }
}

impl DataRecordSpecRecognizer for NoneRecognizer {
    fn recognize_for_data(&self, _: &HashMap<String, String>, _: &HashMap<String, RecordSpec>) -> Option<String> {
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use spec::*;
    use std::collections::{HashMap, BTreeMap};
    use super::super::test::test_spec;

    #[test]
    fn none_recognizer() {
        let recognizer = NoneRecognizer;
        assert_eq!(None, recognizer.recognize_for_data(&HashMap::new(), &HashMap::new()));
        assert_eq!(None, recognizer.recognize_for_line(
            &String::new(),
            &HashMap::new()
        ));
    }

    #[test]
    fn id_spec_recognizer() {
        let specs = FileSpecBuilder::new()
            .with_record(
                "record1",
                RecordSpecBuilder::new(LineSpecBuilder::new().with_length(10))
                    .with_field(
                        "field1",
                        FieldSpecBuilder::new()
                            .with_default("foo")
                            .with_range(0..3)
                            .with_padding("dsasd")
                            .with_padding_direction(PaddingDirection::Left)
                    )
                    .with_field(
                        "field2",
                        FieldSpecBuilder::new()
                            .with_range(4..9)
                            .with_padding("sdf".to_string())
                            .with_padding_direction(PaddingDirection::Right)
                    )
            )
            .with_record(
                "record2",
                RecordSpecBuilder::new(LineSpecBuilder::new().with_length(10))
                    .with_field(
                        "$id",
                        FieldSpecBuilder::new_string()
                            .with_default("bar")
                            .with_range(0..3)
                    )
                    .with_field(
                        "field2".to_string(),
                        FieldSpecBuilder::new_string()
                            .with_range(4..9)
                    )
            ).with_record(
            "record3",
            RecordSpecBuilder::new(LineSpecBuilder::new().with_length(10))
                .with_field(
                    "field1",
                    FieldSpecBuilder::new_string()
                        .with_default("bar")
                        .with_range(0..3)
                )
                .with_field(
                    "field2",
                    FieldSpecBuilder::new_string()
                        .with_range(4..9)
                )
        )
            .with_record(
                "record4",
                RecordSpecBuilder::new(LineSpecBuilder::new().with_length(10))
                    .with_field(
                        "$id",
                        FieldSpecBuilder::new_string()
                            .with_default("foo")
                            .with_range(0..3)
                    )
                    .with_field(
                        "field2".to_string(),
                        FieldSpecBuilder::new_string()
                            .with_range(4..9)
                    )
            )
            .build()
            .record_specs
        ;
        let recognizer = IdFieldRecognizer::new();
        let recognizer_with_field = IdFieldRecognizer::new_with_field("field1");
        let mut data = HashMap::new();

        data.insert("$id".to_string(), "bar".to_string());
        assert_eq!(Some("record2".to_string()), recognizer.recognize_for_data(&data, &specs));
        assert_eq!(None, recognizer_with_field.recognize_for_data(&data, &specs));

        data.insert("$id".to_string(), "foo".to_string());
        assert_eq!(Some("record4".to_string()), recognizer.recognize_for_data(&data, &specs));
        assert_eq!(None, recognizer_with_field.recognize_for_data(&data, &specs));

        data.insert("$id".to_string(), "foobar".to_string());
        assert_eq!(None, recognizer.recognize_for_data(&data, &specs));
        assert_eq!(None, recognizer_with_field.recognize_for_data(&data, &specs));

        data.insert("field1".to_string(), "bar".to_string());
        assert_eq!(None, recognizer.recognize_for_data(&data, &specs));
        assert_eq!(Some("record3".to_string()), recognizer_with_field.recognize_for_data(&data, &specs));
        data.remove(&"$id".to_string());

        assert_eq!(None, recognizer.recognize_for_data(&data, &specs));
        assert_eq!(Some("record3".to_string()), recognizer_with_field.recognize_for_data(&data, &specs));

        data.insert("field1".to_string(), "foo".to_string());
        assert_eq!(None, recognizer.recognize_for_data(&data, &specs));
        assert_eq!(Some("record1".to_string()), recognizer_with_field.recognize_for_data(&data, &specs));

        data.insert("field1".to_string(), "foobar".to_string());
        assert_eq!(None, recognizer.recognize_for_data(&data, &specs));
        assert_eq!(None, recognizer_with_field.recognize_for_data(&data, &specs));

        assert_eq!(None, recognizer.recognize_for_line(&"dsfdsfsdfd".to_string(), &specs));
        assert_eq!(None, recognizer_with_field.recognize_for_line(&"dsfdsfsdfd".to_string(), &specs));
        assert_eq!(Some("record2".to_string()), recognizer.recognize_for_line(&"barasdasdd".to_string(), &specs));
        assert_eq!(Some("record3".to_string()), recognizer_with_field.recognize_for_line(&"barasdasdd".to_string(), &specs));
        assert_eq!(Some("record4".to_string()), recognizer.recognize_for_line(&"foodsfsdfd".to_string(), &specs));
        assert_eq!(Some("record1".to_string()), recognizer_with_field.recognize_for_line(&"foodsfsdfd".to_string(), &specs));
    }

    #[test]
    fn recognizer_reference() {
        let recognizer = NoneRecognizer;
        assert_eq!(None, DataRecordSpecRecognizer::recognize_for_data(&&recognizer, &HashMap::new(), &HashMap::new()));
        assert_eq!(None, LineRecordSpecRecognizer::recognize_for_line(
            &&recognizer,
            &String::new(),
            &HashMap::new()
        ));
    }
}