use common::{File, Line, Range, FromField};
use spec::{FileSpec, RecordSpec, RecordSpecRecognizer};
use std::collections::HashMap;

#[derive(Debug)]
pub enum Error<T: File> {
    FailedToGetLine(T::Error),
    RecordSpecNotFound(String),
    FailedToRecognizeRecordSpec(String),
    RecordSpecNameRequired,
}

pub struct FileReader<'a, T: 'a + File, U: 'a + Range, V: 'a + RecordSpecRecognizer> {
    spec: &'a FileSpec<U>,
    file: &'a T,
    recognizer: Option<&'a V>
}

impl<'a, T: 'a + File, U: 'a + Range, V: 'a + RecordSpecRecognizer> FileReader<'a, T, U, V> {
    pub fn new(spec: &'a FileSpec<U>, file: &'a T, recognizer: Option<&'a V>) -> Self {
        FileReader {spec: spec, file: file, recognizer: recognizer}
    }

    pub fn get_line_reader(&self, index: usize, spec_name: Option<String>) -> Result<LineReader<'a, <T as File>::Line, U>, Error<T>> {
        let line = try!(self.file.line(index).map_err(Error::FailedToGetLine));
        let record_spec_name = try!(spec_name.map_or_else(
            || self.recognizer.ok_or(
                Error::RecordSpecNameRequired
            ).and_then(
                |recognizer| recognizer.recognize(line, self.spec).map_err(Error::FailedToRecognizeRecordSpec)
            ),
            |name| Ok(name))
        );

        Ok(LineReader::new(
            try!(self.spec.record_specs.get(
                &record_spec_name
            ).ok_or(Error::RecordSpecNotFound(record_spec_name))),
            line
        ))
    }
}

#[derive(Debug)]
pub enum LineError<T: Line> {
    FieldSpecNotFound {
        name: String,
        record_spec_name: String
    },
    LineGetFailed(T::Error),
    FromFieldFail(String)
}

pub struct LineReader<'a, T: 'a + Line, U: 'a + Range> {
    spec: &'a RecordSpec<U>,
    line: &'a T
}

impl<'a, T: 'a + Line, U: 'a + Range> LineReader<'a, T, U> {
    pub fn new(spec: &'a RecordSpec<U>, line: &'a T) -> Self {
        LineReader {spec: spec, line: line}
    }

    pub fn field<V: FromField>(&self, name: String) -> Result<V, LineError<T>> {
        V::from_field(try!(self.line.get(
            try!(self.spec.field_specs.get(&name).ok_or(LineError::FieldSpecNotFound { name: name, record_spec_name: self.spec.name.clone() })).range.clone()
        ).map_err(LineError::LineGetFailed))).map_err(LineError::FromFieldFail)
    }

    pub fn fields<V: FromField>(&self) -> HashMap<String, Result<V, LineError<T>>> {
        self.spec.field_specs.iter().map(|(name, field_spec)| (name.clone(), self.line.get(
            field_spec.range.clone()
        ).map_err(LineError::LineGetFailed).and_then(|v| V::from_field(v).map_err(LineError::FromFieldFail)))).collect()
    }
}
