pub mod reader;
pub mod writer;
pub mod recognizers;
pub mod error;
pub mod record;

pub use self::error::{Error};
pub use self::reader::{Reader, ReaderBuilder};
pub use self::writer::{Writer, WriterBuilder};
pub use self::record::{Record, RecordData};

type Result<T> = ::std::result::Result<T, Error>;

