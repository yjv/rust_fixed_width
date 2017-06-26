use spec::RecordSpec;
use std::collections::HashMap;
use record::{Data, DataRanges};
use data_type::{WriteSupport};
use super::super::BoxedErrorResult as Result;
use spec::resolver::{IdFieldResolver, NoneResolver};
use std::borrow::Borrow;

pub trait Stream<'a, T: WriteSupport + 'a> {
    fn next<'b, 'c, U: DataRanges + 'b>(&mut self, data: &'b Data<U, T::DataHolder>, record_specs: &'c HashMap<String, RecordSpec>, write_support: &'b T) -> Result<Option<&'c str>>;
}

impl<'c, T: Stream<'c, U> + 'c, U: WriteSupport + 'c> Stream<'c, U> for &'c mut T {
    fn next<'a, 'b, V: DataRanges + 'a>(&mut self, data: &'a Data<V, U::DataHolder>, record_specs: &'b HashMap<String, RecordSpec>, write_support: &'a U) -> Result<Option<&'b str>> {
        Stream::next(*self, data, record_specs, write_support)
    }
}

pub trait Resolver<'a, T: WriteSupport + 'a> {
    fn resolve<'b, 'c, U: DataRanges + 'b>(&self, data: &'b Data<U, T::DataHolder>, record_specs: &'c HashMap<String, RecordSpec>, write_support: &'b T) -> Result<Option<&'c str>>;
}

impl<'c, T: Resolver<'c, U> + 'c, U: WriteSupport + 'c> Resolver<'c, U> for &'c mut T {
    fn resolve<'a, 'b, V: DataRanges + 'a>(&self, data: &'a Data<V, U::DataHolder>, record_specs: &'b HashMap<String, RecordSpec>, write_support: &'a U) -> Result<Option<&'b str>> {
        Resolver::resolve(*self, data, record_specs, write_support)
    }
}

pub struct ResolverSource<'a, T: Resolver<'a, U> + 'a, U: WriteSupport + 'a> {
    resolver: T,
    read_support: ::std::marker::PhantomData<&'a U>
}

impl <'a, T, U> ResolverSource<'a, T, U>
    where T: Resolver<'a, U> + 'a,
          U: WriteSupport + 'a {
    pub fn new(resolver: T) -> Self {
        ResolverSource {
            resolver: resolver,
            read_support: ::std::marker::PhantomData
        }
    }
}

impl <'a, T, U> Stream<'a, U> for ResolverSource<'a, T, U>
    where T: Resolver<'a, U> + 'a,
          U: WriteSupport + 'a {
    fn next<'b, 'c, V: DataRanges + 'b>(&mut self, data: &'b Data<V, U::DataHolder>, record_specs: &'c HashMap<String, RecordSpec>, write_support: &'b U) -> Result<Option<&'c str>> {
        self.resolver.resolve(data, record_specs, write_support)
    }
}

impl<'a, T: WriteSupport + 'a, U: Borrow<str>> Resolver<'a, T> for IdFieldResolver<U> {
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

impl<'a, T: WriteSupport + 'a> Resolver<'a, T> for NoneResolver {
    fn resolve<'b, 'c, U: DataRanges + 'b>(&self, _: &'b Data<U, T::DataHolder>, _: &'c HashMap<String, RecordSpec>, _: &'b T) -> Result<Option<&'c str>> {
        Ok(None)
    }
}