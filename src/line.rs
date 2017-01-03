use std::fmt::{Display, Error as FmtError, Formatter};
use spec::LineSpec;
use std::io::{Read, Error as IoError, Write, Seek, SeekFrom, ErrorKind};
use std::cmp::min;
use std::error::Error as ErrorTrait;
use std::borrow::Borrow;

type Result<T> = ::std::result::Result<T, IoError>;

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
    fn fmt(&self, f: &mut Formatter) -> ::std::result::Result<(), FmtError> {
        match self {
            &Error::StringDoesntMatchLineSeparator(
                ref expected_separator,
                ref actual_separator
            ) => write!(f, "StringDoesntMatchLineSeparator: line separator was expected to be '{}' was actually '{}'", expected_separator, actual_separator),
            &Error::BufferOverflowsEndOfLine(
                ref buffer_length,
                ref bytes_to_end_of_line
            ) => write!(f, "BufferOverflowsEndOfLine: the buffer length {} is more than the {} bytes which are left until the end of the line", buffer_length, bytes_to_end_of_line),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Position {
    line_length: usize,
    position: usize,
    line: usize,
    column: usize
}

impl Position {
    pub fn new(position: usize, line_length: usize) -> Self {
        Position {
            line_length: line_length,
            position: position,
            line: if line_length == 0 {
                0
            } else {
                position / line_length
            },
            column: if line_length == 0 {
                0
            } else {
                position % line_length
            }
        }
    }

    pub fn add(&self, amount: usize) -> Self {
        Self::new(self.position + amount, self.line_length)
    }

    pub fn get_position(&self) -> usize {
        self.position
    }

    pub fn get_line(&self) -> usize {
        self.line
    }

    pub fn get_column(&self) -> usize {
        self.column
    }
}

impl From<(usize, usize)> for Position {
    fn from(tuple: (usize, usize)) -> Self {
        Position::new(
            tuple.0,
            tuple.1
        )
    }
}

impl Into<(usize, usize)> for Position {
    fn into(self) -> (usize, usize) {
        (self.position, self.line_length)
    }
}

pub struct Handler<T, U: Borrow<LineSpec>> {
    inner: T,
    line_spec: U,
    position: Position,
    end_of_line_validation: bool
}

impl <T, U: Borrow<LineSpec>> Handler<T, U> {
    pub fn get_ref(&self) -> &T { &self.inner }

    pub fn get_mut(&mut self) -> &mut T { &mut self.inner }

    pub fn into_inner(self) -> T { self.inner }

    pub fn get_position(&self) -> &Position {
        &self.position
    }
}

impl<T: Read, U: Borrow<LineSpec>> Read for Handler<T, U> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.absorb_separator()?;
        let mut total_amount = 0;
        let length = buf.len();

        if self.end_of_line_validation && length > self.line_spec.borrow().length - self.position.column {
            return Err(IoError::new(ErrorKind::Other, Error::BufferOverflowsEndOfLine(length, self.line_spec.borrow().length - self.position.column)));
        }

        while total_amount < length {
            let remaining_amount = min(self.line_spec.borrow().length - self.position.column, buf.len() - total_amount);
            let amount = match self.inner.read(&mut buf[total_amount..total_amount + remaining_amount]) {
                Ok(0) => return Ok(total_amount),
                Ok(len) => len,
                Err(e) => return Err(e),
            };

            total_amount += amount;
            self.position = self.position.add(amount);
            self.absorb_separator()?;
        }

        Ok(total_amount)
    }
}

impl<T: Read, U: Borrow<LineSpec>> Handler<T, U> {
    fn absorb_separator(&mut self) -> Result<()> {
        if self.position.column >= self.line_spec.borrow().length {
            let mut separator = String::new();
            let read_length = self.line_spec.borrow().separator.len() - (self.position.column - self.line_spec.borrow().length);
            self.position = self.position.add(self.inner.by_ref().take(read_length as u64).read_to_string(&mut separator)?);
            let check_range = self.line_spec.borrow().separator.len() - read_length..self.line_spec.borrow().separator.len();
            if separator.len() != 0 && &separator[..] != &self.line_spec.borrow().separator[check_range.clone()] {
                return Err(IoError::new(ErrorKind::Other, Error::StringDoesntMatchLineSeparator(
                    self.line_spec.borrow().separator[check_range.clone()].to_string(),
                    separator
                )));
            }
        }

        Ok(())
    }
}

