pub struct FileBuilder<'a, T: File> {
    pub file: T,
    spec: &'a FileSpec
}

#[derive(Debug)]
pub enum FileBuilderError<T: File> {
    LineGenerateFailed(T::Error),
    RecordSpecNotFound(String),
    FieldSpecNotFound {
        name: String,
        record_spec_name: String
    },
    LineSetFailed(<T::Line as Line>::Error),
    ToFieldFail(String)
}

impl<'a, T: File> FileBuilder<'a, T> {
    pub fn new(file: T, spec: &'a FileSpec) -> Self {
        FileBuilder { file: file, spec: spec }
    }

    pub fn add_record<'b, U: ToField>(&mut self, data: HashMap<String, &'b U>, record_spec_name: String) -> Result<(), FileBuilderError<T>> {
        let mut line = try!(self.file.generate_line().map_err(|e| FileBuilderError::LineGenerateFailed(e)));
        let record_spec = try!(self.spec.record_specs.get(&record_spec_name).ok_or(FileBuilderError::RecordSpecNotFound(record_spec_name)));

        for (name, value) in data {
            let field_spec = try!(record_spec.field_specs.get(&name).ok_or(FileBuilderError::FieldSpecNotFound {name: name, record_spec_name: record_spec.name.clone()}));
            try!(line.set(field_spec.range.clone(), &try!(value.to_field().map_err(FileBuilderError::ToFieldFail))).map_err(FileBuilderError::LineSetFailed));
        }

        Ok(())
    }
}