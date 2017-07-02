use spec::RecordSpec;
use std::collections::HashMap;
use record::{Data, DataRanges};
use data_type::{WriteSupport};
use super::super::BoxedErrorResult as Result;
use spec::resolver::IdFieldResolver;
use spec::stream::VecStream;
use std::borrow::Borrow;

pub trait Stream<T: WriteSupport> {
    fn next<'a, 'b, U: DataRanges + 'a>(&mut self, data: &'a Data<U, T::DataHolder>, record_specs: &'b HashMap<String, RecordSpec>, write_support: &'a T) -> Result<Option<&'b str>>;
}

impl<'c, T: Stream<U> + 'c, U: WriteSupport> Stream<U> for &'c mut T {
    fn next<'a, 'b, V: DataRanges + 'a>(&mut self, data: &'a Data<V, U::DataHolder>, record_specs: &'b HashMap<String, RecordSpec>, write_support: &'a U) -> Result<Option<&'b str>> {
        Stream::next(*self, data, record_specs, write_support)
    }
}

pub trait Resolver<T: WriteSupport> {
    fn resolve<'a, 'b, U: DataRanges + 'a>(&self, data: &'a Data<U, T::DataHolder>, record_specs: &'b HashMap<String, RecordSpec>, write_support: &'a T) -> Result<Option<&'b str>>;
}

impl<'c, T: Resolver<U> + 'c, U: WriteSupport> Resolver<U> for &'c mut T {
    fn resolve<'a, 'b, V: DataRanges + 'a>(&self, data: &'a Data<V, U::DataHolder>, record_specs: &'b HashMap<String, RecordSpec>, write_support: &'a U) -> Result<Option<&'b str>> {
        Resolver::resolve(*self, data, record_specs, write_support)
    }
}

pub struct ResolverSource<'a, T: Resolver<U> + 'a, U: WriteSupport> {
    resolver: T,
    read_support: ::std::marker::PhantomData<U>,
    lifetime: ::std::marker::PhantomData<&'a ()>
}

impl <'a, T, U> ResolverSource<'a, T, U>
    where T: Resolver<U> + 'a,
          U: WriteSupport {
    pub fn new(resolver: T) -> Self {
        ResolverSource {
            resolver: resolver,
            read_support: ::std::marker::PhantomData,
            lifetime: ::std::marker::PhantomData
        }
    }
}

impl <'a, T, U> Stream<U> for ResolverSource<'a, T, U>
    where T: Resolver<U> + 'a,
          U: WriteSupport {
    fn next<'b, 'c, V: DataRanges + 'b>(&mut self, data: &'b Data<V, U::DataHolder>, record_specs: &'c HashMap<String, RecordSpec>, write_support: &'b U) -> Result<Option<&'c str>> {
        self.resolver.resolve(data, record_specs, write_support)
    }
}

impl<'a, T: WriteSupport, U: Borrow<str>> Resolver<T> for IdFieldResolver<U> {
    fn resolve<'b, 'c, V: DataRanges + 'b>(&self, data: &'b Data<V, T::DataHolder>, record_specs: &'c HashMap<String, RecordSpec>, write_support: &'b T) -> Result<Option<&'c str>> {
        for (name, record_spec) in record_specs.iter() {
            if let Some(ref field_spec) = record_spec.field_specs.get(self.id_field()) {
                if let Some(ref default) = field_spec.default {
                    if let Some(data) = write_support.get_data_by_name(&self.id_field(), data) {
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

impl<T: WriteSupport> Resolver<T> for () {
    fn resolve<'a, 'b, U: DataRanges + 'a>(&self, _: &'a Data<U, T::DataHolder>, _: &'b HashMap<String, RecordSpec>, _: &'a T) -> Result<Option<&'b str>> {
        Ok(None)
    }
}

impl<T: WriteSupport> Stream<T> for () {
    fn next<'a, 'b, U: DataRanges + 'a>(&mut self, _: &'a Data<U, T::DataHolder>, _: &'b HashMap<String, RecordSpec>, _: &'a T) -> Result<Option<&'b str>> {
        Ok(None)
    }
}

impl<T: WriteSupport, U: Borrow<str>> Stream<T> for VecStream<U> {
    fn next<'a, 'b, V: DataRanges + 'a>(&mut self, _: &'a Data<V, T::DataHolder>, record_specs: &'b HashMap<String, RecordSpec>, _: &'a T) -> Result<Option<&'b str>> {
        self.next(record_specs)
    }
}