use std::ops::Range;
use std::string::ToString;

#[test]
fn it_works() {
}
pub trait File: ToString {
    type Line: Line;
    fn name(&self) -> &str;
    fn width(&self) -> i64;
    fn line_seperator(&self) -> &str;
    fn lines(&self) -> &Vec<Self::Line>;
    fn line(&self, index: i64) -> Option<&Self::Line>;
    fn add_line<T: Line>(&mut self, line: T) -> Self;
    fn set_line<T: Line>(&mut self, index: i64, line: T) -> Self;
    fn remove_line(&mut self, index: usize) -> Self;
    fn length(&self) -> i64;
}

pub trait Line: ToString {
    fn length(&self) -> i64;
    fn get(range: Range<usize>) -> Option<String>;
    fn set(range: Range<usize>, string: String) -> Self;
    fn remove(range: Range<usize>) -> Self;
}

pub struct InMemoryFile {
    name: String,
    width: i64,
    lines: Vec<InMemoryLine>,
    line_seperator: String
}

pub struct InMemoryLine {
    data: String
}

impl InMemoryFile {
    pub fn new(name: String, width: i64) -> Self {
        Self::new_with_lines(name, width, Vec::new())
    }

    pub fn new_with_lines(name: String, width: i64, lines: Vec<InMemoryLine>) -> Self {
        Self::new_with_lines_and_line_seperator(name, width, lines, "\r\n".to_string())
    }

    pub fn new_with_lines_and_line_seperator(name: String, width: i64, lines: Vec<InMemoryLine>, line_seperator: String) -> Self {
        InMemoryFile {
            name: name,
            width: width,
            lines: lines,
            line_seperator: line_seperator
        }
    }

    pub fn new_with_line_seperator(name: String, width: i64, line_seperator: String) -> Self {
        Self::new_with_lines_and_line_seperator(name, width, Vec::new(), line_seperator)
    }
}

impl File for InMemoryFile {
    type Line = InMemoryLine;
    fn name(&self) -> &str {
        &self.name[..]
    }
    fn width(&self) -> i64 {
        self.width
    }
    fn line_seperator(&self) -> &str {
        &self.line_seperator[..]
    }
    fn lines(&self) -> &Vec<Self::Line> {
        &self.lines
    }
    fn line(&self, index: i64) -> Option<&Self::Line> {
        self.lines.get(index as usize)
    }

    fn add_line<T: Line>(&mut self, line: T) -> Self {
        self.lines.push(From::from(line))
    }

    fn set_line<T: Line>(&mut self, index: usize, line: T) -> Self {
        self.lines[index] = From::from(line);
    }

    fn remove_line(&mut self, index: usize) -> Self {
        self.lines.removew(index)
    }

    fn length(&self) -> i64 {
        self.lines.len() as i64
    }
}

impl ToString for InMemoryFile {
    fn to_string(&self) -> String {
        let mut string = String::new();
        for line in self.lines.iter() {
            if string.len() != 0 {
                string.push_str(&self.line_seperator[..]);
            }

            string.push_str(&line[..])
        }

        string
    }
}

impl Line for InMemoryLine {
    fn length(&self) -> i64 {
        unimplemented!()
    }
    fn get(range: Range<usize>) -> Option<String> {
        unimplemented!()
    }
    fn set(range: Range<usize>, string: String) -> Self {
        unimplemented!()
    }
    fn remove(range: Range<usize>) -> Self {
        unimplemented!()
    }
}

impl ToString for InMemoryLine {
    fn to_string(&self) -> String {
        self.data.clone()
    }
}