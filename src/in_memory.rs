use std::string::ToString;
use std::iter::repeat;
use common::{File, Line, Range};

pub struct InMemoryFile {
    name: String,
    width: usize,
    lines: Vec<InMemoryLine>,
    line_seperator: String
}

impl InMemoryFile {
    pub fn new(name: String, width: usize) -> Self {
        Self::new_with_lines(name, width, Vec::new())
    }

    pub fn new_with_lines(name: String, width: usize, lines: Vec<InMemoryLine>) -> Self {
        Self::new_with_lines_and_line_seperator(name, width, lines, "\r\n".to_string())
    }

    pub fn new_with_lines_and_line_seperator(name: String, width: usize, lines: Vec<InMemoryLine>, line_seperator: String) -> Self {
        InMemoryFile {
            name: name,
            width: width,
            lines: lines,
            line_seperator: line_seperator
        }
    }

    pub fn new_with_line_seperator(name: String, width: usize, line_seperator: String) -> Self {
        Self::new_with_lines_and_line_seperator(name, width, Vec::new(), line_seperator)
    }
}

impl File for InMemoryFile {
    type Line = InMemoryLine;
    type Error = String;
    fn name(&self) -> &str {
        &self.name[..]
    }

    fn width(&self) -> usize {
        self.width
    }

    fn line_seperator(&self) -> &str {
        &self.line_seperator[..]
    }

    fn lines(&self) -> &Vec<Self::Line> {
        &self.lines
    }

    fn line(&self, index: usize) -> Result<&Self::Line, Self::Error> {
        self.lines.get(index as usize).ok_or(format!("index {} is out of bounds", index))
    }

    fn add_line<T: Line>(&mut self, line: T) -> Result<(), Self::Error> {
        if line.length() != self.width() {
            return Err("the length of the line does not match the width of the file".to_string());
        }

        self.lines.push(InMemoryLine::new(line.get(..).unwrap()));
        Ok(())
    }

    fn set_line<T: Line>(&mut self, index: usize, line: T) -> Result<(), Self::Error> {
        if line.length() != self.width() {
            return Err("the length of the line does not match the width of the file".to_string());
        }

        let length = self.length();

        if index > length {
            self.lines.extend(repeat(InMemoryLine::new_from_length(self.width)).take(index - length))
        }
        self.lines.insert(index, InMemoryLine::new(line.get(..).unwrap_or(String::new())));
        Ok(())
    }

    fn remove_line(&mut self, index: usize) -> Result<(), Self::Error> {
        self.lines.remove(index);
        Ok(())
    }

    fn length(&self) -> usize {
        self.lines.len() as usize
    }
}

impl ToString for InMemoryFile {
    fn to_string(&self) -> String {
        let mut string = String::new();
        for line in self.lines.iter() {
            if string.len() != 0 {
                string.push_str(&self.line_seperator[..]);
            }

            string.push_str(&line.get(..).unwrap_or(String::new())[..])
        }

        string
    }
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct InMemoryLine {
    data: String
}

impl InMemoryLine {
    pub fn new(data: String) -> Self {
        InMemoryLine { data: data }
    }

    pub fn new_from_length(length: usize) -> Self {
        InMemoryLine { data: repeat(" ").take(length).collect::<String>() }
    }
}

impl Line for InMemoryLine {
    type Error = ();
    fn length(&self) -> usize {
        self.data.len()
    }

    fn get<T: Range>(&self, range: T) -> Result<String, Self::Error> {
        let start = range.start().unwrap_or(0);
        let end = range.end().unwrap_or(self.data.len());
        if start >= self.length() || end > self.length() {
            Err(())
        } else {
            Ok(self.data[start..end].to_string())
        }
    }

