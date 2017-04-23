use spec::RecordSpec;
use std::collections::HashMap;
use record::{Data, DataRanges, WriteType};
use super::super::BoxedErrorResult as Result;

pub trait Stream<T: WriteType> {
    fn next<'a, 'b, U: DataRanges + 'a>(&mut self, data: &'a Data<U, T::DataHolder>, record_specs: &'b HashMap<String, RecordSpec>, write_type: &'a T) -> Result<Option<&'b str>>;
}

impl<'c, T: Stream<U> + 'c, U: WriteType + 'c> Stream<U> for &'c mut T {
    fn next<'a, 'b, V: DataRanges + 'a>(&mut self, data: &'a Data<V, U::DataHolder>, record_specs: &'b HashMap<String, RecordSpec>, write_type: &'a U) -> Result<Option<&'b str>> {
        Stream::next(*self, data, record_specs, write_type)
    }
}

pub trait Resolver<T: WriteType> {
    fn resolve<'a, 'b, U: DataRanges + 'a>(&self, data: &'a Data<U, T::DataHolder>, record_specs: &'b HashMap<String, RecordSpec>, write_type: &'a T) -> Result<Option<&'b str>>;
}

impl<'c, T: Resolver<U> + 'c, U: WriteType + 'c> Resolver<U> for &'c mut T {
    fn resolve<'a, 'b, V: DataRanges + 'a>(&self, data: &'a Data<V, U::DataHolder>, record_specs: &'b HashMap<String, RecordSpec>, write_type: &'a U) -> Result<Option<&'b str>> {
        Resolver::resolve(*self, data, record_specs, write_type)
    }
}

pub struct ResolverSource<T: Resolver<U>, U: WriteType> {
    resolver: T,
    read_type: ::std::marker::PhantomData<U>
}

impl <T, U> ResolverSource<T, U>
    where T: Resolver<U>,
          U: WriteType {
    pub fn new(resolver: T) -> Self {
        ResolverSource {
            resolver: resolver,
            read_type: ::std::marker::PhantomData
        }
    }
}

impl <T, U> Stream<U> for ResolverSource<T, U>
    where T: Resolver<U>,
          U: WriteType {
    fn next<'a, 'b, V: DataRanges + 'a>(&mut self, data: &'a Data<V, U::DataHolder>, record_specs: &'b HashMap<String, RecordSpec>, write_type: &'a U) -> Result<Option<&'b str>> {
        self.resolver.resolve(data, record_specs, write_type)
    }
}
