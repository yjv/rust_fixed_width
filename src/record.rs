use std::collections::{BTreeMap, HashMap, HashSet};
use std::collections::btree_map::{Iter as BTreeMapIter, IntoIter as BTreeMapIntoIter};
use std::collections::hash_map::{Iter as HashMapIter, IntoIter as HashMapIntoIter};
use std::ops::{Range, Index};
use std::iter::{FromIterator, Enumerate};
use std::error::Error;
use std::fmt::{Formatter, Display, Error as FmtError};

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
    fn insert(&mut self, name: String, range: Range<usize>);
    fn get<'a>(&self, name: &'a str) -> Option<Range<usize>>;
}

pub trait DataHolder where Self: Sized {
    fn new(data: Vec<u8>) -> Result<Self, DataHolderError>;
}

pub struct RecordType<T: DataRanges, U: DataHolder> {
    data_ranges: ::std::marker::PhantomData<T>,
    data_holder: ::std::marker::PhantomData<U>
}

impl<T: DataRanges, U: DataHolder> RecordType<T, U> {
    pub fn new() -> Self {
        RecordType {
            data_ranges: ::std::marker::PhantomData,
            data_holder: ::std::marker::PhantomData
        }
    }
}

pub trait WritableDataHolder {
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

impl<T: DataRanges, U: WritableDataHolder> Data<T, U> {
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

    pub fn push<'a>(&mut self, name: String, data: &'a [u8]) {
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

    fn insert(&mut self, name: String, range: Range<usize>) {
        self.insert(name, range);
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

    fn insert(&mut self, name: String, range: Range<usize>) {
        self.insert(name, range);
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

    fn insert(&mut self, _: String, value: Range<usize>) {
        self.push(value);
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

    fn insert(&mut self, _: String, value: Range<usize>) {
        self.insert(value);
    }

    fn get<'a>(&self, _: &'a str) -> Option<Range<usize>> {
        None
    }
}

impl DataRanges for () {
    fn new() -> Self {
        ()
    }

    fn insert(&mut self, _: String, _: Range<usize>) {}

    fn get<'a>(&self, _: &'a str) -> Option<Range<usize>> {
        None
    }
}

impl DataHolder for Vec<u8> {
    fn new(data: Vec<u8>) -> Result<Self, DataHolderError> {
        Ok(data)
    }
}

impl WritableDataHolder for Vec<u8> {
    fn get<'a>(&'a self, range: Range<usize>) -> &'a [u8] {
        &self[range]
    }
}

impl DataHolder for String {
    fn new(data: Vec<u8>) -> Result<Self, DataHolderError> {
        String::from_utf8(data).map_err(DataHolderError::new)
    }
}

impl WritableDataHolder for String {
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

#[cfg(test)]
mod test {

    use super::*;
    use std::collections::HashMap;
    use std::ops::Range;

    #[test]
    fn iteration() {
        let data  = Data {
            data: "hellohello2".as_bytes().to_owned(),
            ranges: [("field2".to_owned(), 0..5),
                ("field3".to_owned(), 5..11)]
                .iter().cloned().collect::<HashMap<String, Range<usize>>>()
        };
        for (name, field) in data.iter() {

        }
    }
}