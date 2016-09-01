use std::string::ToString;
use super::common::{Range, Line as LineTrait, File as FileTrait};

#[derive(Debug)]
pub struct File<'a> {
    pub width: usize,
    pub line_seperator: String,
    pub lines: Vec<Result<&'a Line, ()>>
}

impl<'a> FileTrait for File<'a> {
    type Line = Line;
    type Error = ();
    fn name(&self) -> &str {
        unimplemented!()
    }

    fn width(&self) -> usize {
        self.width
    }

    fn line_separator(&self) -> &str {
        &self.line_seperator[..]
    }

    fn line(&self, index: usize) -> Result<Option<&Self::Line>, Self::Error> {
        match self.lines.get(index) {
            Some(&Ok(line)) => Ok(Some(line)),
            Some(&Err(error)) => Err(error),
            None => Ok(None)
        }
    }

    fn line_mut(&mut self, _: usize) -> Result<Option<&mut Self::Line>, Self::Error> {
        unimplemented!()
    }

    fn add_line(&mut self) -> Result<usize, Self::Error> {
        unimplemented!()
    }

    fn remove_line(&mut self) -> Result<usize, Self::Error> {
        unimplemented!()
    }

    fn len(&self) -> usize {
        unimplemented!()
    }
}

impl<'a> ToString for File<'a> {
    fn to_string(&self) -> String {
        unimplemented!()
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Line {
    pub length: usize,
    pub data: String
}

impl LineTrait for Line {
    type Error = ();
    fn len(&self) -> usize {
        self.length
    }

    fn get<T: Range>(&self, _: T) -> Result<String, Self::Error> {
        unimplemented!()
    }

    fn set<T: Range>(&mut self, _: T, _: &String) -> Result<&mut Self, Self::Error> {
        unimplemented!()
    }

    fn clear<T: Range>(&mut self, _: T) -> Result<&mut Self, Self::Error> {
        unimplemented!()
    }
}

impl ToString for Line {
    fn to_string(&self) -> String {
        self.data.clone()
    }
}
