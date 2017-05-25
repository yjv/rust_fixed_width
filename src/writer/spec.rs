use spec::RecordSpec;
use std::collections::HashMap;
use record::{Data, DataRanges, WriteType};
use super::super::BoxedErrorResult as Result;
use spec::resolver::{IdFieldResolver, NoneResolver};
use std::borrow::Borrow;

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

impl<T: WriteType, U: Borrow<str>> Resolver<T> for IdFieldResolver<U> {
    fn resolve<'a, 'b, V: DataRanges + 'a>(&self, data: &'a Data<V, T::DataHolder>, record_specs: &'b HashMap<String, RecordSpec>, write_type: &'a T) -> Result<Option<&'b str>> {
        for (name, record_spec) in record_specs.iter() {
            if let Some(ref field_spec) = record_spec.field_specs.get(self.id_field()) {
                if let Some(ref default) = field_spec.default {
                    if let Some(data) = write_type.get_data_by_name(&self.id_field(), data) {
                        if data == &default[..] {
                            return Ok(Some(name));
                        }
                    }
                }
            }
        }

        Ok(None)
    }
}

impl<T: WriteType> Resolver<T> for NoneResolver {
    fn resolve<'a, 'b, U: DataRanges + 'a>(&self, _: &'a Data<U, T::DataHolder>, _: &'b HashMap<String, RecordSpec>, _: &'a T) -> Result<Option<&'b str>> {
        Ok(None)
    }
}