use std::string::ToString;
use std::iter::repeat;
use common::{File as FileTrait, Range, normalize_range, InvalidRangeError, FileError};

pub struct File {
    width: usize,
    lines: Vec<String>,
    line_separator: String
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    DataLongerThanRange,
    InvalidRange(InvalidRangeError),
    InvalidIndex(usize)
}

type Result<T> = ::std::result::Result<T, Error>;

impl FileError for Error {
    fn is_invalid_index(&self) -> bool {
        return match self {
            &Error::InvalidIndex(_) => true,
            _ => false
        }
    }

    fn is_invalid_range(&self) -> bool {
        return match self {
            &Error::InvalidRange(_) => true,
            _ => false
        }
    }
}

impl File {
    pub fn new(width: usize) -> Self {
        Self::new_with_line_separator(width, "\r\n".to_string())
    }

    pub fn new_with_line_separator(width: usize, line_separator: String) -> Self {
        File {
            width: width,
            lines: Vec::new(),
            line_separator: line_separator
        }
    }
}

impl FileTrait for File {
    type Error = Error;
    fn width(&self) -> usize {
        self.width
    }

    fn get<T: Range>(&self, index: usize, range: T) -> Result<String> {
        let line = try!(self.lines.get(index).ok_or(Error::InvalidIndex(index)));
        let (start, end) = try!(normalize_range(range, self.width, None).map_err(Error::InvalidRange));
        Ok(line[start..end].to_string())
    }

    fn set<T: Range>(&mut self, index: usize, range: T, string: &String) -> Result<&mut Self> {
        let line = try!(self.lines.get_mut(index).ok_or(Error::InvalidIndex(index)));
        let (start, end) = try!(normalize_range(range, self.width, Some(string)).map_err(Error::InvalidRange));
        if string.len() > end - start {
            Err(Error::DataLongerThanRange)
        } else {
            let data = line.clone();
            line.truncate(0);
            line.push_str(&data[..start]);
            line.push_str(&string[..]);
            line.push_str(&repeat(" ").take(end - start - string.len()).collect::<String>()[..]);
            line.push_str(&data[end..]);
            Ok(self)
        }
    }

    fn clear<T: Range>(&mut self, index: usize, range: T) -> Result<&mut Self> {
        let (start, end) = try!(normalize_range(range, self.width, None).map_err(Error::InvalidRange));
        self.set(index, start..end, &repeat(" ").take(end - start).collect())
    }

    fn add_line(&mut self) -> Result<usize> {
        self.lines.push(repeat(" ").take(self.width).collect::<String>());
        Ok(self.lines.len() - 1)
    }

    fn remove_line(&mut self) -> Result<usize> {
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

            string.push_str(&line[..])
        }

        string
    }
}

#[cfg(test)]
mod test {

    use super::{File, Error};
    use super::super::common::{File as FileTrait, FileIterator};
    use std::iter::repeat;
    use std::iter::Iterator;
    use std::string::ToString;

    #[test]
    fn in_memory_file() {
        let mut file = File::new(10);
        let line1 = repeat("a").take(10).collect::<String>();
        let line2 = repeat(" ").take(10).collect::<String>();
        let line3 = repeat("c").take(10).collect::<String>();
        let index1 = file.add_line().unwrap();
        let _ = file.set(index1, .., &line1);
        let index2 = file.add_line().unwrap();
        let _ = file.set(index2, .., &line2);
        let index3 = file.add_line().unwrap();
        let _ = file.set(index3, .., &line3);
        assert_eq!(line1, file.get(index1, ..).unwrap());
        assert_eq!(line2, file.get(index2, ..).unwrap());
        assert_eq!(line3, file.get(index3, ..).unwrap());
        assert_eq!(Error::InvalidIndex(3), file.get(3, ..).unwrap_err());
        assert_eq!(vec![line1.clone(), line2.clone(), line3.clone()], FileIterator::new(&file).map(|r| r.unwrap()).collect::<Vec<String>>());
        assert_eq!(3, file.len());
        assert_eq!("aaaaaaaaaa\r\n          \r\ncccccccccc".to_string(), file.to_string());
        assert_eq!(line1, file.get(index1, ..).unwrap());
        assert_eq!("aaaa".to_string(), file.get(index1, 1..5).unwrap());
        assert_eq!(line2, file.get(index2, ..).unwrap());
        assert_eq!("abbbbaaaaa".to_string(), file.set(index1, 1..5, &"bbbb".to_string()).unwrap().get(index1, ..).unwrap());
        assert_eq!("abbbba  aa".to_string(), file.clear(index1, 6..8).unwrap().get(index1, ..).unwrap());
        assert_eq!("   a      ".to_string(), file.set(index2, 3, &"a".to_string()).unwrap().get(index2, ..).unwrap());
        assert_eq!("abbbba b a".to_string(), file.set(index1, 7..9, &"b".to_string()).unwrap().get(index1, ..).unwrap());
        assert_eq!("b  a      ".to_string(), file.set(index2, 0, &"b".to_string()).unwrap().get(index2, ..).unwrap());
        assert_eq!("b  a     b".to_string(), file.set(index2, 9, &"b".to_string()).unwrap().get(index2, ..).unwrap());
        assert_eq!(2, file.remove_line().unwrap());
        assert_eq!("abbbba b a".to_string(), file.get(index1, ..).unwrap());
        assert_eq!(Error::InvalidIndex(index3), file.get(index3, ..).unwrap_err());
        assert_eq!("abbbba b a\r\nb  a     b".to_string(), file.to_string());
    }
}
