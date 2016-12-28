use std::ops::Range;
use std::fmt::Debug;
use spec::FileSpec;
use std::io::{Read, Seek, SeekFrom, Error as IoError, Write, ErrorKind};
use std::cmp::min;

pub trait File {
    type Error: FileError;
    fn width(&self) -> usize;
    fn get(&self, line_index: usize, range: Range<usize>) -> Result<String, Self::Error>;
    fn len(&self) -> usize;
}

pub trait MutableFile {
    type Error: FileError;
    fn set(&mut self, line_index: usize, column_index: usize, string: &String) -> Result<&mut Self, Self::Error>;
    fn clear(&mut self, line_index: usize, range: Range<usize>) -> Result<&mut Self, Self::Error>;
    fn add_line(&mut self) -> Result<usize, Self::Error>;
    fn remove_line(&mut self) -> Result<usize, Self::Error>;
}

pub trait FileError: Debug {
    fn is_invalid_index(&self) -> bool;
    fn is_invalid_range(&self) -> bool;
}

#[derive(Debug, Eq, PartialEq)]
pub enum InvalidRangeError {
    StartOffEndOfLine,
    EndOffEndOfLine
}

pub fn validate_range(range: Range<usize>, line_length: usize) -> Result<Range<usize>, InvalidRangeError> {
    if range.start >= line_length {
        Err(InvalidRangeError::StartOffEndOfLine)
    } else if range.end > line_length {
        Err(InvalidRangeError::EndOffEndOfLine)
    } else {
        Ok(range)
    }
}

pub struct FileIterator<'a, T: 'a + File> {
    position: usize,
    file: &'a T
}

impl<'a, T: 'a + File> FileIterator<'a, T> {
    pub fn new(file: &'a T) -> Self {
        FileIterator {
            position: 0,
            file: file
        }
    }
}

impl<'a, T: 'a + File> Iterator for FileIterator<'a, T> {
    type Item = Result<String, T::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.position = self.position + 1;
        if self.position > self.file.len() {
            None
        } else {
            Some(self.file.get(self.position - 1, 0..self.file.width()))
        }
    }
}

pub struct Reader<T: Read> {
    reader: T,
    line_separator: String,
    line_length: usize,
    position: Position
}

pub enum Error {
    Io(IoError),
    NotEnoughRead(usize),
    NotEnoughWritten(usize)
}

impl From<IoError> for Error {
    fn from(e: IoError) -> Self {
        Error::Io(e)
    }
}

//type Result<T> = ::std::result::Result<T, Error>;

pub struct Position {
    pub position: usize,
    pub line: usize,
    pub column: usize
}

impl Position {
    pub fn recalculate(&mut self, line_length: usize) {
        if self.position == 0 {
            self.line = 0;
            self.column = 0;
            return;
        }

        self.line = self.position / line_length;
        self.line = self.position % line_length;
    }
}

impl<T: Read> Read for Reader<T> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, IoError> {
        let mut total_amount = 0;
        let length = buf.len();

        while total_amount < length {
            let remaining_amount = min(self.line_length - self.position.column, buf.len() - total_amount);
            let amount = match self.reader.read(&mut buf[total_amount..total_amount + remaining_amount]) {
                Ok(0) => return Ok(total_amount),
                Ok(len) => len,
                Err(e) => return Err(e),
            };

            total_amount += amount;
            self.position.position += amount;
            self.position.recalculate(self.line_length + self.line_separator.len());
            if self.position.column == self.line_length {
                let mut line_separator = String::new();
                self.position.position += self.reader.by_ref().take(self.line_separator.len() as u64).read_to_string(&mut line_separator)?;
                if line_separator.len() != 0 && line_separator != self.line_separator {
                    return Err(IoError::new(ErrorKind::Other, "dsaadsasd"));
                }
                self.position.recalculate(self.line_length + self.line_separator.len());
            }
        }

        Ok(total_amount)
    }
}

struct Writer<T: Write> {
    writer: T,
    line_separator: String,
    line_length: usize,
    position: Position
}

#[cfg(test)]
mod test {
    use std::string::ToString;
    use super::{InvalidRangeError, validate_range, FileIterator};
    use super::super::test::*;

    #[test]
    fn validate_range_works() {
        assert_eq!(Err(InvalidRangeError::StartOffEndOfLine), validate_range(7..79, 5));
        assert_eq!(Err(InvalidRangeError::EndOffEndOfLine), validate_range(0..6, 5));
        assert_eq!(Ok(4..7), validate_range(4..7, 10));
    }

    #[test]
    fn iterator_works() {
        let line1 = "   ".to_string();
        let line2 = "123".to_string();
        let line3 = "fsd".to_string();
        let mut file = MockFile::new(3, Some(vec![
            &line1,
            &line2,
            &line3
        ]));
        file.add_read_error(2);
        let mut iterator = FileIterator::new(&file);
        assert_eq!(Some(Ok(line1)), iterator.next());
        assert_eq!(Some(Ok(line2)), iterator.next());
        assert_eq!(Some(Err(())), iterator.next());
        assert_eq!(Some(Ok(line3)), iterator.next());
        assert_eq!(None, iterator.next());
        assert_eq!(None, iterator.next());
        assert_eq!(None, iterator.next());
    }
}