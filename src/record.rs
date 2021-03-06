use std::collections::{BTreeMap, HashMap};
use std::collections::btree_map::{Iter as BTreeMapIter, IntoIter as BTreeMapIntoIter};
use std::collections::hash_map::{Iter as HashMapIter, IntoIter as HashMapIntoIter};
use std::ops::{Range, Index};
use std::iter::FromIterator;

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Data<T: DataRanges, U> {
    pub ranges: T,
    pub data: U
}

#[derive(Clone, Eq, PartialEq, Debug)]
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
        DataRanges::get(*self, name)
    }
}

pub trait IterableDataRanges<'a>: DataRanges {
    type Iter: Iterator<Item=(&'a String, &'a Range<usize>)>;
    fn range_iter(&'a self) -> Self::Iter;
}

pub trait IntoIterableDataRanges: DataRanges {
    type Iter: Iterator<Item=(String, Range<usize>)>;
    fn into_range_iter(self) -> Self::Iter;
}

impl<T: DataRanges, U: Index<Range<usize>>> Data<T, U> {
    pub fn get<'a>(&self, name: &'a str) -> Option<&U::Output> {
        self.ranges.get(name).map(|range| &self.data[range])
    }
}

impl <T: BuildableDataRanges> Data<T, Vec<u8>> {
    pub fn new() -> Self {
        Data {
            ranges: T::new(),
            data: Vec::new()
        }
    }

    pub fn push<'a, U: AsRef<[u8]> + 'a>(&mut self, name: &'a str, data: U) {
        let data = data.as_ref();
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
            iter: self.ranges.range_iter(),
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
            iter: self.ranges.into_range_iter(),
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
    fn range_iter(&'a self) -> BTreeMapIter<'a, String, Range<usize>> {
        self.iter()
    }
}

impl IntoIterableDataRanges for BTreeMap<String, Range<usize>> {
    type Iter = BTreeMapIntoIter<String, Range<usize>>;
    fn into_range_iter(self) -> BTreeMapIntoIter<String, Range<usize>> {
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
    fn range_iter(&'a self) -> HashMapIter<'a, String, Range<usize>> {
        self.iter()
    }
}

impl IntoIterableDataRanges for HashMap<String, Range<usize>> {
    type Iter = HashMapIntoIter<String, Range<usize>>;
    fn into_range_iter(self) -> HashMapIntoIter<String, Range<usize>> {
        self.into_iter()
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