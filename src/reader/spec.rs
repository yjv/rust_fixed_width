use spec::RecordSpec;
use std::collections::{HashMap};
use std::io::BufRead;
use data_type::FieldReadSupport;
use super::super::BoxedErrorResult as Result;
use spec::resolver::{IdFieldResolver};
use spec::stream::{VecStream};
use std::borrow::Borrow;

pub trait RequiresBufRead<T: FieldReadSupport> {
    fn get_suggested_buffer_size<'a>(&self, _: &'a HashMap<String, RecordSpec>, _: &'a T) -> Option<usize> {
        None
    }
}

pub trait Stream<T: FieldReadSupport>: RequiresBufRead<T> {
    fn next<'a, 'b, U: BufRead + 'a>(&mut self, reader: &'a mut U, record_specs: &'b HashMap<String, RecordSpec>, read_support: &'a T) -> Result<Option<&'b str>>;
}

impl<'b, T: RequiresBufRead<U> + 'b, U: FieldReadSupport> RequiresBufRead<U> for &'b T {
    fn get_suggested_buffer_size<'a>(&self, record_specs: &'a HashMap<String, RecordSpec>, read_support: &'a U) -> Option<usize> {
        RequiresBufRead::get_suggested_buffer_size(*self, record_specs, read_support)
    }
}

impl<'b, T: RequiresBufRead<U> + 'b, U: FieldReadSupport> RequiresBufRead<U> for &'b mut T {
    fn get_suggested_buffer_size<'a>(&self, record_specs: &'a HashMap<String, RecordSpec>, read_support: &'a U) -> Option<usize> {
        RequiresBufRead::get_suggested_buffer_size(*self, record_specs, read_support)
    }
}

impl<'c, T: Stream<U> + 'c, U: FieldReadSupport> Stream<U> for &'c mut T {
    fn next<'a, 'b, V: BufRead + 'a>(&mut self, reader: &'a mut V, record_specs: &'b HashMap<String, RecordSpec>, read_support: &'a U) -> Result<Option<&'b str>> {
        Stream::next(*self, reader, record_specs, read_support)
    }
}

pub trait Resolver<T: FieldReadSupport>: RequiresBufRead<T> {
    fn resolve<'a, 'b, U: BufRead + 'a>(&self, reader: &'a mut U, record_specs: &'b HashMap<String, RecordSpec>, read_support: &'a T) -> Result<Option<&'b str>>;
}

impl<'c, T: Resolver<U> + 'c, U: FieldReadSupport> Resolver<U> for &'c mut T {
    fn resolve<'a, 'b, V: BufRead + 'a>(&self, reader: &'a mut V, record_specs: &'b HashMap<String, RecordSpec>, read_support: &'a U) -> Result<Option<&'b str>> {
        Resolver::resolve(*self, reader, record_specs, read_support)
    }
}

pub struct ResolverSource<'a, T: Resolver<U> + 'a, U: FieldReadSupport> {
    resolver: T,
    read_support: ::std::marker::PhantomData<U>,
    lifetime: ::std::marker::PhantomData<&'a ()>
}

impl <'a, T, U> RequiresBufRead<U> for ResolverSource<'a, T, U>
    where T: Resolver<U> + 'a,
          U: FieldReadSupport {
    fn get_suggested_buffer_size<'b>(&self, record_specs: &'b HashMap<String, RecordSpec>, read_support: &'b U) -> Option<usize> {
        self.resolver.get_suggested_buffer_size(record_specs, read_support)
    }
}

impl <'a, T, U> ResolverSource<'a, T, U>
    where T: Resolver<U> + 'a,
          U: FieldReadSupport {
    pub fn new(resolver: T) -> Self {
        ResolverSource {
            resolver: resolver,
            read_support: ::std::marker::PhantomData,
            lifetime: ::std::marker::PhantomData
        }
    }
}

impl<'a, T, U> From<T> for ResolverSource<'a, T, U>
    where T: Resolver<U> + 'a,
          U: FieldReadSupport {
    fn from(resolver: T) -> Self {
        ResolverSource::new(resolver)
    }
}

impl <'a, T, U> Stream<U> for ResolverSource<'a, T, U>
    where T: Resolver<U> + 'a,
          U: FieldReadSupport {
    fn next<'b, 'c, X: BufRead + 'b>(&mut self, reader: &'b mut X, record_specs: &'c HashMap<String, RecordSpec>, read_support: &'b U) -> Result<Option<&'c str>> {
        self.resolver.resolve(reader, record_specs, read_support)
    }
}

impl<T: FieldReadSupport, U: Borrow<str>> RequiresBufRead<T> for IdFieldResolver<U> {
    fn get_suggested_buffer_size<'a>(&self, record_specs: &'a HashMap<String, RecordSpec>, read_support: &'a T) -> Option<usize> {
        let min = record_specs.iter().map(|(_, spec)| spec.field_range(self.id_field()).map(|range| range.end).unwrap_or(0)).min().unwrap_or(0);
        if min == 0 {
            None
        } else {
            read_support.get_size_hint(min).1
        }
    }
}

impl<T: FieldReadSupport, U: Borrow<str>> Resolver<T> for IdFieldResolver<U> {
    fn resolve<'a, 'b, V: BufRead + 'a>(&self, buffer: &'a mut V, record_specs: &'b HashMap<String, RecordSpec>, read_support: &'a T) -> Result<Option<&'b str>> {
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

impl<T: FieldReadSupport> Resolver<T> for () {
    fn resolve<'a, 'b, U: BufRead + 'a>(&self, _: &'a mut U, _: &'b HashMap<String, RecordSpec>, _: &'a T) -> Result<Option<&'b str>> {
        Ok(None)
    }
}

impl<T: FieldReadSupport> Stream<T> for () {
    fn next<'a, 'b, U: BufRead + 'a>(&mut self, _: &'a mut U, _: &'b HashMap<String, RecordSpec>, _: &'a T) -> Result<Option<&'b str>> {
        Ok(None)
    }
}

impl<T: FieldReadSupport> RequiresBufRead<T> for () {}

impl<T: FieldReadSupport, U: Borrow<str>> RequiresBufRead<T> for VecStream<U> {}

impl<T: FieldReadSupport, U: Borrow<str>> Stream<T> for VecStream<U> {
    fn next<'a, 'b, V: BufRead + 'a>(&mut self, _: &'a mut V, record_specs: &'b HashMap<String, RecordSpec>, _: &'a T) -> Result<Option<&'b str>> {
        self.position += self.position;

        Ok(match self.vec.get(self.position) {
            None => None,
            Some(v) => {
                for (name, _) in record_specs.iter() {
                    if name == v.borrow() {
                        return Ok(Some(name));
                    }
                }
                return Err("The nes".into())
            }
        })
    }
}