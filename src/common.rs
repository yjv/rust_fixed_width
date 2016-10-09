use std::ops::{Range as RangeStruct, RangeFull, RangeFrom, RangeTo};
use std::fmt::Debug;

pub trait File {
    type Error: Debug;
    fn width(&self) -> usize;
    fn get<T: Range>(&self, line_index: usize, range: T) -> Result<String, Self::Error>;
    fn set<T: Range>(&mut self, line_index: usize, range: T, string: &String) -> Result<&mut Self, Self::Error>;
    fn clear<T: Range>(&mut self, line_index: usize, range: T) -> Result<&mut Self, Self::Error>;
    fn add_line(&mut self) -> Result<usize, Self::Error>;
    fn remove_line(&mut self) -> Result<usize, Self::Error>;
    fn len(&self) -> usize;
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

#[derive(Debug, Eq, PartialEq)]
pub enum InvalidRangeError {
    StartOffEndOfLine,
    EndOffEndOfLine
}

pub fn normalize_range<T: Range, U: Line>(range: T, line_length: usize) -> Result<(usize, usize), InvalidRangeError> {
    let start = range.start().unwrap_or(0);
    let end = range.end().unwrap_or(line_length);
    if start >= line_length {
        Err(InvalidRangeError::StartOffEndOfLine)
    } else if end > line_length {
        Err(InvalidRangeError::EndOffEndOfLine)
    } else {
        Ok((start, end))
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

#[cfg(test)]
mod test {
    use std::string::ToString;
    use super::{Range, InvalidRangeError, normalize_range, FileIterator};
    use super::super::test::*;
    use std::ops::{Range as RangeStruct, RangeFull, RangeFrom, RangeTo};

    #[test]
    fn ranges_work() {
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

    #[test]
    fn normalize_range_works() {
        let line = Line {length: 5, data: "".to_string()};
        assert_eq!(Err(InvalidRangeError::StartOffEndOfLine), normalize_range(7..79, &line));
        assert_eq!(Err(InvalidRangeError::EndOffEndOfLine), normalize_range(..6, &line));
        assert_eq!(Ok((0, 5)), normalize_range(.., &line));
        assert_eq!(Ok((2, 5)), normalize_range(2.., &line));
        assert_eq!(Ok((0, 3)), normalize_range(..3, &line));
    }

    #[test]
    fn iterator_works() {
        let line1 = Line {data: "".to_string(), length: 0};
        let line2 = Line {data: "".to_string(), length: 0};
        let line3 = Line {data: "".to_string(), length: 0};
        let file = File {line_seperator: "\r\n".to_string(), width: 10, lines: vec![
            Ok(&line1),
            Ok(&line2),
            Err(()),
            Ok(&line3)
        ]};
        let mut iterator = FileIterator::new(&file);
        assert_eq!(Some(Ok(&line1)), iterator.next());
        assert_eq!(Some(Ok(&line2)), iterator.next());
        assert_eq!(Some(Err(())), iterator.next());
        assert_eq!(Some(Ok(&line3)), iterator.next());
        assert_eq!(None, iterator.next());
    }
}