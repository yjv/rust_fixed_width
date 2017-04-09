use spec::RecordSpec;
use std::collections::{HashMap};
use std::io::BufRead;
use super::Result;
use record::ReadType;

pub trait RequiresBufRead<T: ReadType> {
    fn get_suggested_buffer_size<'a>(&self, _: &'a HashMap<String, RecordSpec>, _: &'a T) -> Option<usize> {
        None
    }
}

pub trait SpecSource<T: ReadType>: RequiresBufRead<T> {
    fn next<'a, 'b, U: BufRead + 'a>(&mut self, reader: &'a mut U, record_specs: &'b HashMap<String, RecordSpec>, read_type: &'a T) -> Result<&'b str>;
}

impl<'b, T: RequiresBufRead<U> + 'b, U: ReadType + 'b> RequiresBufRead<U> for &'b mut T {
    fn get_suggested_buffer_size<'a>(&self, record_specs: &'a HashMap<String, RecordSpec>, read_type: &'a U) -> Option<usize> {
        RequiresBufRead::get_suggested_buffer_size(*self, record_specs, read_type)
    }
}

impl<'c, T: SpecSource<U> + 'c, U: ReadType + 'c> SpecSource<U> for &'c mut T {
    fn next<'a, 'b, V: BufRead + 'a>(&mut self, reader: &'a mut V, record_specs: &'b HashMap<String, RecordSpec>, read_type: &'a U) -> Result<&'b str> {
        SpecSource::next(*self, reader, record_specs, read_type)
    }
}

pub trait SpecResolver<T: ReadType>: RequiresBufRead<T> {
    fn resolve<'a, 'b, U: BufRead + 'a>(&self, reader: &'a mut U, record_specs: &'b HashMap<String, RecordSpec>, read_type: &'a T) -> Result<&'b str>;
}

impl<'c, T: SpecResolver<U> + 'c, U: ReadType + 'c> SpecResolver<U> for &'c mut T {
    fn resolve<'a, 'b, V: BufRead + 'a>(&self, reader: &'a mut V, record_specs: &'b HashMap<String, RecordSpec>, read_type: &'a U) -> Result<&'b str> {
        SpecResolver::resolve(*self, reader, record_specs, read_type)
    }
}

pub struct ResolverSource<T: SpecResolver<U>, U: ReadType> {
    resolver: T,
    read_type: ::std::marker::PhantomData<U>
}

impl <T, U> RequiresBufRead<U> for ResolverSource<T, U>
    where T: SpecResolver<U>,
          U: ReadType {
    fn get_suggested_buffer_size<'a>(&self, record_specs: &'a HashMap<String, RecordSpec>, read_type: &'a U) -> Option<usize> {
        self.resolver.get_suggested_buffer_size(record_specs, read_type)
    }
}

impl <T, U> ResolverSource<T, U>
    where T: SpecResolver<U>,
          U: ReadType {
    pub fn new(resolver: T) -> Self {
        ResolverSource {
            resolver: resolver,
            read_type: ::std::marker::PhantomData
        }
    }
}

impl <T, U> SpecSource<U> for ResolverSource<T, U>
    where T: SpecResolver<U>,
          U: ReadType {
    fn next<'a, 'b, X: BufRead + 'a>(&mut self, reader: &'a mut X, record_specs: &'b HashMap<String, RecordSpec>, read_type: &'a U) -> Result<&'b str> {
        self.resolver.resolve(reader, record_specs, read_type)
    }
}
