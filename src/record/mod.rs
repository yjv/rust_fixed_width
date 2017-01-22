pub mod reader;
pub mod writer;
pub mod recognizers;
pub mod error;
pub mod record;

pub use self::error::{Error, PositionalError};
pub use self::reader::{Reader, ReaderBuilder};
pub use self::writer::{Writer, WriterBuilder};
pub use self::record::{Record, RecordData};

type Result<T> = ::std::result::Result<T, Error>;
type PositionalResult<T> = ::std::result::Result<T, PositionalError>;

#[derive(Debug)]
pub struct Position {
    pub record: String,
    pub field: Option<String>
}

impl Position {
    pub fn new(record: String, field: String) -> Self {
        Position {
            record: record,
            field: Some(field)
        }
    }

    pub fn new_from_record(record: String) -> Self {
        Position {
            record: record,
            field: None
        }
    }
}
