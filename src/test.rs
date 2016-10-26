use super::common::{Range, File as FileTrait, MutableFile, FileError};

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

    fn get<T: Range>(&self, index: usize, _: T) -> Result<String, Self::Error> {
        self.lines.get(index).unwrap().clone()
    }

    fn len(&self) -> usize {
        self.lines.len()
    }
}

impl MutableFile for File {
    fn set<T: Range>(&mut self, _: usize, _: T, _: &String) -> Result<&mut Self, Self::Error> {
        unimplemented!()
    }

    fn clear<T: Range>(&mut self, _: usize, _: T) -> Result<&mut Self, Self::Error> {
        unimplemented!()
    }

    fn add_line(&mut self) -> Result<usize, Self::Error> {
        unimplemented!()
    }

    fn remove_line(&mut self) -> Result<usize, Self::Error> {
        unimplemented!()
    }
}
