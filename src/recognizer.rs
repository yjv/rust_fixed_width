use std::collections::{HashMap};
use spec::RecordSpec;
use std::io::{Read, BufRead, Error as IoError};
use std::fmt::{Display, Error as FmtError, Formatter};
use std::error::Error as ErrorTrait;
use record::{Data, DataRanges, ReadType, WriteType};
use reader::spec::{Resolver as ReaderResolver, RequiresBufRead};
use writer::spec::Resolver as WriterResolver;

#[derive(Debug)]
pub enum Error {
    CouldNotRecognize,
    Other {
        repr: Box<::std::error::Error + Send + Sync>
    }
}

impl Clone for Error {
    fn clone(&self) -> Self {
        match *self {
            Error::CouldNotRecognize => Error::CouldNotRecognize,
            Error::Other { .. } => Error::new("")
        }
    }
}

impl Error {
    pub fn new<E>(error: E) -> Self
        where E: Into<Box<::std::error::Error + Send + Sync>>
    {
        Error::Other { repr: error.into() }
    }

    pub fn downcast<E: ::std::error::Error + Send + Sync + 'static>(self) -> ::std::result::Result<E, Self> {
        match self {
            Error::CouldNotRecognize => Err(Error::CouldNotRecognize),
            Error::Other { repr } => Ok(*(repr.downcast::<E>().map_err(|e| Error::Other { repr: e })?))
        }
    }

    pub fn downcast_ref<E: ::std::error::Error + Send + Sync + 'static>(&self) -> Option<&E> {
        match *self {
            Error::CouldNotRecognize => None,
            Error::Other { ref repr } => repr.downcast_ref::<E>()
        }
    }
}

impl ErrorTrait for Error {
    fn description(&self) -> &str {
        match *self {
            Error::CouldNotRecognize => "Could not recognize as any specific record spec",
            Error::Other { repr: ref e } => e.description()
        }
    }

    fn cause(&self) -> Option<&::std::error::Error> {
        match *self {
            Error::CouldNotRecognize => None,
            Error::Other { repr: ref e } => e.cause()
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> ::std::result::Result<(), FmtError> {
        match *self {
            Error::CouldNotRecognize => write!(f, "{}", self.description()),
            Error::Other { repr: ref e } => e.fmt(f),
        }
    }
}

impl From<IoError> for Error {
    fn from(e: IoError) -> Self {
        Error::new(e)
    }
}

type Result<T> = ::std::result::Result<T, Error>;

pub struct IdFieldRecognizer {
    id_field: String
}

impl IdFieldRecognizer {
    pub fn new() -> Self {
        Self::new_with_field("$id")
    }

    pub fn new_with_field<U: Into<String>>(id_field: U) -> Self {
        IdFieldRecognizer { id_field: id_field.into() }
    }
}

impl<T: ReadType> RequiresBufRead<T> for IdFieldRecognizer {
    fn get_suggested_buffer_size<'a>(&self, record_specs: &'a HashMap<String, RecordSpec>, read_type: &'a T) -> Option<usize> {
        let min = record_specs.iter().map(|(_, spec)| spec.field_range(&self.id_field).map(|range| range.end).unwrap_or(0)).min().unwrap_or(0);
        if min == 0 {
            None
        } else {
            read_type.get_size_hint(min).1
        }
    }
}

impl<T: ReadType> ReaderResolver<T> for IdFieldRecognizer {
    fn resolve<'a, 'b, U: BufRead + 'a>(&self, mut buffer: &'a mut U, record_specs: &'b HashMap<String, RecordSpec>, read_type: &'a T) -> Result<&'b str> {
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
                            return Ok(name);
                        }
                    }
                }
            }
        }

        Err(Error::CouldNotRecognize)
    }
}

impl<T: WriteType> WriterResolver<T> for IdFieldRecognizer {
    fn resolve<'a, 'b, U: DataRanges + 'a>(&self, data: &'a Data<U, T::DataHolder>, record_specs: &'b HashMap<String, RecordSpec>, write_type: &'a T) -> Result<&'b str> {
        for (name, record_spec) in record_specs.iter() {
            if let Some(ref field_spec) = record_spec.field_specs.get(&self.id_field) {
                if let Some(ref default) = field_spec.default {
                    if let Some(data) = write_type.get_data_by_name(&self.id_field, data) {
                        if data == &default[..] {
                            return Ok(name);
                        }
                    }
                }
            }
        }

        Err(Error::CouldNotRecognize)
    }
}

pub struct NoneRecognizer;

impl<T: ReadType> ReaderResolver<T> for NoneRecognizer {
    fn resolve<'a, 'b, U: Read + 'a>(&self, _: &'a mut U, _: &'b HashMap<String, RecordSpec>, _: &'a T) -> Result<&'b str> {
        Err(Error::CouldNotRecognize)
    }
}

impl<T: ReadType> RequiresBufRead<T> for NoneRecognizer {
}

impl<T: WriteType> WriterResolver<T> for NoneRecognizer {
    fn resolve<'a, 'b, U: DataRanges + 'a>(&self, _: &'a Data<U, T::DataHolder>, _: &'b HashMap<String, RecordSpec>, _: &'a T) -> Result<&'b str> {
        Err(Error::CouldNotRecognize)
    }
}
