use common::{File, Line, LineGenerator, ToField};
use spec::FileSpec;
use std::collections::HashMap;

#[derive(Debug)]
pub enum Error<T: LineGenerator, U: Line, V: ToField> {
    LineGenerateFailed(T::Error),
    RecordSpecNotFound(String),
    FieldSpecNotFound {
        name: String,
        record_spec_name: String
    },
    LineSetFailed(U::Error),
    ToFieldFail(V::Error)
}

pub struct FileBuilder<'a, 'b, T: File, U: 'b + LineGenerator> {
    pub file: T,
    spec: &'a FileSpec,
    line_generator: &'b U
}

impl<'a, 'b, T: File, U: 'b + LineGenerator> FileBuilder<'a, 'b, T, U> {
    pub fn new(file: T, spec: &'a FileSpec, line_generator: &'b U) -> Self {
        FileBuilder { file: file, spec: spec, line_generator: line_generator }
    }

    pub fn add_record<'c, V: ToField>(&mut self, data: HashMap<String, &'c V>, record_spec_name: String) -> Result<(), Error<U, U::Line, V>> {
        let mut line = try!(self.line_generator.generate_line(self.file.width()).map_err(|e| Error::LineGenerateFailed(e)));
        let record_spec = try!(self.spec.record_specs.get(&record_spec_name).ok_or(Error::RecordSpecNotFound(record_spec_name)));

        for (name, value) in data {
            let field_spec = try!(record_spec.field_specs.get(&name).ok_or(Error::FieldSpecNotFound {name: name, record_spec_name: record_spec.name.clone()}));
            try!(line.set(field_spec.range.clone(), &try!(value.to_field().map_err(Error::ToFieldFail))).map_err(Error::LineSetFailed));
        }

        Ok(())
    }
}