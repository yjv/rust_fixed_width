use super::common::{File as FileTrait, MutableFile, FileError};
use std::ops::Range;

#[derive(Debug)]
pub struct File {
    pub width: usize,
    pub line_seperator: String,
    pub lines: Vec<Result<String, ()>>
}

impl FileError for () {
    fn is_invalid_index(&self) -> bool {
        unimplemented!()
    }

    fn is_invalid_range(&self) -> bool {
        unimplemented!()
    }
}

impl FileTrait for File {
    type Error = ();

    fn width(&self) -> usize {
        self.width
    }

    fn get(&self, index: usize, range: Range<usize>) -> Result<String, Self::Error> {
        Ok(try!(self.lines.get(index).unwrap().clone().map(|s| s[range].to_string())))
    }

    fn len(&self) -> usize {
        self.lines.len()
    }
}

impl MutableFile for File {
    fn set(&mut self, _: usize, _: usize, _: &String) -> Result<&mut Self, Self::Error> {
        unimplemented!()
    }

    fn clear(&mut self, _: usize, _: Range<usize>) -> Result<&mut Self, Self::Error> {
        unimplemented!()
    }

    fn add_line(&mut self) -> Result<usize, Self::Error> {
        unimplemented!()
    }

    fn remove_line(&mut self) -> Result<usize, Self::Error> {
        unimplemented!()
    }
}
