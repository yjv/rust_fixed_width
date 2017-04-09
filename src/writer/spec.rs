use spec::RecordSpec;
use std::collections::HashMap;
use super::Result;
use record::{Data, DataRanges, WriteType};

pub trait SpecSource<T: WriteType> {
    fn next<'a, 'b, U: DataRanges + 'a>(&mut self, data: &'a Data<U, T::DataHolder>, record_specs: &'b HashMap<String, RecordSpec>, write_type: &'a T) -> Result<&'b str>;
}

impl<'c, T: SpecSource<U> + 'c, U: WriteType + 'c> SpecSource<U> for &'c mut T {
    fn next<'a, 'b, V: DataRanges + 'a>(&mut self, data: &'a Data<V, U::DataHolder>, record_specs: &'b HashMap<String, RecordSpec>, write_type: &'a U) -> Result<&'b str> {
        SpecSource::next(*self, data, record_specs, write_type)
    }
}

pub trait SpecResolver<T: WriteType> {
    fn resolve<'a, 'b, U: DataRanges + 'a>(&self, data: &'a Data<U, T::DataHolder>, record_specs: &'b HashMap<String, RecordSpec>, write_type: &'a T) -> Result<&'b str>;
}

impl<'c, T: SpecResolver<U> + 'c, U: WriteType + 'c> SpecResolver<U> for &'c mut T {
    fn resolve<'a, 'b, V: DataRanges + 'a>(&self, data: &'a Data<V, U::DataHolder>, record_specs: &'b HashMap<String, RecordSpec>, write_type: &'a U) -> Result<&'b str> {
        SpecResolver::resolve(*self, data, record_specs, write_type)
    }
}

pub struct ResolverSource<T: SpecResolver<U>, U: WriteType> {
    resolver: T,
    read_type: ::std::marker::PhantomData<U>
}

impl <T, U> ResolverSource<T, U>
    where T: SpecResolver<U>,
          U: WriteType {
    pub fn new(resolver: T) -> Self {
        ResolverSource {
            resolver: resolver,
            read_type: ::std::marker::PhantomData
        }
    }
}

impl <T, U> SpecSource<U> for ResolverSource<T, U>
    where T: SpecResolver<U>,
          U: WriteType {
    fn next<'a, 'b, V: DataRanges + 'a>(&mut self, data: &'a Data<V, U::DataHolder>, record_specs: &'b HashMap<String, RecordSpec>, write_type: &'a U) -> Result<&'b str> {
        self.resolver.resolve(data, record_specs, write_type)
    }
}
