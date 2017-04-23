use reader::spec::{Resolver as ReaderResolver, RequiresBufRead};
use writer::spec::Resolver as WriterResolver;
use record::{ReadType, WriteType, DataRanges, Data};
use std::collections::HashMap;
use super::RecordSpec;
use std::io::BufRead;
use super::super::BoxedErrorResult as Result;

pub struct IdFieldResolver {
    id_field: String
}

impl IdFieldResolver {
    pub fn new() -> Self {
        Self::new_with_field("$id")
    }

    pub fn new_with_field<U: Into<String>>(id_field: U) -> Self {
        IdFieldResolver { id_field: id_field.into() }
    }
}

impl<T: ReadType> RequiresBufRead<T> for IdFieldResolver {
    fn get_suggested_buffer_size<'a>(&self, record_specs: &'a HashMap<String, RecordSpec>, read_type: &'a T) -> Option<usize> {
        let min = record_specs.iter().map(|(_, spec)| spec.field_range(&self.id_field).map(|range| range.end).unwrap_or(0)).min().unwrap_or(0);
        if min == 0 {
            None
        } else {
            read_type.get_size_hint(min).1
        }
    }
}

impl<T: ReadType> ReaderResolver<T> for IdFieldResolver {
    fn resolve<'a, 'b, U: BufRead + 'a>(&self, mut buffer: &'a mut U, record_specs: &'b HashMap<String, RecordSpec>, read_type: &'a T) -> Result<Option<&'b str>> {
        for (name, record_spec) in record_specs.iter() {
            if let Some(ref field_spec) = record_spec.field_specs.get(&self.id_field) {
                if let Some(ref default) = field_spec.default {
                    if let Some(field_range) = read_type.get_byte_range(
                        buffer.fill_buf()?,
                        record_spec.field_range(&self.id_field).expect("This should never be None")
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

impl<T: WriteType> WriterResolver<T> for IdFieldResolver {
    fn resolve<'a, 'b, U: DataRanges + 'a>(&self, data: &'a Data<U, T::DataHolder>, record_specs: &'b HashMap<String, RecordSpec>, write_type: &'a T) -> Result<Option<&'b str>> {
        for (name, record_spec) in record_specs.iter() {
            if let Some(ref field_spec) = record_spec.field_specs.get(&self.id_field) {
                if let Some(ref default) = field_spec.default {
                    if let Some(data) = write_type.get_data_by_name(&self.id_field, data) {
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

pub struct NoneResolver;

impl<T: ReadType> ReaderResolver<T> for NoneResolver {
    fn resolve<'a, 'b, U: BufRead + 'a>(&self, _: &'a mut U, _: &'b HashMap<String, RecordSpec>, _: &'a T) -> Result<Option<&'b str>> {
        Ok(None)
    }
}

impl<T: ReadType> RequiresBufRead<T> for NoneResolver {
}

impl<T: WriteType> WriterResolver<T> for NoneResolver {
    fn resolve<'a, 'b, U: DataRanges + 'a>(&self, _: &'a Data<U, T::DataHolder>, _: &'b HashMap<String, RecordSpec>, _: &'a T) -> Result<Option<&'b str>> {
        Ok(None)
    }
}
