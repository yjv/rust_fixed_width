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
    fn get<'a>(&self, name: &'a str) -> Option<Range<usize>>;
}

pub trait BuildableDataRanges: DataRanges {
    fn new() -> Self;
    fn insert<'a>(&mut self, name: &'a str, range: Range<usize>);
}

impl<'a, T> DataRanges for &'a T where T: 'a + DataRanges {
    fn get<'b>(&self, name: &'b str) -> Option<Range<usize>> {
        (**self).get(name)
    }
}

pub trait ReadDataHolder {
    fn push<'a>(&mut self, data: &'a [u8]) -> Result<(), DataHolderError>;
}

pub enum ShouldReadMore {
    More(usize),
    NoMore
}

pub trait ReadType {
    type DataHolder;
    fn upcast_data(&self, data: Vec<u8>) -> Result<Self::DataHolder, DataHolderError>;
    fn get_range(&self, old_length: usize, data: &[u8]) -> Range<usize> {
        old_length..data.len()
    }
    fn should_read_more(&self, wanted_length: usize, data: &[u8]) -> ShouldReadMore {
        if wanted_length > data.len() {
            ShouldReadMore::More(wanted_length - data.len())
        } else {
            ShouldReadMore::NoMore
        }
    }
}

pub trait WriteType {
    type DataHolder;
    fn downcast_data<'a, T: DataRanges + 'a>(&self, data: &'a Data<T, Self::DataHolder>) -> Result<Data<&'a T, &'a [u8]>, DataHolderError>;
    fn get_data<'a>(&self, range: Range<usize>, data: &'a Self::DataHolder) -> Option<&'a [u8]>;
}

pub trait FieldData {
    fn get<'a>(&'a self, Range<usize>) -> Result<&'a [u8], DataHolderError>;
}

pub struct BinaryType;

impl ReadType for BinaryType {
    type DataHolder = Vec<u8>;
    fn upcast_data(&self, data: Vec<u8>) -> Result<Self::DataHolder, DataHolderError> {
        Ok(data)
    }
}

impl WriteType for BinaryType {
    type DataHolder = Vec<u8>;
    fn downcast_data<'a, T: DataRanges + 'a>(&self, data: &'a Data<T, Self::DataHolder>) -> Result<Data<&'a T, &'a [u8]>, DataHolderError> {
        Ok(data.internal_references())
    }

    fn get_data<'a>(&self, range: Range<usize>, data: &'a Self::DataHolder) -> Option<&'a [u8]> {
        Some(&data[range])
    }
}

pub struct StringType;

impl ReadType for StringType {
    type DataHolder = String;
    fn upcast_data(&self, data: Vec<u8>) -> Result<Self::DataHolder, DataHolderError> {
        Ok(String::from_utf8(data)?)
    }

    fn should_read_more(&self, wanted_length: usize, data: &[u8]) -> ShouldReadMore {
        let length = match ::std::str::from_utf8(data) {
            Ok(ref string) => string,
            Err(e) => unsafe {
                ::std::str::from_utf8_unchecked(&data[..e.valid_up_to()])
            }
        }.chars().count();

        if wanted_length > length {
            ShouldReadMore::More(wanted_length - length)
        } else {
            ShouldReadMore::NoMore
        }
    }
}

impl WriteType for StringType {
    type DataHolder = String;
    fn downcast_data<'a, T: DataRanges + 'a>(&self, data: &'a Data<T, Self::DataHolder>) -> Result<Data<&'a T, &'a [u8]>, DataHolderError> {
        Ok(Data { ranges: &data.ranges, data: data.data.as_ref() })
    }
    fn get_data<'a>(&self, range: Range<usize>, data: &'a Self::DataHolder) -> Option<&'a [u8]> {
        Some(data[range].as_bytes())
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

impl<'a, T: DataRanges + 'a> Data<T, Vec<u8>> {
    pub fn internal_references(&'a self) -> Data<&'a T, &'a [u8]> {
        Data { ranges: &self.ranges, data: &self.data[..] }
    }
}

impl<'a, T: DataRanges + 'a> Data<T, &'a [u8]> {
    pub fn get_write_data<'b>(&'b self, name: &'b str) -> Option<&'b [u8]> {
        self.ranges.get(name).map(|range| &self.data[range])
    }

    pub fn internal_references(&'a self) -> Data<&'a T, &'a [u8]> {
        Data { ranges: &self.ranges, data: &self.data[..] }
    }
}

