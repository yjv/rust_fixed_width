use std::borrow::Borrow;

pub struct VecStream<T: Borrow<str>> {
    pub vec: Vec<T>,
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