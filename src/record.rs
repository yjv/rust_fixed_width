use std::collections::{BTreeMap, HashMap, HashSet};
use std::collections::btree_map::{Iter as BTreeMapIter, IntoIter as BTreeMapIntoIter};
use std::collections::hash_map::{Iter as HashMapIter, IntoIter as HashMapIntoIter};
use std::ops::{Range, Index};
use std::iter::{FromIterator, Enumerate};
use std::error::Error;
use std::fmt::{Formatter, Display, Error as FmtError};
use std::string::FromUtf8Error;
use std::str::Utf8Error;

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Data<T: DataRanges, U> {
    pub ranges: T,
    pub data: U
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct Record<T: DataRanges, U> {
    pub data: Data<T, U>,
    pub name: String
}

pub trait DataRanges {
    fn new() -> Self;
    fn insert<'a>(&mut self, name: &'a str, range: Range<usize>);
    fn get<'a>(&self, name: &'a str) -> Option<Range<usize>>;
}

pub trait ReadDataHolder where Self: Sized {
    fn push<'a>(&mut self, data: &'a [u8]) -> Result<(), DataHolderError>;
}

pub trait DataType {
    type ReadDataHolder: ReadDataHolder;
    type WriteDataHolder: WriteDataHolder;
    fn new_data_holder(&self, data: Vec<u8>) -> Result<Self::ReadDataHolder, DataHolderError>;
}

pub struct BinaryType;

impl DataType for BinaryType {
    type ReadDataHolder = Vec<u8>;
    type WriteDataHolder = Vec<u8>;
    fn new_data_holder(&self, data: Vec<u8>) -> Result<Self::ReadDataHolder, DataHolderError> {
        Ok(data)
    }
}

pub struct StringType;

impl DataType for StringType {
    type ReadDataHolder = String;
    type WriteDataHolder = String;
    fn new_data_holder(&self, data: Vec<u8>) -> Result<Self::ReadDataHolder, DataHolderError> {
        Ok(String::from_utf8(data)?)
    }
}

pub trait WriteDataHolder {
    fn get<'a>(&'a self, range: Range<usize>) -> &'a [u8];
}

#[derive(Debug)]
pub struct DataHolderError {
    repr: Box<::std::error::Error + Send + Sync>
}

impl Clone for DataHolderError {
    fn clone(&self) -> Self {
        DataHolderError::new("")
    }
}

impl DataHolderError {
    pub fn new<E>(error: E) -> Self
        where E: Into<Box<Error + Send + Sync>>
    {
        DataHolderError { repr: error.into() }
    }

    pub fn downcast<E: Error + Send + Sync + 'static>(self) -> Result<E, Self> {
        Ok(*(self.repr.downcast::<E>().map_err(|e| DataHolderError { repr: e })?))
    }

    pub fn downcast_ref<E: Error + Send + Sync + 'static>(&self) -> Option<&E> {
        self.repr.downcast_ref::<E>()
    }
}

impl Error for DataHolderError {
    fn description(&self) -> &str {
        self.repr.description()
    }

    fn cause(&self) -> Option<&Error> {
        self.repr.cause()
    }
}

impl Display for DataHolderError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        Display::fmt(&*self.repr, f)
    }
}

impl From<FromUtf8Error> for DataHolderError {
    fn from(e: FromUtf8Error) -> Self {
        DataHolderError::new(e)
    }
}

impl From<Utf8Error> for DataHolderError {
    fn from(e: Utf8Error) -> Self {
        DataHolderError::new(e)
    }
}

pub trait IterableDataRanges<'a>: DataRanges {
    type Iter: Iterator<Item=(&'a String, &'a Range<usize>)>;
    fn field_iter(&'a self) -> Self::Iter;
}

pub trait IntoIterableDataRanges: DataRanges {
    type Iter: Iterator<Item=(String, Range<usize>)>;
    fn into_field_iter(self) -> Self::Iter;
}

impl<T: DataRanges, U: Index<Range<usize>>> Data<T, U> {
    pub fn get<'a>(&self, name: &'a str) -> Option<&U::Output> {
        self.ranges.get(name).map(|range| &self.data[range])
    }
}

impl<T: DataRanges, U: WriteDataHolder> Data<T, U> {
    pub fn get_writable_data<'a>(&'a self, name: &'a str) -> Option<&'a [u8]> {
        self.ranges.get(name).map(|range| self.data.get(range))
    }
}