    fn set<T: Range>(&mut self, range: T, string: &String) -> Result<(), Self::Error> {
        let start = range.start().unwrap_or(0);
        let end = range.end().unwrap_or(self.data.len());
        if start >= self.length() || end > self.length() || string.len() != end - start{
            Err(())
        } else {
            let mut data = String::new();
            {
                data.push_str(&self.data[..start]);
                data.push_str(&string[..]);
                data.push_str(&self.data[end..]);
            }
            self.data = data;
            Ok(())
        }

    }

    fn remove<T: Range>(&mut self, range: T) -> Result<(), Self::Error> {
        let start = range.start().unwrap_or(0);
        let end = range.end().unwrap_or(self.length());
        if start >= self.length() {
            Err(())
        } else {
            self.set(range, &repeat(" ").take(end - start).collect())
        }
    }
}

impl ToString for InMemoryLine {
    fn to_string(&self) -> String {
        self.data.clone()
    }
}

#[cfg(test)]
mod test {

    use super::{InMemoryLine, InMemoryFile};
    use super::super::common::{Line, File};
    use std::iter::repeat;

    #[test]
    fn in_memory_file() {
        let mut file = InMemoryFile::new("bla".to_string(), 10);
        assert_eq!("bla", file.name());
        let line1 = InMemoryLine::new(repeat("a").take(10).collect::<String>());
        let line2 = InMemoryLine::new(repeat("b").take(10).collect::<String>());
        let line3 = InMemoryLine::new(repeat("c").take(10).collect::<String>());
        let _ = file.add_line(line1.clone());
        let _ = file.add_line(line2.clone());
        assert_eq!(Ok(&line1), file.line(0));
        assert_eq!(Ok(&line2), file.line(1));
        assert_eq!(Err("index 2 is out of bounds".to_string()), file.line(2));
        assert_eq!(&vec![line1.clone(), line2.clone()], file.lines());
        assert_eq!(2, file.length());
        let _ = file.set_line(4, line3.clone());
        assert_eq!(5, file.length());
        assert_eq!(Ok(&line1), file.line(0));
        assert_eq!(Ok(&line2), file.line(1));
        assert_eq!(Ok(&InMemoryLine::new_from_length(10)), file.line(2));
        assert_eq!(Ok(&InMemoryLine::new_from_length(10)), file.line(3));
        assert_eq!(Ok(&line3), file.line(4));
        assert_eq!(&vec![line1.clone(), line2.clone(), InMemoryLine::new_from_length(10), InMemoryLine::new_from_length(10), line3.clone()], file.lines());
        let _ = file.remove_line(2);
        assert_eq!(Ok(&line1), file.line(0));
        assert_eq!(Ok(&line2), file.line(1));
        assert_eq!(Ok(&InMemoryLine::new_from_length(10)), file.line(2));
        assert_eq!(Ok(&line3), file.line(3));
        assert_eq!(&vec![line1, line2, InMemoryLine::new_from_length(10), line3], file.lines());
        assert_eq!(4, file.length());
        assert_eq!("aaaaaaaaaa\r\nbbbbbbbbbb\r\n          \r\ncccccccccc".to_string(), file.to_string());
    }

    #[test]
    fn in_memory_line() {
        let mut line1 = InMemoryLine::new(repeat("a").take(10).collect());
        let mut line2 = InMemoryLine::new_from_length(10);
        assert_eq!(10, line1.length());
        assert_eq!(Ok("aaaaaaaaaa".to_string()), line1.get(..));
        assert_eq!(Ok("aaaa".to_string()), line1.get(1..5));
        assert_eq!(Ok("          ".to_string()), line2.get(..));
        let _ = line1.set(1..5, &"bbbb".to_string()).unwrap();
        assert_eq!(Ok("abbbbaaaaa".to_string()), line1.get(..));
        let _ = line1.remove(6..8).unwrap();
        assert_eq!(Ok("abbbba  aa".to_string()), line1.get(..));
        let _ = line2.set(3, &"a".to_string());
        assert_eq!(Ok("   a      ".to_string()), line2.get(..));
    }


}
