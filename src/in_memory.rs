use std::string::ToString;
use std::iter::repeat;
use common::{File as FileTrait, Line as LineTrait, Range, normalize_range, validate_line, LineGenerator as LineGeneratorTrait, InvalidRangeError, InvalidLineError};

pub struct File {
    name: String,
    width: usize,
    lines: Vec<Line>,
    line_separator: String
}

impl File {
    pub fn new(name: String, width: usize, lines: Vec<Line>, line_separator: String) -> Self {
        File {
            name: name,
            width: width,
            lines: lines,
            line_separator: line_separator
        }
    }

    pub fn new_with_name_and_width(name: String, width: usize) -> Self {
        Self::new(name, width, Vec::new(), "\r\n".to_string())
    }
}

#[derive(Debug)]
pub enum FileError {
    InvalidLine(InvalidLineError)
}

impl FileTrait for File {
    type Line = Line;
    type Error = FileError;
    fn name(&self) -> &str {
        &self.name[..]
    }

    fn width(&self) -> usize {
        self.width
    }

    fn line_separator(&self) -> &str {
        &self.line_separator[..]
    }

    fn line(&self, index: usize) -> Result<Option<&Self::Line>, Self::Error> {
        Ok(self.lines.get(index))
    }

    fn line_mut(&mut self, index: usize) -> Result<Option<&mut Self::Line>, Self::Error> {
        Ok(self.lines.get_mut(index))
    }

    fn add_line<T: LineTrait>(&mut self, line: T) -> Result<&mut Self, Self::Error> {
        let line = try!(validate_line(line, self).map_err(FileError::InvalidLine));
        self.lines.push(Line::new(line.get(..).unwrap()));
        Ok(self)
    }

    fn set_line<T: LineTrait>(&mut self, index: usize, line: T) -> Result<&mut Self, Self::Error> {
        let line = try!(validate_line(line, self).map_err(FileError::InvalidLine));

        let length = self.len();

        if index > length {
            self.lines.extend(repeat(Line::new_from_length(self.width)).take(index - length))
        }

        self.lines.insert(index, Line::new(line.get(..).unwrap_or(String::new())));
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

impl ToString for File {
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
pub struct Line {
    data: String
}

impl Line {
    pub fn new(data: String) -> Self {
        Line { data: data }
    }

    pub fn new_from_length(length: usize) -> Self {
        Line { data: repeat(" ").take(length).collect::<String>() }
    }
}

#[derive(Debug)]
pub enum LineError {
    DataLongerThanRange,
    InvalidRange(InvalidRangeError)
}

impl LineTrait for Line {
    type Error = LineError;
    fn len(&self) -> usize {
        self.data.len()
    }

    fn get<T: Range>(&self, range: T) -> Result<String, Self::Error> {
        let (start, end) = try!(normalize_range(range, self).map_err(LineError::InvalidRange));
        Ok(self.data[start..end].to_string())
    }

    fn set<T: Range>(&mut self, range: T, string: &String) -> Result<&mut Self, Self::Error> {
        let (start, end) = try!(normalize_range(range, self).map_err(LineError::InvalidRange));
        if string.len() > end - start {
            Err(LineError::DataLongerThanRange)
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
        let (start, end) = try!(normalize_range(range.clone(), self).map_err(LineError::InvalidRange));
        self.set(range, &repeat(" ").take(end - start).collect())
    }
}

impl ToString for Line {
    fn to_string(&self) -> String {
        self.data.clone()
    }
}

pub struct LineGenerator;

impl LineGeneratorTrait for LineGenerator {
    type Error = ();
    type Line = Line;

    fn generate_line(&self, length: usize) -> Result<Self::Line, Self::Error> {
        Ok(Line::new_from_length(length))
    }
}

#[cfg(test)]
mod test {

    use super::{Line, File, LineGenerator};
    use super::super::common::{Line as LineTrait, File as FileTrait, FileIterator, LineGenerator as LineGeneratorTrait};
    use std::iter::repeat;
    use std::iter::Iterator;

    #[test]
    fn in_memory_file() {
        let mut file = File::new_with_name_and_width("bla".to_string(), 10);
        assert_eq!("bla", file.name());
        let line1 = Line::new(repeat("a").take(10).collect::<String>());
        let line2 = Line::new(repeat("b").take(10).collect::<String>());
        let line3 = Line::new(repeat("c").take(10).collect::<String>());
        let _ = file.add_line(line1.clone());
        let _ = file.add_line(line2.clone());
        assert_eq!(Some(&line1), file.line(0).unwrap());
        assert_eq!(Some(&line2), file.line(1).unwrap());
        assert_eq!(None, file.line(2).unwrap());
        assert_eq!(vec![&line1, &line2], FileIterator::new(&file).map(|r| r.unwrap()).collect::<Vec<&Line>>());
        assert_eq!(2, file.len());
        let _ = file.set_line(4, line3.clone());
        assert_eq!(5, file.len());
        assert_eq!(Some(&line1), file.line(0).unwrap());
        assert_eq!(Some(&line2), file.line(1).unwrap());
        assert_eq!(Some(&Line::new_from_length(10)), file.line(2).unwrap());
        assert_eq!(Some(&Line::new_from_length(10)), file.line(3).unwrap());
        assert_eq!(Some(&line3), file.line(4).unwrap());
        assert_eq!(vec![&line1, &line2, &Line::new_from_length(10), &Line::new_from_length(10), &line3], FileIterator::new(&file).map(|r| r.unwrap()).collect::<Vec<&Line>>());
        let _ = file.remove_line(2);
        assert_eq!(Some(&line1), file.line(0).unwrap());
        assert_eq!(Some(&line2), file.line(1).unwrap());
        assert_eq!(Some(&Line::new_from_length(10)), file.line(2).unwrap());
        assert_eq!(Some(&line3), file.line(3).unwrap());
        assert_eq!(vec![&line1, &line2, &Line::new_from_length(10), &line3], FileIterator::new(&file).map(|r| r.unwrap()).collect::<Vec<&Line>>());
        assert_eq!(4, file.len());
        assert_eq!("aaaaaaaaaa\r\nbbbbbbbbbb\r\n          \r\ncccccccccc".to_string(), file.to_string());
    }

    #[test]
    fn in_memory_line() {
        let mut line1 = Line::new(repeat("a").take(10).collect());
        let mut line2 = Line::new_from_length(10);
        assert_eq!(10, line1.len());
        assert_eq!("aaaaaaaaaa".to_string(), line1.get(..).unwrap());
        assert_eq!("aaaa".to_string(), line1.get(1..5).unwrap());
        assert_eq!("          ".to_string(), line2.get(..).unwrap());
        assert_eq!("abbbbaaaaa".to_string(), line1.set(1..5, &"bbbb".to_string()).unwrap().get(..).unwrap());
        assert_eq!("abbbba  aa".to_string(), line1.remove(6..8).unwrap().get(..).unwrap());
        assert_eq!("   a      ".to_string(), line2.set(3, &"a".to_string()).unwrap().get(..).unwrap());
        assert_eq!("abbbba b a".to_string(), line1.set(7..9, &"b".to_string()).unwrap().get(..).unwrap());
        assert_eq!("b  a      ".to_string(), line2.set(0, &"b".to_string()).unwrap().get(..).unwrap());
        assert_eq!("b  a     b".to_string(), line2.set(9, &"b".to_string()).unwrap().get(..).unwrap());
    }

    #[test]
    fn in_memory_line_generator() {
        let generator = LineGenerator;
        assert_eq!(Ok(Line::new_from_length(12)), generator.generate_line(12));
    }
}
