//!This lib is built to make reading/writing fixed width data alot easier. It uses a
//!the idea of a spec to define the kinds of records you'd like to be able to read off
//!of or write to a data stream and allows you to read and/or write them to data collections
//!
//! #Example
//!```
//! use spec::*;
//!
//! let spec = SpecBuilder::new()
//!     .with_record("record1")
//!         .with_field("field1", FieldSpecBuilder::new_string()
//!             .with_length(10)
//!         )
//!         .with_field(FieldSpecBuilder::new().new_number()
//!             .with_length(5)
//!         )
//!         .end()
//!     )
//!     .with_record("record2")
//!         .with_field("filler")
//!             .filler(5)
//!         .end()
//!         .with_field("field1")
//!             .string()
//!             .with_length(10)
//!         .end()
//!         .with_field("field2")
//!             .number()
//!             .with_length(5)
//!         .end()
//!     )
//!;
//!```

#[cfg(test)]
#[macro_use]
pub mod test;
pub mod error;
pub mod reader;
pub mod record;
pub mod spec;
pub mod writer;
pub mod data_type;

pub use self::error::{Error, FieldError, PositionalError, Position, BoxedError};
pub use self::reader::{Reader, ReaderBuilder};
pub use self::writer::{Writer, WriterBuilder};
pub use self::record::{Record, Data};

type Result<T> = ::std::result::Result<T, error::Error>;
type FieldResult<T> = ::std::result::Result<T, error::FieldError>;
type PositionalResult<T> = ::std::result::Result<T, error::PositionalError>;
type BoxedErrorResult<T> = ::std::result::Result<T, BoxedError>;