impl<T: Write, U: Borrow<LineSpec>> Write for Handler<T, U> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.write_separator()?;
        let mut total_amount = 0;
        let length = buf.len();

        if self.end_of_line_validation && length > self.line_spec.borrow().length - self.position.column {
            return Err(IoError::new(ErrorKind::Other, Error::BufferOverflowsEndOfLine(length, self.line_spec.borrow().length - self.position.column)));
        }

        while total_amount < length {
            let remaining_amount = min(self.line_spec.borrow().length - self.position.column, buf.len() - total_amount);
            let amount = match self.inner.write(&buf[total_amount..total_amount + remaining_amount]) {
                Ok(0) => return Ok(total_amount),
                Ok(len) => len,
                Err(e) => return Err(e),
            };

            total_amount += amount;
            self.position = self.position.add(amount);
            self.write_separator()?;
        }

        Ok(total_amount)
    }

    fn flush(&mut self) -> Result<()> {
        self.inner.flush()
    }
}


impl <T: Write, U: Borrow<LineSpec>> Handler<T, U> {
    fn write_separator(&mut self) -> Result<()> {
        if self.position.column >= self.line_spec.borrow().length {
            let write_length = self.line_spec.borrow().separator.len() - (self.position.column - self.line_spec.borrow().length);
            let write_range = self.line_spec.borrow().separator.len() - write_length..self.line_spec.borrow().separator.len();
            self.position = self.position.add(self.inner.write((&self.line_spec.borrow().separator[write_range]).as_bytes())?);
        }

        Ok(())
    }
}

impl <T: Seek, U: Borrow<LineSpec>> Seek for Handler<T, U> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        self.position = Position::new(
            self.inner.seek(pos)? as usize,
            self.line_spec.borrow().len()
        );
        Ok(self.position.get_position() as u64)
    }
}

#[derive(Clone)]
pub struct HandlerBuilder<T, U: Borrow<LineSpec>> {
    inner: Option<T>,
    line_spec: Option<U>,
    position: Option<Position>,
    end_of_line_validation: bool
}

