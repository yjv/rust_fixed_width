use std::string::ToString;
use std::ops::{Range as RangeStruct, RangeFull, RangeFrom, RangeTo};
use std::fmt::Debug;

pub trait File: ToString {
    type Line: Line;
    type Error: Debug;
    fn name(&self) -> &str;
    fn width(&self) -> usize;
    fn line_seperator(&self) -> &str;
    fn lines(&self) -> &Vec<Self::Line>;
    fn line(&self, index: usize) -> Result<&Self::Line, Self::Error>;
    fn add_line<T: Line>(&mut self, line: T) -> Result<(), Self::Error>;
    fn set_line<T: Line>(&mut self, index: usize, line: T) -> Result<(), Self::Error>;
    fn remove_line(&mut self, index: usize) -> Result<(), Self::Error>;
    fn length(&self) -> usize;
}

pub trait Line: ToString {
    type Error: Debug;
    fn length(&self) -> usize;
    fn get<T: Range>(&self, range: T) -> Result<String, Self::Error>;
    fn set<T: Range>(&mut self, range: T, string: &String) -> Result<(), Self::Error>;
    fn remove<T: Range>(&mut self, range: T) -> Result<(), Self::Error>;
    fn is_range_valid<T: Range>(&self, range: T) -> bool {
        let end  = range.end();
        return end.is_none() || end.unwrap() < self.length() + 1;
    }
}

pub trait Range {
    fn start(&self) -> Option<usize>;
    fn end(&self) -> Option<usize>;
}

impl Range for RangeStruct<usize> {
    fn start(&self) -> Option<usize> {
        Some(self.start)
    }

    fn end(&self) -> Option<usize> {
        Some(self.end)
    }
}

impl Range for RangeFull {
    fn start(&self) -> Option<usize> {
        None
    }

    fn end (&self) -> Option<usize> {
        None
    }
}

impl Range for RangeFrom<usize> {
    fn start(&self) -> Option<usize> {
        Some(self.start)
    }

    fn end(&self) -> Option<usize> {
        None
    }
}

impl Range for RangeTo<usize> {
    fn start(&self) -> Option<usize> {
        None
    }

    fn end(&self) -> Option<usize> {
        Some(self.end)
    }
}

impl Range for usize {
    fn start(&self) -> Option<usize> {
        Some(*self)
    }

    fn end(&self) -> Option<usize> {
        Some(*self + 1)
    }
}

    #[cfg(test)]
mod test {

    use super::Range;
    use std::ops::{Range as RangeStruct, RangeFull, RangeFrom, RangeTo};

    #[test]
    fn ranges() {
        let range1 = RangeStruct { start: 2, end: 5 };
        let range2 = RangeFull;
        let range3 = RangeFrom { start: 4 };
        let range4 = RangeTo { end: 8 };
        let range5: usize = 4;
        assert_eq!(Some(2), range1.start());
        assert_eq!(Some(5), range1.end());
        assert_eq!(None, range2.start());
        assert_eq!(None, range2.end());
        assert_eq!(Some(4), range3.start());
        assert_eq!(None, range3.end());
        assert_eq!(None, range4.start());
        assert_eq!(Some(8), range4.end());
        assert_eq!(Some(4), range5.start());
        assert_eq!(Some(5), range5.end());
    }


}