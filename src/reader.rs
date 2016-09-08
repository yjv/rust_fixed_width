use common::{File, Line, Range, FromField};
use spec::{FileSpec, RecordSpec, LineRecordSpecRecognizer};
use std::collections::HashMap;

#[derive(Debug)]
pub enum Error<T: File, U: LineRecordSpecRecognizer> {
    FailedToGetLine(T::Error),
    RecordSpecNotFound(String),
    FailedToRecognizeRecordSpec(U::Error),
    RecordSpecNameRequired,
}

pub struct FileReader<'a, T: 'a + File, U: 'a + Range, V: 'a + LineRecordSpecRecognizer> {
    spec: &'a FileSpec<U>,
    file: &'a T,
    recognizer: Option<&'a V>
}

impl<'a, T: 'a + File, U: 'a + Range, V: 'a + LineRecordSpecRecognizer> FileReader<'a, T, U, V> {
    pub fn new(spec: &'a FileSpec<U>, file: &'a T, recognizer: Option<&'a V>) -> Self {
        FileReader {spec: spec, file: file, recognizer: recognizer}
    }

    pub fn get_line_reader(&self, index: usize, spec_name: Option<String>) -> Result<Option<LineReader<'a, <T as File>::Line, U>>, Error<T, V>> {
        let line = match self.file.line(index).map_err(Error::FailedToGetLine) {
            Ok(Some(line)) => line,
            Err(error) => return Err(error),
            Ok(None) => return Ok(None)
        };

        let record_spec_name = try!(spec_name.map_or_else(
            || self.recognizer.ok_or(
                Error::RecordSpecNameRequired
            ).and_then(
                |recognizer| recognizer.recognize_for_line(line, &self.spec.record_specs).map_err(Error::FailedToRecognizeRecordSpec)
            ),
            |name| Ok(name))
        );

        Ok(Some(LineReader::new(
            try!(self.spec.record_specs.get(
                &record_spec_name
            ).ok_or(Error::RecordSpecNotFound(record_spec_name))),
            line
        )))
    }
}

pub struct FileIterator<'a, T: 'a + File, U: 'a + Range, V: 'a + LineRecordSpecRecognizer> {
    position: usize,
    reader: &'a FileReader<'a, T, U, V>
}

impl<'a, T: 'a + File, U: 'a + Range, V: 'a + LineRecordSpecRecognizer> FileIterator<'a, T, U, V> {
    pub fn new(reader: &'a FileReader<'a, T, U, V>) -> Self {
        FileIterator {
            position: 0,
            reader: reader
        }
    }
}

impl<'a, T: 'a + File, U: 'a + Range, V: 'a + LineRecordSpecRecognizer> Iterator for FileIterator<'a, T, U, V> {
    type Item = Result<LineReader<'a, <T as File>::Line, U>, Error<T, V>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.position = self.position + 1;
        match self.reader.get_line_reader(self.position - 1, None) {
            Ok(Some(line)) => Some(Ok(line)),
            Err(error) => Some(Err(error)),
            Ok(None) => None
        }
    }
}

#[derive(Debug)]
pub enum LineError<T: Line, U: FromField> {
    FieldSpecNotFound {
        name: String,
        record_spec_name: String
    },
    LineGetFailed(T::Error),
    FromFieldFail(U::Error)
}

pub struct LineReader<'a, T: 'a + Line, U: 'a + Range> {
    spec: &'a RecordSpec<U>,
    line: &'a T
}

impl<'a, T: 'a + Line, U: 'a + Range> LineReader<'a, T, U> {
    pub fn new(spec: &'a RecordSpec<U>, line: &'a T) -> Self {
        LineReader {spec: spec, line: line}
    }

    pub fn field<V: FromField>(&self, name: String) -> Result<V, LineError<T, V>> {
        V::from_field(try!(self.line.get(
            try!(self.spec.field_specs.get(&name).ok_or(LineError::FieldSpecNotFound { name: name, record_spec_name: self.spec.name.clone() })).range.clone()
        ).map_err(LineError::LineGetFailed))).map_err(LineError::FromFieldFail)
    }

    pub fn fields<V: FromField>(&self) -> HashMap<String, Result<V, LineError<T, V>>> {
        self.spec.field_specs.iter().map(|(name, field_spec)| (name.clone(), self.line.get(
            field_spec.range.clone()
        ).map_err(LineError::LineGetFailed).and_then(|v| V::from_field(v).map_err(LineError::FromFieldFail)))).collect()
    }
}