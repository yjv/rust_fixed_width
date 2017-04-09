use std::collections::VecDeque;

pub trait Source {
    fn get(&mut self) -> Option<Vec<u8>>;
}

impl<'a, T: Source + 'a> Source for &'a mut T {
    fn get(&mut self) -> Option<Vec<u8>> {
        Source::get(*self)
    }
}

impl Source for Vec<u8> {
    fn get(&mut self) -> Option<Vec<u8>> {
        Some(self.clone())
    }
}

impl Source for Option<Vec<u8>> {
    fn get(&mut self) -> Option<Vec<u8>> {
        self.take()
    }
}

impl Source for Vec<Vec<u8>> {
    fn get(&mut self) -> Option<Vec<u8>> {
        self.pop()
    }
}

impl Source for VecDeque<Vec<u8>> {
    fn get(&mut self) -> Option<Vec<u8>> {
        self.pop_front()
    }
}