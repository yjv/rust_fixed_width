use std::ops::Range;
use record::{DataRanges, Data};
use super::BoxedErrorResult as Result;

pub enum ShouldReadMore {
    More(usize),
    NoMore
}

pub trait DataSupporter {
    type DataHolder;
    fn get_length(&self, data: &[u8]) -> Length {
        Length { length: data.len(), remainder: 0 }
    }

    fn get_byte_range(&self, _: &[u8], range: Range<usize>) -> Option<Range<usize>> {
        Some(range)
    }

    fn get_size_hint(&self, length: usize) -> (usize, Option<usize>) {
        (length, None)
    }
}

pub struct Length {
    pub length: usize,
    pub remainder: usize
}

pub trait FieldReadSupporter: DataSupporter {
    fn should_read_more(&self, wanted_length: usize, data: &[u8]) -> ShouldReadMore {
        let length = self.get_length(data).length;
        if wanted_length > length {
            ShouldReadMore::More(wanted_length - length)
        } else {
            ShouldReadMore::NoMore
        }
    }
}

pub trait RecordReadSupporter: FieldReadSupporter {
    fn upcast_data(&self, data: Vec<u8>) -> Result<Self::DataHolder>;
    fn get_range(&self, old_length: usize, data: &[u8]) -> Range<usize> {
        old_length..data.len()
    }
}

pub trait WriteSupporter: DataSupporter {
    fn get_data<'a>(&self, range: Range<usize>, data: &'a Self::DataHolder) -> Option<&'a [u8]>;
    fn get_data_by_name<'a, T: DataRanges + 'a>(&self, name: &'a str, data: &'a Data<T, Self::DataHolder>) -> Option<&'a [u8]> {
        data.ranges.get(name).and_then(|range| self.get_data(range, &data.data))
    }
}

pub struct BinarySupporter;

impl DataSupporter for BinarySupporter {
    type DataHolder = Vec<u8>;

    fn get_size_hint(&self, length: usize) -> (usize, Option<usize>) {
        (length, Some(length))
    }
}

impl FieldReadSupporter for BinarySupporter {}

impl RecordReadSupporter for BinarySupporter {
    fn upcast_data(&self, data: Vec<u8>) -> Result<Self::DataHolder> {
        Ok(data)
    }
}

impl WriteSupporter for BinarySupporter {
    fn get_data<'a>(&self, range: Range<usize>, data: &'a Self::DataHolder) -> Option<&'a [u8]> {
        Some(&data[range])
    }
}

pub struct StringSupporter;

impl StringSupporter {
    fn get_string<'a>(&self, data: &'a [u8]) -> &'a str {
        match ::std::str::from_utf8(data) {
            Ok(ref string) => string,
            Err(e) => unsafe {
                ::std::str::from_utf8_unchecked(&data[..e.valid_up_to()])
            }
        }
    }
}

impl DataSupporter for StringSupporter {
    type DataHolder = String;
    fn get_length(&self, data: &[u8]) -> Length {
        let string = self.get_string(data);

        Length {
            length: string.chars().count(),
            remainder: data.len() - string.len()
        }
    }

    fn get_byte_range(&self, data: &[u8], range: Range<usize>) -> Option<Range<usize>> {
        let mut iterator = self.get_string(data).char_indices();

        match (iterator.nth(range.start), iterator.nth(range.end - 1 - range.start)) {
            (Some((start, _)), Some((end, _))) => Some(start..end + 1),
            _ => None
        }
    }

    fn get_size_hint(&self, length: usize) -> (usize, Option<usize>) {
        (length, Some(length * 4))
    }
}

impl FieldReadSupporter for StringSupporter {
    fn should_read_more(&self, wanted_length: usize, data: &[u8]) -> ShouldReadMore {
        let length = self.get_length(data).length;

        if wanted_length > length {
            ShouldReadMore::More(wanted_length - length)
        } else {
            ShouldReadMore::NoMore
        }
    }
}

impl RecordReadSupporter for StringSupporter {
    fn upcast_data(&self, data: Vec<u8>) -> Result<Self::DataHolder> {
        Ok(String::from_utf8(data)?)
    }
}

impl WriteSupporter for StringSupporter {
    fn get_data<'a>(&self, range: Range<usize>, data: &'a Self::DataHolder) -> Option<&'a [u8]> {
        Some(data[range].as_bytes())
    }
}