impl <T: DataRanges> Data<T, Vec<u8>> {
    pub fn new() -> Self {
        Data {
            ranges: T::new(),
            data: Vec::new()
        }
    }

    pub fn push<'a>(&mut self, name: &'a str, data: &'a [u8]) {
        self.ranges.insert(name, self.data.len()..self.data.len() + data.len());
        self.data.extend(data);
    }
}

pub struct Iter<'a, T: Iterator<Item=(&'a String, &'a Range<usize>)>, U: Index<Range<usize>> + 'a> {
    iter: T,
    data: &'a U
}

impl<'a, T: Iterator<Item=(&'a String, &'a Range<usize>)>, U: Index<Range<usize>> + 'a>  Iterator for Iter<'a, T, U> {
    type Item = (&'a String, &'a U::Output);
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(name, range)| (name, &self.data[range.clone()]))
    }
}

impl<'a, T: IterableDataRanges<'a>, U: Index<Range<usize>> + 'a> Data<T, U> {
    pub fn iter(&'a self) -> Iter<'a, T::Iter, U> {
        Iter {
            iter: self.ranges.field_iter(),
            data: &self.data
        }
    }
}

pub struct IntoIter<T: Iterator<Item=(String, Range<usize>)>, U: ToOwned, V: Index<Range<usize>, Output=U>> {
    iter: T,
    data: V,
    marker: ::std::marker::PhantomData<U>
}

impl<T: Iterator<Item=(String, Range<usize>)>, U: ToOwned, V: Index<Range<usize>, Output=U>>  Iterator for IntoIter<T, U, V> {
    type Item = (String, U::Owned);
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(name, range)| (name, self.data[range].to_owned()))
    }
}

impl<T: IntoIterableDataRanges, U: ToOwned, V: Index<Range<usize>, Output=U>> IntoIterator for Data<T, V> {
    type Item = (String, U::Owned);
    type IntoIter = IntoIter<T::Iter, U, V>;
    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            iter: self.ranges.into_field_iter(),
            data: self.data,
            marker: ::std::marker::PhantomData
        }
    }
}

impl<T: DataRanges> FromIterator<(String, Vec<u8>)> for Data<T, Vec<u8>> {
    fn from_iter<U: IntoIterator<Item = (String, Vec<u8>)>>(iter: U) -> Self {
        let mut ranges = T::new();
        let mut data = Vec::new();
        let mut current_index = 0;

        for (name, field) in iter {
            ranges.insert(&name, current_index..current_index + field.len());
            data.extend(field.iter());
            current_index += field.len();
        }

        Data { data: data, ranges: ranges }
    }
}

impl<'a, T: DataRanges> FromIterator<(&'a str, Vec<u8>)> for Data<T, Vec<u8>> {
    fn from_iter<U: IntoIterator<Item = (&'a str, Vec<u8>)>>(iter: U) -> Self {
        let mut ranges = T::new();
        let mut data = Vec::new();
        let mut current_index = 0;

        for (name, field) in iter {
            ranges.insert(name, current_index..current_index + field.len());
            data.extend(field.iter());
            current_index += field.len();
        }

        Data { data: data, ranges: ranges }
    }
}

impl DataRanges for BTreeMap<String, Range<usize>> {
    fn new() -> Self {
        BTreeMap::new()
    }

    fn insert<'a>(&mut self, name: &'a str, range: Range<usize>) {
        self.insert(name.to_owned(), range);
    }

    fn get<'a>(&self, name: &'a str) -> Option<Range<usize>> {
        self.get(name).cloned()
    }
}

impl<'a> IterableDataRanges<'a> for BTreeMap<String, Range<usize>> {
    type Iter = BTreeMapIter<'a, String, Range<usize>>;
    fn field_iter(&'a self) -> BTreeMapIter<'a, String, Range<usize>> {
        self.iter()
    }
}

impl IntoIterableDataRanges for BTreeMap<String, Range<usize>> {
    type Iter = BTreeMapIntoIter<String, Range<usize>>;
    fn into_field_iter(self) -> BTreeMapIntoIter<String, Range<usize>> {
        self.into_iter()
    }
}

impl DataRanges for HashMap<String, Range<usize>> {
    fn new() -> Self {
        HashMap::new()
    }

