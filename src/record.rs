use std::collections::{BTreeMap, HashMap, HashSet, BTreeSet};

pub trait RecordData {
    fn new() -> Self;
    fn insert(&mut self, name: String, value: Vec<u8>);
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct Record<T: RecordData> {
    pub data: T,
    pub name: String
}

impl RecordData for BTreeMap<String, Vec<u8>> {
    fn new() -> Self {
        BTreeMap::new()
    }

    fn insert(&mut self, name: String, value: Vec<u8>) {
        self.insert(name, value);
    }
}

impl RecordData for HashMap<String, Vec<u8>> {
    fn new() -> Self {
        HashMap::new()
    }

    fn insert(&mut self, name: String, value: Vec<u8>) {
        self.insert(name, value);
    }
}

impl RecordData for Vec<Vec<u8>> {
    fn new() -> Self {
        Vec::new()
    }

    fn insert(&mut self, _: String, value: Vec<u8>) {
        self.push(value);
    }
}

impl RecordData for HashSet<Vec<u8>> {
    fn new() -> Self {
        HashSet::new()
    }

    fn insert(&mut self, _: String, value: Vec<u8>) {
        self.insert(value);
    }
}

impl RecordData for BTreeSet<Vec<u8>> {
    fn new() -> Self {
        BTreeSet::new()
    }

    fn insert(&mut self, _: String, value: Vec<u8>) {
        self.insert(value);
    }
}
