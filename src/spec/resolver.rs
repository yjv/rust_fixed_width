use std::borrow::Borrow;

pub struct IdFieldResolver<T: Borrow<str>> {
    id_field: T
}

impl<T: Borrow<str>> IdFieldResolver<T> {
    pub fn new_with_field(id_field: T) -> Self {
        IdFieldResolver { id_field: id_field }
    }

    pub fn id_field(&self) -> &str {
        &self.id_field.borrow()
    }
}

impl IdFieldResolver<&'static str> {
    pub fn new() -> Self {
        Self::new_with_field("$id")
    }
}

pub struct NoneResolver;


#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn id_field_resolver() {
        assert_eq!("$id", IdFieldResolver::new().id_field);
        assert_eq!("field", IdFieldResolver::new_with_field("field").id_field);
    }
}