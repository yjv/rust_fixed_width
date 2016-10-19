use super::common::{Range, File as FileTrait};

#[derive(Debug)]
pub struct File {
    pub width: usize,
    pub line_seperator: String,
    pub lines: Vec<Result<Option<String>, ()>>
}

impl FileTrait for File {
    type Error = ();

    fn width(&self) -> usize {
        self.width
    }

    fn get<T: Range>(&self, _: usize, _: T) -> Result<Option<String>, Self::Error> {
        unimplemented!()
    }

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

    fn len(&self) -> usize {
        unimplemented!()
    }
}
