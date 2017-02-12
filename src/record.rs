use std::collections::{BTreeMap, HashMap, HashSet};
use std::collections::btree_map::{Iter as BTreeMapIter, IntoIter as BTreeMapIntoIter};
use std::collections::hash_map::{Iter as HashMapIter, IntoIter as HashMapIntoIter};
//use std::slice::Iter as VecIter;
//use std::iter::Map;
use std::ops::Range;

pub trait RecordData {
    fn new() -> Self;
    fn insert(&mut self, name: String, value: Vec<u8>);
}

pub trait RecordRanges {
    fn new() -> Self;
    fn insert(&mut self, name: String, range: Range<usize>);
    fn get<'a>(&self, name: &'a str) -> Option<Range<usize>>;
}

pub trait IterableRecordRanges<'a>: RecordRanges {
    type Iter: Iterator<Item=(&'a String, &'a Range<usize>)>;
    fn field_iter(&'a self) -> Self::Iter;
}

pub trait IntoIterableRecordRanges: RecordRanges {
    type Iter: Iterator<Item=(String, Range<usize>)>;
    fn into_field_iter(self) -> Self::Iter;
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct Record<T: RecordRanges = HashMap<String, Range<usize>>> {
    pub data: Vec<u8>,
    pub name: String,
    pub ranges: T
}

impl<T: RecordRanges> Record<T> {
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

impl<'a, T: IterableRecordRanges<'a>> Record<T> {
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

impl<T: IntoIterableRecordRanges> IntoIterator for Record<T> {
    type Item = (String, Vec<u8>);
    type IntoIter = IntoIter<T::Iter>;
    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            iter: self.ranges.into_field_iter(),
            data: self.data
        }
    }
}

impl RecordRanges for BTreeMap<String, Range<usize>> {
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

impl<'a> IterableRecordRanges<'a> for BTreeMap<String, Range<usize>> {
    type Iter = BTreeMapIter<'a, String, Range<usize>>;
    fn field_iter(&'a self) -> BTreeMapIter<'a, String, Range<usize>> {
        self.iter()
    }
}

impl IntoIterableRecordRanges for BTreeMap<String, Range<usize>> {
    type Iter = BTreeMapIntoIter<String, Range<usize>>;
    fn into_field_iter(self) -> BTreeMapIntoIter<String, Range<usize>> {
        self.into_iter()
    }
}

impl RecordRanges for HashMap<String, Range<usize>> {
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

impl<'a> IterableRecordRanges<'a> for HashMap<String, Range<usize>> {
    type Iter = HashMapIter<'a, String, Range<usize>>;
    fn field_iter(&'a self) -> HashMapIter<'a, String, Range<usize>> {
        self.iter()
    }
}

impl IntoIterableRecordRanges for HashMap<String, Range<usize>> {
    type Iter = HashMapIntoIter<String, Range<usize>>;
    fn into_field_iter(self) -> HashMapIntoIter<String, Range<usize>> {
        self.into_iter()
    }
}

impl RecordRanges for Vec<Range<usize>> {
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

impl RecordRanges for HashSet<Range<usize>> {
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

impl RecordRanges for () {
    fn new() -> Self {
        ()
    }

    fn insert(&mut self, _: String, _: Range<usize>) {}

    fn get<'a>(&self, _: &'a str) -> Option<Range<usize>> {
        None
    }
}

