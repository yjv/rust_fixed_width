use common::{File, Line, Range};
use spec::{FileSpec, RecordSpec, LineRecordSpecRecognizer, NoneRecognizer};
use std::collections::HashMap;

#[derive(Debug)]
pub enum Error<T: File> {
    FailedToGetLine(T::Error),
    RecordSpecNotFound(String),
    RecordSpecNameRequired,
    LineGetFailed(<T::Line as Line>::Error),
    FieldSpecNotFound(String)
}

pub struct FileReader<'a, T: 'a + File, U: 'a + Range, V: LineRecordSpecRecognizer> {
    spec: &'a FileSpec<U>,
    file: &'a T,
    recognizer: V
}

impl<'a, T: 'a + File, U: 'a + Range, V: 'a + LineRecordSpecRecognizer> FileReader<'a, T, U, V> {
    pub fn new(spec: &'a FileSpec<U>, file: &'a T) -> FileReader<'a, T, U, NoneRecognizer> {
        FileReader { spec: spec, file: file, recognizer: NoneRecognizer }
    }

    pub fn new_with_recognizer(spec: &'a FileSpec<U>, file: &'a T, recognizer: V) -> Self {
        FileReader {spec: spec, file: file, recognizer: recognizer}
    }

    pub fn get_line_reader(&self, index: usize, spec_name: Option<String>) -> Result<Option<LineReader<'a, <T as File>::Line, U>>, Error<T>> {
        let line = match self.file.line(index).map_err(Error::FailedToGetLine) {
            Ok(Some(line)) => line,
            Err(error) => return Err(error),
            Ok(None) => return Ok(None)
        };

        let record_spec_name = try!(spec_name.or_else(|| self.recognizer.recognize_for_line(line, &self.spec.record_specs)).ok_or(Error::RecordSpecNameRequired));

        Ok(Some(LineReader::new(
            try!(self.spec.record_specs.get(
                &record_spec_name
            ).ok_or(Error::RecordSpecNotFound(record_spec_name))),
            line
        )))
    }

    pub fn field(&self, index: usize, name: String, spec_name: Option<String>) -> Result<Option<String>, Error<T>> {
        let line = match self.file.line(index).map_err(Error::FailedToGetLine) {
            Ok(Some(line)) => line,
            Err(error) => return Err(error),
            Ok(None) => return Ok(None)
        };

        let record_spec_name = try!(spec_name.or_else(|| self.recognizer.recognize_for_line(line, &self.spec.record_specs)).ok_or(Error::RecordSpecNameRequired));

        let record_spec = try!(self.spec.record_specs.get(
            &record_spec_name
        ).ok_or(Error::RecordSpecNotFound(record_spec_name)));

        Ok(try!(line.get(
            try!(record_spec.field_specs.get(&name).ok_or(Error::FieldSpecNotFound(name))).range.clone()
        ).map_err(Error::LineGetFailed)))
    }

    pub fn fields(&self, index: usize, spec_name: Option<String>) -> Result<Option<HashMap<String, Result<String, Error<T>>>>, Error<T>> {
        let line = match self.file.line(index).map_err(Error::FailedToGetLine) {
            Ok(Some(line)) => line,
            Err(error) => return Err(error),
            Ok(None) => return Ok(None)
        };

        let record_spec_name = try!(spec_name.or_else(|| self.recognizer.recognize_for_line(line, &self.spec.record_specs)).ok_or(Error::RecordSpecNameRequired));
        let record_spec = try!(self.spec.record_specs.get(
            &record_spec_name
        ).ok_or(Error::RecordSpecNotFound(record_spec_name)));
        Ok(Some(record_spec.field_specs.iter().map(|(name, field_spec)| (name.clone(), line.get(
            field_spec.range.clone()
        ).map_err(Error::LineGetFailed))).collect()))
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
    type Item = Result<LineReader<'a, <T as File>::Line, U>, Error<T>>;

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
pub enum LineError<T: Line> {
    FieldSpecNotFound(String),
    LineGetFailed(T::Error),
}

pub struct LineReader<'a, T: 'a + Line, U: 'a + Range> {
    spec: &'a RecordSpec<U>,
    line: &'a T
}

impl<'a, T: 'a + Line, U: 'a + Range> LineReader<'a, T, U> {
    pub fn new(spec: &'a RecordSpec<U>, line: &'a T) -> Self {
        LineReader {spec: spec, line: line}
    }

    pub fn field(&self, name: String) -> Result<String, LineError<T>> {
        Ok(try!(self.line.get(
            try!(self.spec.field_specs.get(&name).ok_or(LineError::FieldSpecNotFound(name))).range.clone()
        ).map_err(LineError::LineGetFailed)))
    }

    pub fn fields(&self) -> HashMap<String, Result<String, LineError<T>>> {
        self.spec.field_specs.iter().map(|(name, field_spec)| (name.clone(), self.line.get(
            field_spec.range.clone()
        ).map_err(LineError::LineGetFailed))).collect()
    }
}

#[cfg(test)]
mod test {

    #[test]
    fn ranges_work() {
    }
}