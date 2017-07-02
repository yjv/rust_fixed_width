use std::borrow::Borrow;

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
    pub fn get_next(&mut self) -> Option<&str> {
        self.position += 1;
        self.vec.get(self.position - 1).map(|v| v.borrow())
    }
}