use std::string::ToString;
use std::iter::repeat;
use common::{File, Line, Range, normalize_range, validate_line, LineGenerator};

pub struct InMemoryFile {
    name: String,
    width: usize,
    lines: Vec<InMemoryLine>,
    line_separator: String
}

impl InMemoryFile {
    pub fn new(name: String, width: usize) -> Self {
        Self::new_with_lines(name, width, Vec::new())
    }

    pub fn new_with_lines(name: String, width: usize, lines: Vec<InMemoryLine>) -> Self {
        Self::new_with_lines_and_line_separator(name, width, lines, "\r\n".to_string())
    }

    pub fn new_with_lines_and_line_separator(name: String, width: usize, lines: Vec<InMemoryLine>, line_separator: String) -> Self {
        InMemoryFile {
            name: name,
            width: width,
            lines: lines,
            line_separator: line_separator
        }
    }

    pub fn new_with_line_separator(name: String, width: usize, line_separator: String) -> Self {
        Self::new_with_lines_and_line_separator(name, width, Vec::new(), line_separator)
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

    fn line_separator(&self) -> &str {
        &self.line_separator[..]
    }

    fn line(&self, index: usize) -> Result<&Self::Line, Self::Error> {
        self.lines.get(index).ok_or(format!("index {} is out of bounds", index))
    }

    fn line_mut(&mut self, index: usize) -> Result<&mut Self::Line, Self::Error> {
        self.lines.get_mut(index).ok_or(format!("index {} is out of bounds", index))
    }

    fn add_line<T: Line>(&mut self, line: T) -> Result<&mut Self, Self::Error> {
        let line = try!(validate_line(line, self));
        self.lines.push(InMemoryLine::new(line.get(..).unwrap()));
        Ok(self)
    }

    fn set_line<T: Line>(&mut self, index: usize, line: T) -> Result<&mut Self, Self::Error> {
        let line = try!(validate_line(line, self));

        let length = self.len();

        if index > length {
            self.lines.extend(repeat(InMemoryLine::new_from_length(self.width)).take(index - length))
        }

        self.lines.insert(index, InMemoryLine::new(line.get(..).unwrap_or(String::new())));
        Ok(self)
    }

    fn remove_line(&mut self, index: usize) -> Result<&mut Self, Self::Error> {
        self.lines.remove(index);
        Ok(self)
    }

    fn len(&self) -> usize {
        self.lines.len()
    }
}

impl ToString for InMemoryFile {
    fn to_string(&self) -> String {
        let mut string = String::new();
        for line in self.lines.iter() {
            if string.len() != 0 {
                string.push_str(&self.line_separator[..]);
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
    type Error = String;
    fn len(&self) -> usize {
        self.data.len()
    }

    fn get<T: Range>(&self, range: T) -> Result<String, Self::Error> {
        let (start, end) = try!(normalize_range(range, self));
        Ok(self.data[start..end].to_string())
    }

    fn set<T: Range>(&mut self, range: T, string: &String) -> Result<&mut Self, Self::Error> {
        let (start, end) = try!(normalize_range(range, self));
        if string.len() > end - start {
            Err("string longer than the range being set".to_string())
        } else {
            let mut data = String::new();
            data.push_str(&self.data[..start]);
            data.push_str(&string[..]);
            data.push_str(&repeat(" ").take(end - start - string.len()).collect::<String>()[..]);
            data.push_str(&self.data[end..]);
            self.data = data;
            Ok(self)
        }
    }

    fn remove<T: Range>(&mut self, range: T) -> Result<&mut Self, Self::Error> {
        let (start, end) = try!(normalize_range(range.clone(), self));
        self.set(range, &repeat(" ").take(end - start).collect())
    }
}

impl ToString for InMemoryLine {
    fn to_string(&self) -> String {
        self.data.clone()
    }
}

pub struct InMemoryLineGenerator;

impl LineGenerator for InMemoryLineGenerator {
    type Error = ();
    type Line = InMemoryLine;

    fn generate_line(&self, length: usize) -> Result<Self::Line, Self::Error> {
        Ok(InMemoryLine::new_from_length(length))
    }
}

#[cfg(test)]
mod test {

    use super::{InMemoryLine, InMemoryFile, InMemoryLineGenerator};
    use super::super::common::{Line, File, FileIterator, LineGenerator};
    use std::iter::repeat;
    use std::iter::Iterator;

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
        assert_eq!(vec![&line1, &line2], FileIterator::new(&file).map(|r| r.unwrap()).collect::<Vec<&InMemoryLine>>());
        assert_eq!(2, file.len());
        let _ = file.set_line(4, line3.clone());
        assert_eq!(5, file.len());
        assert_eq!(Ok(&line1), file.line(0));
        assert_eq!(Ok(&line2), file.line(1));
        assert_eq!(Ok(&InMemoryLine::new_from_length(10)), file.line(2));
        assert_eq!(Ok(&InMemoryLine::new_from_length(10)), file.line(3));
        assert_eq!(Ok(&line3), file.line(4));
        assert_eq!(vec![&line1, &line2, &InMemoryLine::new_from_length(10), &InMemoryLine::new_from_length(10), &line3], FileIterator::new(&file).map(|r| r.unwrap()).collect::<Vec<&InMemoryLine>>());
        let _ = file.remove_line(2);
        assert_eq!(Ok(&line1), file.line(0));
        assert_eq!(Ok(&line2), file.line(1));
        assert_eq!(Ok(&InMemoryLine::new_from_length(10)), file.line(2));
        assert_eq!(Ok(&line3), file.line(3));
        assert_eq!(vec![&line1, &line2, &InMemoryLine::new_from_length(10), &line3], FileIterator::new(&file).map(|r| r.unwrap()).collect::<Vec<&InMemoryLine>>());
        assert_eq!(4, file.len());
        assert_eq!("aaaaaaaaaa\r\nbbbbbbbbbb\r\n          \r\ncccccccccc".to_string(), file.to_string());
    }

    #[test]
    fn in_memory_line() {
        let mut line1 = InMemoryLine::new(repeat("a").take(10).collect());
        let mut line2 = InMemoryLine::new_from_length(10);
        assert_eq!(10, line1.len());
        assert_eq!(Ok("aaaaaaaaaa".to_string()), line1.get(..));
        assert_eq!(Ok("aaaa".to_string()), line1.get(1..5));
        assert_eq!(Ok("          ".to_string()), line2.get(..));
        assert_eq!(Ok("abbbbaaaaa".to_string()), line1.set(1..5, &"bbbb".to_string()).unwrap().get(..));
        assert_eq!(Ok("abbbba  aa".to_string()), line1.remove(6..8).unwrap().get(..));
        assert_eq!(Ok("   a      ".to_string()), line2.set(3, &"a".to_string()).unwrap().get(..));
        assert_eq!(Ok("abbbba b a".to_string()), line1.set(7..9, &"b".to_string()).unwrap().get(..));
        assert_eq!(Ok("b  a      ".to_string()), line2.set(0, &"b".to_string()).unwrap().get(..));
        assert_eq!(Ok("b  a     b".to_string()), line2.set(9, &"b".to_string()).unwrap().get(..));
    }

    #[test]
    fn in_memory_line_generator() {
        let generator = InMemoryLineGenerator;
        assert_eq!(Ok(InMemoryLine::new_from_length(12)), generator.generate_line(12));
    }
}
