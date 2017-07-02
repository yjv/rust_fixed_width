use std::borrow::Borrow;
use ::BoxedErrorResult as Result;
use std::collections::HashMap;
use spec::RecordSpec;

pub struct VecStream<T: Borrow<str>> {
    vec: Vec<T>,
    pub position: usize
}

impl<T: Borrow<str>> From<Vec<T>> for VecStream<T> {
    fn from(vec: Vec<T>) -> Self {
        VecStream {
            vec: vec,
            position: 0
        }
    }
}

impl<T: Borrow<str>> VecStream<T> {
    pub fn next<'a>(&mut self, record_specs: &'a HashMap<String, RecordSpec>) -> Result<Option<&'a str>> {
        self.position += 1;

        Ok(match self.vec.get(self.position - 1).map(|v| v.borrow()) {
            None => None,
            Some(v) => {
                for (name, _) in record_specs.iter() {
                    if name == v {
                        return Ok(Some(name));
                    }
                }
                return Err("The nes".into())
            }
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use spec::{SpecBuilder, Builder};

    #[test]
    fn next() {
        let mut stream = VecStream::from(vec!["record1", "record2"]);
        let specs = SpecBuilder::new()
            .with_record("record1")
            .end()
            .with_record("record2")
            .end()
            .build()
            .unwrap()
            .record_specs
        ;
        assert_result!(Ok(Some("record1")), stream.next(&specs));
        assert_result!(Ok(Some("record2")), stream.next(&specs));
    }
}