    fn insert<'a>(&mut self, name: &'a str, range: Range<usize>) {
        self.insert(name.to_owned(), range);
    }

    fn get<'a>(&self, name: &'a str) -> Option<Range<usize>> {
        self.get(name).cloned()
    }
}

impl<'a> IterableDataRanges<'a> for HashMap<String, Range<usize>> {
    type Iter = HashMapIter<'a, String, Range<usize>>;
    fn field_iter(&'a self) -> HashMapIter<'a, String, Range<usize>> {
        self.iter()
    }
}

impl IntoIterableDataRanges for HashMap<String, Range<usize>> {
    type Iter = HashMapIntoIter<String, Range<usize>>;
    fn into_field_iter(self) -> HashMapIntoIter<String, Range<usize>> {
        self.into_iter()
    }
}

impl DataRanges for Vec<Range<usize>> {
    fn new() -> Self {
        Vec::new()
    }

    fn insert<'a>(&mut self, _: &'a str, range: Range<usize>) {
        self.push(range);
    }

    fn get<'a>(&self, _: &'a str) -> Option<Range<usize>> {
        None
    }
}
//
//pub struct StringEnumerateIterator {
//    iter: Enumerate<(usize, Range<usize>)>
//}
//
//impl Iterator for StringEnumerateIterator {
//    type Item = (String, Range<usize>);
//    fn next(&mut self) -> Option<Self::Item> {
//        self.iter.next().map(|(key, value)| (key.to_string(), value))
//    }
//}
//
//pub struct StrEnumerateIterator {
//    iter: Enumerate<(usize, Range<usize>)>,
//    key: Option
//}
//
//impl Iterator for StringEnumerateIterator {
//    type
//    fn next(&mut self) -> Option<Self::Item> {
//        self.iter.next().map(|(key, value)| (key.to_string(), value))
//    }
//}
//
//impl IterableDataRanges for Vec<Range<usize>> {
//
//}

impl DataRanges for HashSet<Range<usize>> {
    fn new() -> Self {
        HashSet::new()
    }

    fn insert<'a>(&mut self, _: &'a str, range: Range<usize>) {
        self.insert(range);
    }

    fn get<'a>(&self, _: &'a str) -> Option<Range<usize>> {
        None
    }
}

impl DataRanges for () {
    fn new() -> Self {
        ()
    }

    fn insert<'a>(&mut self, _: &'a str, _: Range<usize>) {}

    fn get<'a>(&self, _: &'a str) -> Option<Range<usize>> {
        None
    }
}

impl ReadDataHolder for Vec<u8> {
    fn push<'a>(&mut self, data: &'a [u8]) -> Result<(), DataHolderError> {
        Ok(self.extend(data))
    }
}

impl WriteDataHolder for Vec<u8> {
    fn get<'a>(&'a self, range: Range<usize>) -> &'a [u8] {
        &self[range]
    }
}

impl ReadDataHolder for String {
    fn push<'a>(&mut self, data: &'a [u8]) -> Result<(), DataHolderError> {
        Ok(self.push_str(::std::str::from_utf8(data)?))
    }
}

impl WriteDataHolder for String {
    fn get<'a>(&'a self, range: Range<usize>) -> &'a [u8] {
        self[range].as_ref()
    }
}

impl From<HashMap<String, Vec<u8>>> for Data<HashMap<String, Range<usize>>, Vec<u8>> {
    fn from(data: HashMap<String, Vec<u8>>) -> Self {
        data.into_iter().collect()
    }
}

impl From<BTreeMap<String, Vec<u8>>> for Data<BTreeMap<String, Range<usize>>, Vec<u8>> {
    fn from(data: BTreeMap<String, Vec<u8>>) -> Self {
        data.into_iter().collect()
    }
}

//#[cfg(test)]
//mod test {
//
//    use super::*;
//    use std::collections::HashMap;
//    use std::ops::Range;
//
//    #[test]
//    fn iteration() {
//        let data  = Data {
//            data: "hellohello2".as_bytes().to_owned(),
//            ranges: [("field2".to_owned(), 0..5),
//                ("field3".to_owned(), 5..11)]
//                .iter().cloned().collect::<HashMap<String, Range<usize>>>()
//        };
//        for (name, field) in data.iter() {
//
//        }
//    }
//}