use std::string::ToString;
use std::ops::{Range as RangeStruct, RangeFull, RangeFrom, RangeTo};
use std::fmt::Debug;

pub trait File: ToString {
    type Line: Line;
    type Error: Debug;
    fn name(&self) -> &str;
    fn width(&self) -> usize;
    fn line_separator(&self) -> &str;
    fn line(&self, index: usize) -> Result<Option<&Self::Line>, Self::Error>;
    fn line_mut(&mut self, index: usize) -> Result<Option<&mut Self::Line>, Self::Error>;
    fn add_line<T: Line>(&mut self, line: T) -> Result<&mut Self, Self::Error>;
    fn set_line<T: Line>(&mut self, index: usize, line: T) -> Result<&mut Self, Self::Error>;
    fn remove_line(&mut self, index: usize) -> Result<&mut Self, Self::Error>;
    fn len(&self) -> usize;
}

pub trait Line: ToString {
    type Error: Debug;
    fn len(&self) -> usize;
    fn get<T: Range>(&self, range: T) -> Result<String, Self::Error>;
    fn set<T: Range>(&mut self, range: T, string: &String) -> Result<&mut Self, Self::Error>;
    fn remove<T: Range>(&mut self, range: T) -> Result<&mut Self, Self::Error>;
}

pub trait Range: Clone {
    fn start(&self) -> Option<usize>;
    fn end(&self) -> Option<usize>;
}

impl Range for RangeStruct<usize> {
    fn start(&self) -> Option<usize> {
        Some(self.start)
    }

    fn end(&self) -> Option<usize> {
        Some(self.end)
    }
}

impl Range for RangeFull {
    fn start(&self) -> Option<usize> {
        None
    }

    fn end (&self) -> Option<usize> {
        None
    }
}

impl Range for RangeFrom<usize> {
    fn start(&self) -> Option<usize> {
        Some(self.start)
    }

    fn end(&self) -> Option<usize> {
        None
    }
}

impl Range for RangeTo<usize> {
    fn start(&self) -> Option<usize> {
        None
    }

    fn end(&self) -> Option<usize> {
        Some(self.end)
    }
}

impl Range for usize {
    fn start(&self) -> Option<usize> {
        Some(*self)
    }

    fn end(&self) -> Option<usize> {
        Some(*self + 1)
    }
}

#[derive(Debug)]
pub enum InvalidRangeError {
    StartOffEndOfLine,
    EndOffEndOfLine
}

pub fn normalize_range<T: Range, U: Line>(range: T, line: &U) -> Result<(usize, usize), InvalidRangeError> {
    let start = range.start().unwrap_or(0);
    let end = range.end().unwrap_or(line.len());
    if start >= line.len() {
        Err(InvalidRangeError::StartOffEndOfLine)
    } else if end > line.len() {
        Err(InvalidRangeError::EndOffEndOfLine)
    } else {
        Ok((start, end))
    }
}

#[derive(Debug)]
pub enum InvalidLineError {
    LineLengthWrong,
    LineContainsLineSeparator
}

pub fn validate_line<T: Line, U: File>(line: T, file: &U) -> Result<T, InvalidLineError> {
    if line.len() != file.width() {
        Err(InvalidLineError::LineLengthWrong)
    } else if line.to_string().contains(file.line_separator()) {
        Err(InvalidLineError::LineContainsLineSeparator)
    } else {
        Ok(line)
    }
}

pub struct FileIterator<'a, T: File + 'a> {
    position: usize,
    file: &'a T
}

impl<'a, T: File + 'a> FileIterator<'a, T> {
    pub fn new(file: &'a T) -> Self {
        FileIterator {
            position: 0,
            file: file
        }
    }
}

impl<'a, T: File + 'a> Iterator for FileIterator<'a, T> {
    type Item = Result<&'a <T as File>::Line, <T as File>::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.position = self.position + 1;
        match self.file.line(self.position - 1) {
            Ok(Some(line)) => Some(Ok(line)),
            Err(error) => Some(Err(error)),
            Ok(None) => None
        }
    }
}

pub trait FromField: Sized {
    fn from_field(string: String) -> Result<Self, String>;
}

pub trait ToField {
    fn to_field(&self) -> Result<String, String>;
}

impl FromField for String {
    fn from_field(string: String) -> Result<Self, String> {
        Ok(string)
    }
}

pub trait LineGenerator {
    type Error: Debug;
    type Line: Line;
    fn generate_line(&self, length: usize) -> Result<Self::Line, Self::Error>;
}

#[cfg(test)]
mod test {

    use super::Range;
    use std::ops::{Range as RangeStruct, RangeFull, RangeFrom, RangeTo};

    #[test]
    fn ranges() {
        let range1 = RangeStruct { start: 2, end: 5 };
        let range2 = RangeFull;
        let range3 = RangeFrom { start: 4 };
        let range4 = RangeTo { end: 8 };
        let range5: usize = 4;
        assert_eq!(Some(2), range1.start());
        assert_eq!(Some(5), range1.end());
        assert_eq!(None, range2.start());
        assert_eq!(None, range2.end());
        assert_eq!(Some(4), range3.start());
        assert_eq!(None, range3.end());
        assert_eq!(None, range4.start());
        assert_eq!(Some(8), range4.end());
        assert_eq!(Some(4), range5.start());
        assert_eq!(Some(5), range5.end());
    }
}