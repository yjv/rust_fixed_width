use std::collections::{BTreeMap, HashMap, HashSet};
use std::collections::btree_map::{Iter as BTreeMapIter, IntoIter as BTreeMapIntoIter};
use std::collections::hash_map::{Iter as HashMapIter, IntoIter as HashMapIntoIter};
use std::ops::Range;
use std::iter::FromIterator;

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Data<T: DataRanges = HashMap<String, Range<usize>>> {
    pub data: Vec<u8>,
    pub ranges: T
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct Record<T: DataRanges = HashMap<String, Range<usize>>> {
    pub data: Data<T>,
    pub name: String
}

pub trait DataRanges {
    fn new() -> Self;
    fn insert(&mut self, name: String, range: Range<usize>);
    fn get<'a>(&self, name: &'a str) -> Option<Range<usize>>;
}

pub trait IterableDataRanges<'a>: DataRanges {
    type Iter: Iterator<Item=(&'a String, &'a Range<usize>)>;
    fn field_iter(&'a self) -> Self::Iter;
}

pub trait IntoIterableDataRanges: DataRanges {
    type Iter: Iterator<Item=(String, Range<usize>)>;
    fn into_field_iter(self) -> Self::Iter;
}

impl<T: DataRanges> Data<T> {
    pub fn get<'a>(&self, name: &'a str) -> Option<&[u8]> {
        self.ranges.get(name).map(|range| &self.data[range])
    }
}

pub struct Iter<'a, T: Iterator<Item=(&'a String, &'a Range<usize>)>> {
    iter: T,
    data: &'a Vec<u8>
}

impl<'a, T: Iterator<Item=(&'a String, &'a Range<usize>)>>  Iterator for Iter<'a, T> {
    type Item = (&'a String, &'a [u8]);
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(name, range)| (name, &self.data[range.clone()]))
    }
}

impl<'a, T: IterableDataRanges<'a>> Data<T> {
    pub fn iter(&'a self) -> Iter<'a, T::Iter> {
        Iter {
            iter: self.ranges.field_iter(),
            data: &self.data
        }
    }
}

pub struct IntoIter<T: Iterator<Item=(String, Range<usize>)>> {
    iter: T,
    data: Vec<u8>
}

impl<T: Iterator<Item=(String, Range<usize>)>>  Iterator for IntoIter<T> {
    type Item = (String, Vec<u8>);
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(name, range)| (name, self.data[range].to_owned()))
    }
}

impl<T: IntoIterableDataRanges> IntoIterator for Data<T> {
    type Item = (String, Vec<u8>);
    type IntoIter = IntoIter<T::Iter>;
    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            iter: self.ranges.into_field_iter(),
            data: self.data
        }
    }
}

impl<T: DataRanges> FromIterator<(String, Vec<u8>)> for Data<T> {
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