impl<'a, T, U: Borrow<LineSpec>> HandlerBuilder<T, U> {
    pub fn new() -> Self {
        HandlerBuilder {
            inner: None,
            line_spec: None,
            position: None,
            end_of_line_validation: true
        }
    }

    pub fn with_inner(mut self, inner: T) -> Self {
        self.inner = Some(inner);
        self
    }

    pub fn with_line_spec(mut self, line_spec: U) -> Self {
        self.line_spec = Some(line_spec);
        self
    }

    pub fn with_position<V: Into<Position>>(mut self, position: V) -> Self {
        self.position = Some(position.into());
        self
    }

    pub fn without_end_of_line_validation(mut self) -> Self {
        self.end_of_line_validation = false;
        self
    }

    pub fn build(self) -> Handler<T, U> {
        let line_spec = self.line_spec.expect("line_spec is required in order to build");
        let line_length = line_spec.borrow().len();
        Handler {
            inner: self.inner.expect("inner is required in order to build"),
            line_spec: line_spec,
            position: self.position.unwrap_or_else(|| Position {
                position: 0,
                line: 0,
                column: 0,
                line_length: line_length
            }),
            end_of_line_validation: self.end_of_line_validation
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use spec::LineSpec;
    use std::io::{Read, Write, Seek, SeekFrom, Cursor};

    #[test]
    pub fn read() {
        let spec = LineSpec {
            length: 10,
            separator: "h\n".to_string()
        };
        let mut handler = HandlerBuilder::new()
            .with_line_spec(&spec)
            .with_inner(Cursor::new("1234567890h\n0987654321h\n1234567890".as_bytes()))
            .without_end_of_line_validation()
            .build()
        ;
        let mut buf = String::new();
        handler.read_to_string(&mut buf).unwrap();
        assert_eq!("123456789009876543211234567890".to_string(), buf);

        handler.seek(SeekFrom::Start(9)).unwrap();
        let mut buf = String::new();
        handler.read_to_string(&mut buf).unwrap();
        assert_eq!("009876543211234567890".to_string(), buf);

        handler.seek(SeekFrom::Start(10)).unwrap();
        let mut buf = String::new();
        handler.read_to_string(&mut buf).unwrap();
        assert_eq!("09876543211234567890".to_string(), buf);

        handler.seek(SeekFrom::Start(11)).unwrap();
        let mut buf = String::new();
        handler.read_to_string(&mut buf).unwrap();
        assert_eq!("09876543211234567890".to_string(), buf);

        handler.seek(SeekFrom::Start(12)).unwrap();
        let mut buf = String::new();
        handler.read_to_string(&mut buf).unwrap();
        assert_eq!("09876543211234567890".to_string(), buf);

        handler.seek(SeekFrom::Start(0)).unwrap();
        let buf = &mut [0; 11];
        handler.read(buf).unwrap();
        assert_eq!("12345678900".as_bytes(), buf);

        handler.seek(SeekFrom::Start(0)).unwrap();
        let buf = &mut [0; 10];
        handler.read(buf).unwrap();
        assert_eq!("1234567890".as_bytes(), buf);
        let buf = &mut [0; 10];
        handler.read(buf).unwrap();
        assert_eq!("0987654321".as_bytes(), buf);

        let mut handler = HandlerBuilder::new()
            .with_line_spec(&spec)
            .with_inner(Cursor::new("1234567890h\n0987654321h\n1234567890".as_bytes()))
            .build()
        ;

        handler.seek(SeekFrom::Start(0)).unwrap();
        let buf = &mut [0; 11];
        match handler.read(buf) {
            Err(_)  => (),
            _ => panic!("overflow end of line not returned")
        }

        let mut handler = HandlerBuilder::new()
            .with_line_spec(&spec)
            .with_inner(Cursor::new("1234567890h20987654321h\n1234567890".as_bytes()))
            .without_end_of_line_validation()
            .build()
        ;

        handler.seek(SeekFrom::Start(0)).unwrap();
        let mut buf = String::new();
        match handler.read_to_string(&mut buf) {
            Err(_)  => (),
            _ => panic!("bad line ending not returned")
        }
    }

    #[test]
    pub fn write() {
        let spec = LineSpec {
            length: 10,
            separator: "h\n".to_string()
        };
        let mut handler = HandlerBuilder::new()
            .with_line_spec(&spec)
            .with_inner(Cursor::new(Vec::new()))
            .without_end_of_line_validation()
            .build()
        ;
        handler.write_all("123456789009876543211234567890".as_bytes()).unwrap();
        assert_eq!("1234567890h\n0987654321h\n1234567890h\n".to_string(), String::from_utf8(handler.get_ref().get_ref().clone()).unwrap());

        handler.seek(SeekFrom::Start(9)).unwrap();
        handler.write_all("009876543211234567890".as_bytes()).unwrap();
        assert_eq!("1234567890h\n0987654321h\n1234567890h\n".to_string(), String::from_utf8(handler.get_ref().get_ref().clone()).unwrap());

        handler.seek(SeekFrom::Start(10)).unwrap();
        handler.write_all("09876543211234567890".as_bytes()).unwrap();
        assert_eq!("1234567890h\n0987654321h\n1234567890h\n".to_string(), String::from_utf8(handler.get_ref().get_ref().clone()).unwrap());

        handler.seek(SeekFrom::Start(11)).unwrap();
        handler.write_all("09876543211234567890".as_bytes()).unwrap();
        assert_eq!("1234567890h\n0987654321h\n1234567890h\n".to_string(), String::from_utf8(handler.get_ref().get_ref().clone()).unwrap());

        handler.seek(SeekFrom::Start(12)).unwrap();
        handler.write_all("09876543211234567890".as_bytes()).unwrap();
        assert_eq!("1234567890h\n0987654321h\n1234567890h\n".to_string(), String::from_utf8(handler.get_ref().get_ref().clone()).unwrap());

        handler.seek(SeekFrom::Start(0)).unwrap();
        handler.write_all("1234567890".as_bytes()).unwrap();
        assert_eq!("1234567890h\n0987654321h\n1234567890h\n".to_string(), String::from_utf8(handler.get_ref().get_ref().clone()).unwrap());
        handler.write_all("0987654321".as_bytes()).unwrap();
        assert_eq!("1234567890h\n0987654321h\n1234567890h\n".to_string(), String::from_utf8(handler.get_ref().get_ref().clone()).unwrap());
        handler.write_all("0987654321".as_bytes()).unwrap();
        assert_eq!("1234567890h\n0987654321h\n0987654321h\n".to_string(), String::from_utf8(handler.get_ref().get_ref().clone()).unwrap());

        let mut handler = HandlerBuilder::new()
            .with_line_spec(&spec)
            .with_inner(Cursor::new(Vec::new()))
            .build()
        ;

        handler.seek(SeekFrom::Start(0)).unwrap();
        match handler.write_all("12345678900".as_bytes()) {
            Err(_)  => (),
            _ => panic!("overflow end of line not returned")
        }
    }
}