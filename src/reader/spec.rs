use spec::RecordSpec;
use std::collections::{HashMap};
use std::io::BufRead;
use data_type::FieldReadSupport;
use super::super::BoxedErrorResult as Result;
use spec::resolver::{IdFieldResolver, NoneResolver};
use std::borrow::Borrow;

pub trait RequiresBufRead<'a, T: FieldReadSupport + 'a> {
    fn get_suggested_buffer_size<'b>(&self, _: &'b HashMap<String, RecordSpec>, _: &'b T) -> Option<usize> {
        None
    }
}

pub trait Stream<'a, T: FieldReadSupport + 'a>: RequiresBufRead<'a, T> {
    fn next<'b, 'c, U: BufRead + 'b>(&mut self, reader: &'b mut U, record_specs: &'c HashMap<String, RecordSpec>, read_support: &'b T) -> Result<Option<&'c str>>;
}

impl<'b, T: RequiresBufRead<'b, U> + 'b, U: FieldReadSupport + 'b> RequiresBufRead<'b, U> for &'b T {
    fn get_suggested_buffer_size<'a>(&self, record_specs: &'a HashMap<String, RecordSpec>, read_support: &'a U) -> Option<usize> {
        RequiresBufRead::get_suggested_buffer_size(*self, record_specs, read_support)
    }
}

impl<'b, T: RequiresBufRead<'b, U> + 'b, U: FieldReadSupport + 'b> RequiresBufRead<'b, U> for &'b mut T {
    fn get_suggested_buffer_size<'a>(&self, record_specs: &'a HashMap<String, RecordSpec>, read_support: &'a U) -> Option<usize> {
        RequiresBufRead::get_suggested_buffer_size(*self, record_specs, read_support)
    }
}

impl<'c, T: Stream<'c, U> + 'c, U: FieldReadSupport + 'c> Stream<'c, U> for &'c mut T {
    fn next<'a, 'b, V: BufRead + 'a>(&mut self, reader: &'a mut V, record_specs: &'b HashMap<String, RecordSpec>, read_support: &'a U) -> Result<Option<&'b str>> {
        Stream::next(*self, reader, record_specs, read_support)
    }
}

pub trait Resolver<'a, T: FieldReadSupport + 'a>: RequiresBufRead<'a, T> {
    fn resolve<'b, 'c, U: BufRead + 'b>(&self, reader: &'b mut U, record_specs: &'c HashMap<String, RecordSpec>, read_support: &'b T) -> Result<Option<&'c str>>;
}

impl<'c, T: Resolver<'c, U> + 'c, U: FieldReadSupport + 'c> Resolver<'c, U> for &'c mut T {
    fn resolve<'a, 'b, V: BufRead + 'a>(&self, reader: &'a mut V, record_specs: &'b HashMap<String, RecordSpec>, read_support: &'a U) -> Result<Option<&'b str>> {
        Resolver::resolve(*self, reader, record_specs, read_support)
    }
}

pub struct ResolverSource<'a, T: Resolver<'a, U> + 'a, U: FieldReadSupport + 'a> {
    resolver: T,
    read_support: ::std::marker::PhantomData<&'a U>
}

impl <'a, T, U> RequiresBufRead<'a, U> for ResolverSource<'a, T, U>
    where T: Resolver<'a, U> + 'a,
          U: FieldReadSupport + 'a {
    fn get_suggested_buffer_size<'b>(&self, record_specs: &'b HashMap<String, RecordSpec>, read_support: &'b U) -> Option<usize> {
        self.resolver.get_suggested_buffer_size(record_specs, read_support)
    }
}

impl <'a, T, U> ResolverSource<'a, T, U>
    where T: Resolver<'a, U> + 'a,
          U: FieldReadSupport + 'a {
    pub fn new(resolver: T) -> Self {
        ResolverSource {
            resolver: resolver,
            read_support: ::std::marker::PhantomData
        }
    }
}

impl<'a, T, U> From<T> for ResolverSource<'a, T, U>
    where T: Resolver<'a, U> + 'a,
          U: FieldReadSupport + 'a {
    fn from(resolver: T) -> Self {
        ResolverSource::new(resolver)
    }
}

impl <'a, T, U> Stream<'a, U> for ResolverSource<'a, T, U>
    where T: Resolver<'a, U> + 'a,
          U: FieldReadSupport + 'a {
    fn next<'b, 'c, X: BufRead + 'b>(&mut self, reader: &'b mut X, record_specs: &'c HashMap<String, RecordSpec>, read_support: &'b U) -> Result<Option<&'c str>> {
        self.resolver.resolve(reader, record_specs, read_support)
    }
}

impl<'a, T: FieldReadSupport + 'a, U: Borrow<str>> RequiresBufRead<'a, T> for IdFieldResolver<U> {
    fn get_suggested_buffer_size<'b>(&self, record_specs: &'b HashMap<String, RecordSpec>, read_support: &'b T) -> Option<usize> {
        let min = record_specs.iter().map(|(_, spec)| spec.field_range(self.id_field()).map(|range| range.end).unwrap_or(0)).min().unwrap_or(0);
        if min == 0 {
            None
        } else {
            read_support.get_size_hint(min).1
        }
    }
}

impl<'a, T: FieldReadSupport + 'a, U: Borrow<str>> Resolver<'a, T> for IdFieldResolver<U> {
    fn resolve<'b, 'c, V: BufRead + 'b>(&self, buffer: &'b mut V, record_specs: &'c HashMap<String, RecordSpec>, read_support: &'b T) -> Result<Option<&'c str>> {
        for (name, record_spec) in record_specs.iter() {
            if let Some(ref field_spec) = record_spec.field_specs.get(self.id_field()) {
                if let Some(ref default) = field_spec.default {
                    if let Some(field_range) = read_support.get_byte_range(
                        buffer.fill_buf()?,
                        record_spec.field_range(self.id_field()).expect("This should never be None")
                    ) {
                        if buffer.fill_buf()?.len() < field_range.end {
                            continue;
                        }

                        if &buffer.fill_buf()?[field_range] == &default[..] {
                            return Ok(Some(name));
                        }
                    }
                }
            }
        }

        Ok(None)
    }
}

impl<'a, T: FieldReadSupport + 'a> Resolver<'a, T> for NoneResolver {
    fn resolve<'b, 'c, U: BufRead + 'b>(&self, _: &'b mut U, _: &'c HashMap<String, RecordSpec>, _: &'b T) -> Result<Option<&'c str>> {
        Ok(None)
    }
}

impl<'a, T: FieldReadSupport + 'a> RequiresBufRead<'a, T> for NoneResolver {}
