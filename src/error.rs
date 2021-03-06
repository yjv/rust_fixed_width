use std::fmt::{Display, Formatter, Error as FmtError};
use std::io::Error as IoError;

#[derive(Debug)]
pub enum Error {
    SpecStreamReturnedNone,
    SpecStreamError(BoxedError),
    RecordSpecNotFound(String),
    ParserFailure(BoxedError),
    FormatterFailure(BoxedError),
    IoError(IoError),
    DataDoesNotMatchLineEnding(Vec<u8>, Vec<u8>),
    CouldNotReadEnough(Vec<u8>),
    FormattedValueWrongLength(usize, Vec<u8>),
    FieldValueRequired,
    DataHolderError(BoxedError),
    FieldRequiredToBuild(&'static str)
}

impl ::std::error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::SpecStreamReturnedNone => "record spec stream returned no record spec",
            Error::SpecStreamError(_) => "record spec stream encountered an error",
            Error::RecordSpecNotFound(_) => "record spec could not be found",
            Error::ParserFailure(_) => "The field parser encountered an error",
            Error::FormatterFailure(_) => "The field formatter encountered an error",
            Error::IoError(_) => "An IO error occurred while trying to read",
            Error::CouldNotReadEnough(_) => "Could not read enough data",
            Error::DataDoesNotMatchLineEnding(_, _) => "The encountered line ending doesn't match the expected one",
            Error::FormattedValueWrongLength(_, _) => "The value returned after padding is either longer or shorter than the length for the field",
            Error::FieldValueRequired => "The value for the given field is required since it has no default",
            Error::DataHolderError(_) => "There was an error creating the records data holder",
            Error::FieldRequiredToBuild(_) => "There is a required field missing",
        }
    }

    fn cause(&self) -> Option<&::std::error::Error> {
        match *self {
            Error::SpecStreamError(ref e) => Some(&**e),
            Error::IoError(ref e) => Some(e),
            Error::DataHolderError(ref e) => Some(&**e),
            _ => None
        }
    }
}

macro_rules! write_with_data {
    ($f:expr, $m:expr, $($d:expr)*) => {
        write!($f, $m)?;
        $(match ::std::str::from_utf8($d) {
            Ok(v) => write!($f, "{}", v),
            Err(_) => write!("{:?}", $d)
        };)*
        Ok(())
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> ::std::result::Result<(), FmtError> {
        match *self {
            Error::SpecStreamReturnedNone => write!(f, "record spec stream returned no record spec"),
            Error::SpecStreamError(ref e) => write!(f, "record spec stream encountered an error: {}", e),
            Error::RecordSpecNotFound(ref name) => write!(f, "record spec named {} could not be found", name),
            Error::ParserFailure(ref e) => write!(f, "The field parser encountered an error: {}", e),
            Error::FormatterFailure(ref e) => write!(f, "The field formatter encountered an error: {}", e),
            Error::IoError(ref e) => write!(f, "An IO error occurred while trying to read: {}", e),
            Error::CouldNotReadEnough(ref data) => write!(
                f,
                "Could not read enough data. only got: {}",
                DataDisplayer(data)
            ),
            Error::DataDoesNotMatchLineEnding(ref expected, ref actual) => write!(
                f,
                "The encountered line ending \"{}\" doesn't match the expected one \"{}\"",
                DataDisplayer(actual),
                DataDisplayer(expected)
            ),
            Error::FormattedValueWrongLength(ref expected_length, ref actual_value) => write!(
                f,
                "The value {} returned after padding is {} long and is required to be {} long for the given field",
                DataDisplayer(actual_value),
                actual_value.len(),
                expected_length
            ),
            Error::FieldValueRequired => write!(f, "The value for the field is required since it has no default"),
            Error::DataHolderError(ref e) => write!(f, "An error occurred while trying to create the record data holder: {}", e),
            Error::FieldRequiredToBuild(ref field) => write!(f, "{} must be set in order to build", field),
        }
    }
}

struct DataDisplayer<'a>(&'a Vec<u8>);

impl<'a> Display for DataDisplayer<'a> {
    fn fmt(&self, f: &mut Formatter) -> ::std::result::Result<(), FmtError> {
        match ::std::str::from_utf8(self.0) {
            Ok(v) => write!(f, "{}", v),
            Err(_) => write!(f, "{:?}", self.0)
        }
    }
}

impl From<IoError> for Error {
    fn from(e: IoError) -> Self {
        Error::IoError(e)
    }
}

pub type BoxedError = Box<::std::error::Error + Send + Sync>;

impl From<PositionalError> for Error {
    fn from(error: PositionalError) -> Self {
        error.error
    }
}

#[derive(Debug)]
pub struct PositionalError {
    pub error: Error,
    pub position: Option<Position>
}

impl PositionalError {
    pub fn new(error: Error, position: Position) -> Self {
        PositionalError {
            error: error,
            position: Some(position)
        }
    }
}

impl From<Error> for PositionalError {
    fn from(error: Error) -> Self {
        PositionalError {
            error: error,
            position: None
        }
    }
}

impl<'a> From<(FieldError, &'a str)> for PositionalError {
    fn from(data: (FieldError, &'a str)) -> Self {
        PositionalError {
            error: data.0.error,
            position: Some(if let Some(field) = data.0.field {
                Position::new(data.1.to_string(), field)
            } else {
                Position::new_from_record(data.1.to_string())
            })
        }
    }
}

impl ::std::error::Error for PositionalError {
    fn description(&self) -> &str {
        self.error.description()
    }

    fn cause(&self) -> Option<&::std::error::Error> {
        self.error.cause()
    }
}

impl Display for PositionalError {
    fn fmt(&self, f: &mut Formatter) -> ::std::result::Result<(), FmtError> {
        match self.position {
            None => self.error.fmt(f),
            Some(Position { ref record, field: None }) => write!(f, "{} at record {}", self.error, record),
            Some(Position { ref record, field: Some(ref field) }) => write!(f, "{} at field {} of record {}", self.error, field, record)
        }
    }
}

#[derive(Debug)]
pub struct FieldError {
    pub error: Error,
    pub field: Option<String>
}

impl FieldError {
    pub fn new(error: Error, field: String) -> Self {
        FieldError {
            error: error,
            field: Some(field)
        }
    }
}

impl From<IoError> for FieldError {
    fn from(error: IoError) -> Self {
        FieldError::from(Error::from(error))
    }
}

impl From<Error> for FieldError {
    fn from(error: Error) -> Self {
        FieldError {
            error: error,
            field: None
        }
    }
}

impl<'a> From<(Error, &'a String)> for FieldError {
    fn from(data: (Error, &'a String)) -> Self {
        FieldError {
            error: data.0,
            field: Some(data.1.to_string())
        }
    }
}

impl<'a> From<(Error, &'a str)> for FieldError {
    fn from(data: (Error, &'a str)) -> Self {
        FieldError {
            error: data.0,
            field: Some(data.1.to_string())
        }
    }
}

impl ::std::error::Error for FieldError {
    fn description(&self) -> &str {
        self.error.description()
    }

    fn cause(&self) -> Option<&::std::error::Error> {
        self.error.cause()
    }
}

impl Display for FieldError {
    fn fmt(&self, f: &mut Formatter) -> ::std::result::Result<(), FmtError> {
        match self.field {
            None => self.error.fmt(f),
            Some(ref field) => write!(f, "{} at field {}", self.error, field)
        }
    }
}

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
