use spec::RecordSpec;
use std::collections::{HashMap};
use std::io::BufRead;
use record::ReadType;
use super::super::BoxedErrorResult as Result;

pub trait RequiresBufRead<T: ReadType> {
    fn get_suggested_buffer_size<'a>(&self, _: &'a HashMap<String, RecordSpec>, _: &'a T) -> Option<usize> {
        None
    }
}

pub trait Stream<T: ReadType>: RequiresBufRead<T> {
    fn next<'a, 'b, U: BufRead + 'a>(&mut self, reader: &'a mut U, record_specs: &'b HashMap<String, RecordSpec>, read_type: &'a T) -> Result<Option<&'b str>>;
}

impl<'b, T: RequiresBufRead<U> + 'b, U: ReadType + 'b> RequiresBufRead<U> for &'b mut T {
    fn get_suggested_buffer_size<'a>(&self, record_specs: &'a HashMap<String, RecordSpec>, read_type: &'a U) -> Option<usize> {
        RequiresBufRead::get_suggested_buffer_size(*self, record_specs, read_type)
    }
}

impl<'c, T: Stream<U> + 'c, U: ReadType + 'c> Stream<U> for &'c mut T {
    fn next<'a, 'b, V: BufRead + 'a>(&mut self, reader: &'a mut V, record_specs: &'b HashMap<String, RecordSpec>, read_type: &'a U) -> Result<Option<&'b str>> {
        Stream::next(*self, reader, record_specs, read_type)
    }
}

pub trait Resolver<T: ReadType>: RequiresBufRead<T> {
    fn resolve<'a, 'b, U: BufRead + 'a>(&self, reader: &'a mut U, record_specs: &'b HashMap<String, RecordSpec>, read_type: &'a T) -> Result<Option<&'b str>>;
}

impl<'c, T: Resolver<U> + 'c, U: ReadType + 'c> Resolver<U> for &'c mut T {
    fn resolve<'a, 'b, V: BufRead + 'a>(&self, reader: &'a mut V, record_specs: &'b HashMap<String, RecordSpec>, read_type: &'a U) -> Result<Option<&'b str>> {
        Resolver::resolve(*self, reader, record_specs, read_type)
    }
}

pub struct ResolverSource<T: Resolver<U>, U: ReadType> {
    resolver: T,
    read_type: ::std::marker::PhantomData<U>
}

impl <T, U> RequiresBufRead<U> for ResolverSource<T, U>
    where T: Resolver<U>,
          U: ReadType {
    fn get_suggested_buffer_size<'a>(&self, record_specs: &'a HashMap<String, RecordSpec>, read_type: &'a U) -> Option<usize> {
        self.resolver.get_suggested_buffer_size(record_specs, read_type)
    }
}

impl <T, U> ResolverSource<T, U>
    where T: Resolver<U>,
          U: ReadType {
    pub fn new(resolver: T) -> Self {
        ResolverSource {
            resolver: resolver,
            read_type: ::std::marker::PhantomData
        }
    }
}

impl <T, U> Stream<U> for ResolverSource<T, U>
    where T: Resolver<U>,
          U: ReadType {
    fn next<'a, 'b, X: BufRead + 'a>(&mut self, reader: &'a mut X, record_specs: &'b HashMap<String, RecordSpec>, read_type: &'a U) -> Result<Option<&'b str>> {
        self.resolver.resolve(reader, record_specs, read_type)
    }
}
