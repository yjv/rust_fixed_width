use std::error::Error as ErrorTrait;
use std::fmt::{Display, Formatter, Error as FmtError};
use padders::Error as PadderError;
use std::io::Error as IoError;
use super::recognizers::Error as RecognizerError;
use super::Position;

#[derive(Debug)]
pub enum Error {
    RecordSpecNameRequired,
    RecordSpecRecognizerError(RecognizerError),
    RecordSpecNotFound(String),
    FieldSpecNotFound(String, String),
    PadderFailure(PadderError),
    IoError(IoError),
    StringDoesNotMatchLineEnding(String, String),
    PaddedValueWrongLength(usize, String),
    FieldValueRequired
}

impl ErrorTrait for Error {
    fn description(&self) -> &str {
        match *self {
            Error::RecordSpecNameRequired => "spec name is required since it cannot be recognized",
            Error::RecordSpecRecognizerError(_) => "record spec recognizer encountered an error",
            Error::RecordSpecNotFound(_) => "record spec could not be found",
            Error::FieldSpecNotFound(_, _) => "field spec could not be found",
            Error::PadderFailure(_) => "The un-padder encountered an error",
            Error::IoError(_) => "An IO error occurred while trying to read",
            Error::StringDoesNotMatchLineEnding(_, _) => "The encountered line ending doesn't match the expected one",
            Error::PaddedValueWrongLength(_, _) => "The value returned after padding is either longer or shorter than the length for the field",
            Error::FieldValueRequired => "The value for the given field is required since it has no default"
        }
    }

    fn cause(&self) -> Option<&ErrorTrait> {
        match *self {
            Error::RecordSpecRecognizerError(ref e) => Some(e),
            Error::PadderFailure(ref e) => Some(e),
            Error::IoError(ref e) => Some(e),
            _ => None
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> ::std::result::Result<(), FmtError> {
        match *self {
            Error::RecordSpecNameRequired => write!(f, "spec name is required since it cannot be recognized"),
            Error::RecordSpecRecognizerError(ref e) => write!(f, "record spec recognizer encountered an error: {}", e),
            Error::RecordSpecNotFound(ref name) => write!(f, "record spec named {} could not be found", name),
            Error::FieldSpecNotFound(ref record_name, ref name) => write!(f, "field spec named {} in record spec {} could not be found", name, record_name),
            Error::PadderFailure(ref e) => write!(f, "The un-padder encountered an error: {}", e),
            Error::IoError(ref e) => write!(f, "An IO error occurred while trying to read: {}", e),
            Error::StringDoesNotMatchLineEnding(ref expected, ref actual) => write!(f, "The encountered line ending \"{}\" doesn't match the expected one \"{}\"", actual, expected),
            Error::PaddedValueWrongLength(ref expected, ref actual) => write!(f, "The value {} returned after padding is {} long and is required to be {} long for the given field", actual, actual.len(), expected),
            Error::FieldValueRequired => write!(f, "The value for the field is required since it has no default")
        }
    }
}

impl From<IoError> for Error {
    fn from(e: IoError) -> Self {
        Error::IoError(e)
    }
}

impl From<RecognizerError> for Error {
    fn from(e: RecognizerError) -> Self {
        match e {
            RecognizerError::CouldNotRecognize => Error::RecordSpecNameRequired,
            _ => Error::RecordSpecRecognizerError(e)
        }
    }
}

impl From<PadderError> for Error {
    fn from(e: PadderError) -> Self {
        Error::PadderFailure(e)
    }
}

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

impl From<RecognizerError> for PositionalError {
    fn from(error: RecognizerError) -> Self {
        PositionalError::from(Error::from(error))
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

impl From<(Error, String)> for PositionalError {
    fn from(data: (Error, String)) -> Self {
        PositionalError {
            error: data.0,
            position: Some(Position::new_from_record(data.1))
        }
    }
}

impl From<(Error, String, String)> for PositionalError {
    fn from(data: (Error, String, String)) -> Self {
        PositionalError {
            error: data.0,
            position: Some(Position::new(data.1, data.2))
        }
    }
}

impl ErrorTrait for PositionalError {
    fn description(&self) -> &str {
        self.error.description()
    }

    fn cause(&self) -> Option<&ErrorTrait> {
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
