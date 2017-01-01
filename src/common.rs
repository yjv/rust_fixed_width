use std::fmt::{Display, Error as FmtError, Formatter};
use spec::FileSpec;
use std::io::{Read, Error as IoError, Write, Seek, SeekFrom, ErrorKind};
use std::cmp::min;
use std::error::Error as ErrorTrait;

#[derive(Debug)]
pub enum Error {
    StringDoesntMatchLineSeparator(String, String),
    BufferOverflowsEndOfLine(usize, usize)
}

impl ErrorTrait for Error {
    fn description(&self) -> &str {
        match self {
            &Error::StringDoesntMatchLineSeparator(_, _) => "line separator was not the one expected",
            &Error::BufferOverflowsEndOfLine(_, _) => "the buffer given is larger than what remains until the end of the line"
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        match self {
            &Error::StringDoesntMatchLineSeparator(
                ref expected_line_separator,
                ref actual_line_separator
            ) => write!(f, "StringDoesntMatchLineSeparator: line separator was expected to be '{}' was actually '{}'", expected_line_separator, actual_line_separator),
            &Error::BufferOverflowsEndOfLine(
                ref buffer_length,
                ref bytes_to_end_of_line
            ) => write!(f, "BufferOverflowsEndOfLine: the buffer length {} is more than the {} bytes which are left until the end of the line", buffer_length, bytes_to_end_of_line),
        }
    }
}

//type Result<T> = ::std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub struct Position {
    pub position: usize,
    pub line: usize,
    pub column: usize
}

impl Position {
    pub fn new(position: usize, spec: &FileSpec) -> Self {
        let line_length = spec.line_length + spec.line_separator.len();
        Position {
            position: position,
            line: if position == 0 {
                0
            } else {
                position / line_length
            },
            column: if position == 0 {
                0
            } else {
                position % line_length
            }
        }
    }
}

pub struct Handler<'a, T> {
    inner: T,
    file_spec: &'a FileSpec,
    position: Position,
    end_of_line_validation: bool
}

impl <'a, T> Handler<'a, T> {
    pub fn get_ref(&self) -> &T { &self.inner }

    pub fn get_mut(&mut self) -> &mut T { &mut self.inner }

    pub fn into_inner(self) -> T { self.inner }

    pub fn get_position(&self) -> &Position {
        &self.position
    }
}

impl<'a, T: Read> Read for Handler<'a, T> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, IoError> {
        self.absorb_line_separator()?;
        let mut total_amount = 0;
        let length = buf.len();

        if self.end_of_line_validation && length > self.file_spec.line_length - self.position.column {
            return Err(IoError::new(ErrorKind::Other, Error::BufferOverflowsEndOfLine(length, self.file_spec.line_length - self.position.column)));
        }

        while total_amount < length {
            let remaining_amount = min(self.file_spec.line_length - self.position.column, buf.len() - total_amount);
            let amount = match self.inner.read(&mut buf[total_amount..total_amount + remaining_amount]) {
                Ok(0) => return Ok(total_amount),
                Ok(len) => len,
                Err(e) => return Err(e),
            };

            total_amount += amount;
            self.position = Position::new(self.position.position + amount, self.file_spec);
            self.absorb_line_separator()?;
        }

        Ok(total_amount)
    }
}

impl <'a, T: Read> Handler<'a, T> {
    fn absorb_line_separator(&mut self) -> Result<(), IoError> {
        if self.position.column >= self.file_spec.line_length {
            let mut line_separator = String::new();
            let read_length = self.file_spec.line_separator.len() - (self.position.column - self.file_spec.line_length);
            self.position = Position::new(
                self.position.position + self.inner.by_ref().take(read_length as u64).read_to_string(&mut line_separator)?,
                self.file_spec
            );
            let check_range = self.file_spec.line_separator.len() - read_length..self.file_spec.line_separator.len();
            if line_separator.len() != 0 && &line_separator[check_range.clone()] != &self.file_spec.line_separator[check_range] {
                return Err(IoError::new(ErrorKind::Other, Error::StringDoesntMatchLineSeparator(
                    self.file_spec.line_separator.clone(),
                    line_separator
                )));
            }
        }

        Ok(())
    }
}

impl<'a, T: Write> Write for Handler<'a, T> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, IoError> {
        self.write_line_separator()?;
        let mut total_amount = 0;
        let length = buf.len();

        if self.end_of_line_validation && length > self.file_spec.line_length - self.position.column {
            return Err(IoError::new(ErrorKind::Other, Error::BufferOverflowsEndOfLine(length, self.file_spec.line_length - self.position.column)));
        }

        while total_amount < length {
            let remaining_amount = min(self.file_spec.line_length - self.position.column, buf.len() - total_amount);
            let amount = match self.inner.write(&buf[total_amount..total_amount + remaining_amount]) {
                Ok(0) => return Ok(total_amount),
                Ok(len) => len,
                Err(e) => return Err(e),
            };

            total_amount += amount;
            self.position = Position::new(self.position.position + amount, self.file_spec);
            self.write_line_separator()?;
        }

        Ok(total_amount)
    }

    fn flush(&mut self) -> Result<(), IoError> {
        self.inner.flush()
    }
}


impl <'a, T: Write> Handler<'a, T> {
    fn write_line_separator(&mut self) -> Result<(), IoError> {
        if self.position.column >= self.file_spec.line_length {
            let write_length = self.file_spec.line_separator.len() - (self.position.column - self.file_spec.line_length);
            let write_range = self.file_spec.line_separator.len() - write_length..self.file_spec.line_separator.len();
            self.position = Position::new(
                self.position.position + self.inner.write((&self.file_spec.line_separator[write_range]).as_bytes())?,
                self.file_spec
            );
        }

        Ok(())
    }
}

impl <'a, T: Seek> Seek for Handler<'a, T> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, IoError> {
        self.inner.seek(pos)
    }
}

impl <'a, T: Seek> Handler<'a, T> {
    pub fn seek_to_position(&mut self, position: Position) -> Result<Position, IoError> {
        let current_position = self.inner.seek(SeekFrom::Current(0))?;
        self.inner.seek(SeekFrom::Current(position.position as i64 - current_position as i64))?;
        self.position = position;
        Ok(self.position.clone())
    }
}

#[cfg(test)]
mod test {
    use std::string::ToString;
    use super::super::test::*;
}