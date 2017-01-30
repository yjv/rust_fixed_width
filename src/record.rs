use std::collections::{BTreeMap, HashMap, HashSet, BTreeSet};

pub trait RecordData {
    fn new() -> Self;
    fn insert(&mut self, name: String, value: String);
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct Record<T: RecordData> {
    pub data: T,
    pub name: String
}

impl RecordData for BTreeMap<String, String> {
    fn new() -> Self {
        BTreeMap::new()
    }

    fn insert(&mut self, name: String, value: String) {
        self.insert(name, value);
    }
}

impl RecordData for HashMap<String, String> {
    fn new() -> Self {
        HashMap::new()
    }

    fn insert(&mut self, name: String, value: String) {
        self.insert(name, value);
    }
}

impl RecordData for Vec<String> {
    fn new() -> Self {
        Vec::new()
    }

    fn insert(&mut self, _: String, value: String) {
        self.push(value);
    }
}

impl RecordData for HashSet<String> {
    fn new() -> Self {
        HashSet::new()
    }

    fn insert(&mut self, _: String, value: String) {
        self.insert(value);
    }
}

impl RecordData for BTreeSet<String> {
    fn new() -> Self {
        BTreeSet::new()
    }

    fn insert(&mut self, _: String, value: String) {
        self.insert(value);
    }
}