impl <T: BuildableDataRanges> Data<T, Vec<u8>> {
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

impl<T: BuildableDataRanges> FromIterator<(String, Vec<u8>)> for Data<T, Vec<u8>> {
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

impl<'a, T: BuildableDataRanges> FromIterator<(&'a str, Vec<u8>)> for Data<T, Vec<u8>> {
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

impl<T: BuildableDataRanges> FromIterator<(String, String)> for Data<T, String> {
    fn from_iter<U: IntoIterator<Item = (String, String)>>(iter: U) -> Self {
        let mut ranges = T::new();
        let mut data = String::new();
        let mut current_index = 0;

        for (name, field) in iter {
            ranges.insert(&name, current_index..current_index + field.len());
            data.push_str(&field);
            current_index += field.len();
        }

        Data { data: data, ranges: ranges }
    }
}

impl<'a, T: BuildableDataRanges> FromIterator<(&'a str, String)> for Data<T, String> {
    fn from_iter<U: IntoIterator<Item = (&'a str, String)>>(iter: U) -> Self {
        let mut ranges = T::new();
        let mut data = String::new();
        let mut current_index = 0;

        for (name, field) in iter {
            ranges.insert(name, current_index..current_index + field.len());
            data.push_str(&field);
            current_index += field.len();
        }

        Data { data: data, ranges: ranges }
    }
}

impl DataRanges for BTreeMap<String, Range<usize>> {
    fn get<'a>(&self, name: &'a str) -> Option<Range<usize>> {
        self.get(name).cloned()
    }
}

impl BuildableDataRanges for BTreeMap<String, Range<usize>> {
    fn new() -> Self {
        BTreeMap::new()
    }

    fn insert<'a>(&mut self, name: &'a str, range: Range<usize>) {
        self.insert(name.to_owned(), range);
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
    fn get<'a>(&self, name: &'a str) -> Option<Range<usize>> {
        self.get(name).cloned()
    }
}

impl BuildableDataRanges for HashMap<String, Range<usize>> {
    fn new() -> Self {
        HashMap::new()
    }

    fn insert<'a>(&mut self, name: &'a str, range: Range<usize>) {
        self.insert(name.to_owned(), range);
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
    fn get<'a>(&self, _: &'a str) -> Option<Range<usize>> {
        None
    }
}

impl BuildableDataRanges for Vec<Range<usize>> {
    fn new() -> Self {
        Vec::new()
    }

    fn insert<'a>(&mut self, _: &'a str, range: Range<usize>) {
        self.push(range);
    }
}
//
//impl<'a> IterableDataRanges<'a> for Vec<Range<usize>> {
//    type Iter = Box<Iterator<Item=(&'a String, &'a Range<usize>)>>;
//    fn field_iter(&'a self) -> Box<Iterator<Item=(&'a String, &'a Range<usize>)>> {
//        Box::new(self.iter().map(|range| (&string.to_string(), range)))
//    }
//}

impl IntoIterableDataRanges for Vec<Range<usize>> {
    type Iter = Box<Iterator<Item=(String, Range<usize>)>>;
    fn into_field_iter(self) -> Box<Iterator<Item=(String, Range<usize>)>> {
        Box::new(self.into_iter().map(|range| ("".to_string(), range)))
    }
}

impl DataRanges for HashSet<Range<usize>> {
    fn get<'a>(&self, _: &'a str) -> Option<Range<usize>> {
        None
    }
}

impl BuildableDataRanges for HashSet<Range<usize>> {
    fn new() -> Self {
        HashSet::new()
    }

    fn insert<'a>(&mut self, _: &'a str, range: Range<usize>) {
        self.insert(range);
    }
}

impl DataRanges for () {
    fn get<'a>(&self, _: &'a str) -> Option<Range<usize>> {
        None
    }
}

impl BuildableDataRanges for () {
    fn new() -> Self {
        ()
    }

    fn insert<'a>(&mut self, _: &'a str, _: Range<usize>) {}
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

impl FieldData for Vec<u8> {
    fn get<'a>(&'a self, range: Range<usize>) -> Result<&'a [u8], DataHolderError> {
        Ok(&self[range])
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

impl FieldData for String {
    fn get<'a>(&'a self, range: Range<usize>) -> Result<&'a [u8], DataHolderError> {
        Ok(self[range].as_bytes())
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

impl From<HashMap<String, String>> for Data<HashMap<String, Range<usize>>, String> {
    fn from(data: HashMap<String, String>) -> Self {
        data.into_iter().collect()
    }
}

impl From<BTreeMap<String, String>> for Data<BTreeMap<String, Range<usize>>, String> {
    fn from(data: BTreeMap<String, String>) -> Self {
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