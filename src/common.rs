use std::ops::Range;
use std::fmt::Debug;
use std::cmp::min;

pub trait File {
    type Error: FileError;
    fn width(&self) -> usize;
    fn get(&self, line_index: usize, range: Range<usize>) -> Result<String, Self::Error>;
    fn len(&self) -> usize;
}

pub trait MutableFile: File {
    fn set(&mut self, line_index: usize, range: Range<usize>, string: &String) -> Result<&mut Self, Self::Error>;
    fn clear(&mut self, line_index: usize, range: Range<usize>) -> Result<&mut Self, Self::Error>;
    fn add_line(&mut self) -> Result<usize, Self::Error>;
    fn remove_line(&mut self) -> Result<usize, Self::Error>;
}

impl<'a, U> File for &'a U where U: 'a + File {
    type Error = U::Error;
    fn width(&self) -> usize {
        (*self).width()
    }

    fn get(&self, line_index: usize, range: Range<usize>) -> Result<String, Self::Error> {
        (*self).get(line_index, range)
    }

    fn len(&self) -> usize {
        (*self).len()
    }
}

impl<'a, U> File for &'a mut U where U: 'a + File {
    type Error = U::Error;
    fn width(&self) -> usize {
        (**self).width()
    }

    fn get(&self, line_index: usize, range: Range<usize>) -> Result<String, Self::Error> {
        (**self).get(line_index, range)
    }

    fn len(&self) -> usize {
        (**self).len()
    }
}

impl<'a, U> MutableFile for &'a mut U where U: 'a + MutableFile {
    fn set(&mut self, line_index: usize, range: Range<usize>, string: &String) -> Result<&mut Self, <Self as File>::Error> {
        match (**self).set(line_index, range, string) {
            Ok(_) => Ok(self),
            Err(err) => Err(err)
        }

    }

    fn clear(&mut self, line_index: usize, range: Range<usize>) -> Result<&mut Self, <Self as File>::Error> {
        match (**self).clear(line_index, range) {
            Ok(_) => Ok(self),
            Err(err) => Err(err)
        }
    }

    fn add_line(&mut self) -> Result<usize, <Self as File>::Error> {
        (**self).add_line()
    }

    fn remove_line(&mut self) -> Result<usize, <Self as File>::Error> {
        (**self).remove_line()
    }
}
//
//pub trait Range: Clone {
//    fn start(&self) -> Option<usize>;
//    fn end(&self) -> Option<usize>;
//}

pub trait FileError: Debug {
    fn is_invalid_index(&self) -> bool;
    fn is_invalid_range(&self) -> bool;
}

#[derive(Debug, Eq, PartialEq)]
pub enum InvalidRangeError {
    StartOffEndOfLine,
    EndOffEndOfLine,
    LengthShorterThanString
}

pub fn validate_range(range: Range<usize>, line_length: usize, string: Option<&String>) -> Result<(usize, usize), InvalidRangeError> {
    if range.start >= line_length {
        Err(InvalidRangeError::StartOffEndOfLine)
    } else if range.end > line_length {
        Err(InvalidRangeError::EndOffEndOfLine)
    } else if string.is_some() && range.end - range.start < string.unwrap().len() {
        Err(InvalidRangeError::LengthShorterThanString)
    } else {
        Ok((range.start, range.end))
    }
}

pub struct FileIterator<T: File> {
    position: usize,
    file: T
}

impl<T: File> FileIterator<T> {
    pub fn new(file: T) -> Self {
        FileIterator {
            position: 0,
            file: file
        }
    }
}

impl<T: File> Iterator for FileIterator<T> {
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

#[cfg(test)]
mod test {
    use std::string::ToString;
    use super::{InvalidRangeError, validate_range, FileIterator};
    use super::super::test::*;
    use std::ops::{Range as RangeStruct, RangeFull, RangeFrom, RangeTo};

    #[test]
    fn validate_range_works() {
        assert_eq!(Err(InvalidRangeError::StartOffEndOfLine), validate_range(7..79, 5, None));
        assert_eq!(Err(InvalidRangeError::EndOffEndOfLine), validate_range(..6, 5, None));
        assert_eq!(Err(InvalidRangeError::LengthShorterThanString), validate_range(3, 5, Some(&"dasadsads".to_string())));
    }

    #[test]
    fn iterator_works() {
        let line1 = "".to_string();
        let line2 = "".to_string();
        let line3 = "".to_string();
        let file = File {line_seperator: "\r\n".to_string(), width: 10, lines: vec![
            Ok(line1.clone()),
            Ok(line2.clone()),
            Err(()),
            Ok(line3.clone())
        ]};
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