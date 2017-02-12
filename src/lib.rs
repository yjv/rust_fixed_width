//!This lib is built to make reading/writing fixed width data alot easier. It uses a
//!the idea of a spec to define the kinds of records youd line to be able to read off
//!of or write a data stream and allows you to read and/or write them to data collections
//!
//! #Example
//!```
//! use record::RecordBuilder;
//! use spec::*;
//!
//! let spec = SpecBuilder::new()
//!     .with_record("record1", RecordSpecBuilder::new()
//!         .with_field("field1", FieldSpecBuilder::new_string()
//!             .with_length(10)
//!         )
//!         .with_field(FieldSpecBuilder::new().new_number()
//!             .with_length(5)
//!         )
//!     )
//!     .with_record("record2", RecordSpecBuilder::new()
//!         .with_field("filler", FieldSpecBuilder::new_filler(5))
//!         .with_field("field1", FieldSpecBuilder::new_string()
//!             .with_length(10)
//!         )
//!         .with_field(FieldSpecBuilder::new().new_number()
//!             .with_length(5)
//!         )
//!     )
//!;
//!```

#[cfg(test)]
#[macro_use]
pub mod test;
pub mod error;
pub mod padder;
pub mod reader;
pub mod recognizer;
pub mod record;
pub mod spec;
//pub mod writer;

pub use self::error::{Error, PositionalError, Position};
pub use self::reader::{Reader, ReaderBuilder};
//pub use self::writer::{Writer, WriterBuilder};
pub use self::record::{Record, RecordRanges};

type Result<T> = ::std::result::Result<T, Error>;
type PositionalResult<T> = ::std::result::Result<T, PositionalError>;
