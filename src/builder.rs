use common::{File, Line, ToField};
use spec::{FileSpec, RecordSpec};
use std::collections::HashMap;

#[derive(Debug)]
pub enum Error<T: File> {
    RecordSpecNotFound(String),
    LineAddFailed(T::Error),
}

pub struct FileBuilder<'a, T: File> {
    pub file: T,
    spec: &'a FileSpec
}

impl<'a, T: File> FileBuilder<'a, T> {
    pub fn new(file: T, spec: &'a FileSpec) -> Self {
        FileBuilder { file: file, spec: spec }
    }

    pub fn add_line(&'a mut self, spec_name: String) -> Result<LineBuilder<'a, T::Line>, Error<T>> {
        let record_spec = try!(self.spec.record_specs.get(&spec_name).ok_or(Error::RecordSpecNotFound(spec_name)));
        let index = try!(self.file.add_line().map_err(Error::LineAddFailed));

        Ok(LineBuilder {
            line: try!(self.file.line_mut(index).map_err(Error::LineAddFailed)).expect("Line just added couldn't be retrieved. This shouldn't be possible."),
            spec: record_spec
        })
    }
}

#[derive(Debug)]
pub enum LineError<T: Line, U: ToField> {
    FieldSpecNotFound {
        name: String,
        record_spec_name: String
    },
    LineSetFailed(T::Error),
    ToFieldFail(U::Error)
}

pub struct LineBuilder<'a, T: 'a + Line> {
    pub line: &'a mut T,
    spec: &'a RecordSpec
}

impl <'a, T: 'a + Line> LineBuilder<'a, T> {
    pub fn set_field<'b, U: 'b + ToField>(&mut self, name: String, value: &'b U) -> Result<(), LineError<T, U>> {
        let field_spec = try!(self.spec.field_specs.get(&name).ok_or(LineError::FieldSpecNotFound {name: name, record_spec_name: self.spec.name.clone()}));
        try!(self.line.set(field_spec.range.clone(), &try!(value.to_field().map_err(LineError::ToFieldFail))).map_err(LineError::LineSetFailed));
        Ok(())
    }
}