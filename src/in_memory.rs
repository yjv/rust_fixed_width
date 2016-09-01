use std::string::ToString;
use std::iter::repeat;
use common::{File as FileTrait, Line as LineTrait, Range, normalize_range, InvalidRangeError};

pub struct File {
    name: String,
    width: usize,
    lines: Vec<Line>,
    line_separator: String
}

impl File {
    pub fn new(name: String, width: usize) -> Self {
        Self::new_with_line_separator(name, width, "\r\n".to_string())
    }

    pub fn new_with_line_separator(name: String, width: usize, line_separator: String) -> Self {
        File {
            name: name,
            width: width,
            lines: Vec::new(),
            line_separator: line_separator
        }
    }
}

impl FileTrait for File {
    type Line = Line;
    type Error = ();
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

    fn add_line(&mut self) -> Result<usize, Self::Error> {
        self.lines.push(Line::new(self.width));
        Ok(self.lines.len() - 1)
    }

    fn remove_line(&mut self) -> Result<usize, Self::Error> {
        self.lines.pop();
        Ok(self.lines.len())
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
    pub fn new(length: usize) -> Self {
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

    fn clear<T: Range>(&mut self, range: T) -> Result<&mut Self, Self::Error> {
        let (start, end) = try!(normalize_range(range.clone(), self).map_err(LineError::InvalidRange));
        self.set(range, &repeat(" ").take(end - start).collect())
    }
}

impl ToString for Line {
    fn to_string(&self) -> String {
        self.data.clone()
    }
}

#[cfg(test)]
mod test {

    use super::File;
    use super::super::common::{Line as LineTrait, File as FileTrait, FileIterator};
    use std::iter::repeat;
    use std::iter::Iterator;
    use std::string::ToString;

    #[test]
    fn in_memory_file() {
        let mut file = File::new("bla".to_string(), 10);
        assert_eq!("bla", file.name());
        let line1 = repeat("a").take(10).collect::<String>();
        let line2 = repeat(" ").take(10).collect::<String>();
        let line3 = repeat("c").take(10).collect::<String>();
        let index1 = file.add_line().unwrap();
        let _ = file.line_mut(index1).unwrap().unwrap().set(.., &line1);
        let index2 = file.add_line().unwrap();
        let _ = file.line_mut(index2).unwrap().unwrap().set(.., &line2);
        let index3 = file.add_line().unwrap();
        let _ = file.line_mut(index3).unwrap().unwrap().set(.., &line3);
        assert_eq!(line1, file.line(index1).unwrap().unwrap().to_string());
        assert_eq!(line2, file.line(index2).unwrap().unwrap().to_string());
        assert_eq!(line3, file.line(index3).unwrap().unwrap().to_string());
        assert_eq!(None, file.line(3).unwrap());
        assert_eq!(vec![line1.clone(), line2.clone(), line3.clone()], FileIterator::new(&file).map(|r| r.unwrap().to_string()).collect::<Vec<String>>());
        assert_eq!(3, file.len());
        assert_eq!("aaaaaaaaaa\r\n          \r\ncccccccccc".to_string(), file.to_string());
        assert_eq!(10, file.line(index1).unwrap().unwrap().len());
        assert_eq!(line1, file.line(index1).unwrap().unwrap().get(..).unwrap());
        assert_eq!("aaaa".to_string(), file.line(index1).unwrap().unwrap().get(1..5).unwrap());
        assert_eq!(line2, file.line(index2).unwrap().unwrap().get(..).unwrap());
        assert_eq!("abbbbaaaaa".to_string(), file.line_mut(index1).unwrap().unwrap().set(1..5, &"bbbb".to_string()).unwrap().get(..).unwrap());
        assert_eq!("abbbba  aa".to_string(), file.line_mut(index1).unwrap().unwrap().clear(6..8).unwrap().get(..).unwrap());
        assert_eq!("   a      ".to_string(), file.line_mut(index2).unwrap().unwrap().set(3, &"a".to_string()).unwrap().get(..).unwrap());
        assert_eq!("abbbba b a".to_string(), file.line_mut(index1).unwrap().unwrap().set(7..9, &"b".to_string()).unwrap().get(..).unwrap());
        assert_eq!("b  a      ".to_string(), file.line_mut(index2).unwrap().unwrap().set(0, &"b".to_string()).unwrap().get(..).unwrap());
        assert_eq!("b  a     b".to_string(), file.line_mut(index2).unwrap().unwrap().set(9, &"b".to_string()).unwrap().get(..).unwrap());
        assert_eq!(2, file.remove_line().unwrap());
        assert_eq!("abbbba b a".to_string(), file.line(index1).unwrap().unwrap().to_string());
        assert_eq!(None, file.line(index3).unwrap());
        assert_eq!("abbbba b a\r\nb  a     b".to_string(), file.to_string());
    }
}
