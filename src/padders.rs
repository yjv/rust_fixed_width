extern crate pad;
use self::pad::{PadStr, Alignment};
use spec::PaddingDirection;
use std::fmt::{Display, Formatter, Error as FmtError};

#[derive(Debug)]
pub struct Error(Box<::std::error::Error + Send + Sync>);

impl Clone for Error {
    fn clone(&self) -> Self {
        Error("".into())
    }
}

impl Error {
    pub fn new<E>(error: E) -> Self
        where E: Into<Box<::std::error::Error + Send + Sync>>
    {
        Error(error.into())
    }
}

impl ::std::error::Error for Error {
    fn description(&self) -> &str {
        self.0.description()
    }

    fn cause(&self) -> Option<&::std::error::Error> {
        self.0.cause()
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> ::std::result::Result<(), FmtError> {
        Display::fmt(&*self.0, f)
    }
}

type Result<T> = ::std::result::Result<T, Error>;

pub trait Padder {
    fn pad(&self, data: String, length: usize, padding: &String, direction: PaddingDirection) -> Result<String>;
}

impl<'a, T> Padder for &'a T where T: 'a + Padder {
    fn pad(&self, data: String, length: usize, padding: &String, direction: PaddingDirection) -> Result<String> {
        (**self).pad(data, length, padding, direction)
    }
}

pub trait UnPadder {
    fn unpad(&self, data: String, padding: &String, direction: PaddingDirection) -> Result<String>;
}

impl<'a, T> UnPadder for &'a T where T: 'a + UnPadder {
    fn unpad(&self, data: String, padding: &String, direction: PaddingDirection) -> Result<String> {
        (**self).unpad(data, padding, direction)
    }
}

pub struct DefaultPadder;

#[derive(Debug, PartialEq, Eq)]
pub enum PaddingError {
    PaddingLongerThanOne(usize)
}

impl ::std::error::Error for PaddingError {
    fn description(&self) -> &str {
        match *self {
            PaddingError::PaddingLongerThanOne(_) => "The padding string must be only one char long to use this padder"
        }
    }
}

impl Display for PaddingError {
    fn fmt(&self, f: &mut Formatter) -> ::std::result::Result<(), FmtError> {
        match *self {
            PaddingError::PaddingLongerThanOne(len) => write!(
                f,
                "PaddingLongerThanOne: the padding string was {} chars long it can only be at most 1 char long",
                len
            )
        }
    }
}

impl From<PaddingError> for Error {
    fn from(e: PaddingError) -> Self {
        Error::new(e)
    }
}

impl DefaultPadder {
    fn get_char(padding: &String) -> ::std::result::Result<char, PaddingError> {
        if padding.len() > 1 {
            Err(PaddingError::PaddingLongerThanOne(padding.len()))
        } else {
            Ok(padding.chars().next().or(Some(' ')).expect("should have a some no matter what"))
        }
    }
}

impl Padder for DefaultPadder {
    fn pad(&self, data: String, length: usize, padding: &String, direction: PaddingDirection) -> Result<String> {
        Ok(data.pad(
            length,
            Self::get_char(padding)?,
            match direction {
                PaddingDirection::Left => Alignment::Right,
                PaddingDirection::Right => Alignment::Left,
            },
            false
        ))
    }
}

impl UnPadder for DefaultPadder {
    fn unpad(&self, data: String, padding: &String, direction: PaddingDirection) -> Result<String> {
        Ok(match direction {
            PaddingDirection::Left => data.trim_left_matches(Self::get_char(padding)?).to_string(),
            PaddingDirection::Right => data.trim_right_matches(Self::get_char(padding)?).to_string(),
        })
    }
}

pub struct IdentityPadder;

impl Padder for IdentityPadder {
    fn pad(&self, data: String, _: usize, _: &String, _: PaddingDirection) -> Result<String> {
        Ok(data)
    }
}

impl UnPadder for IdentityPadder {
    fn unpad(&self, data: String, _: &String, _: PaddingDirection) -> Result<String> {
        Ok(data)
    }
}


#[cfg(test)]
mod test {
    use super::*;
    use spec::*;

    #[test]
    fn default_padder() {
        let padder = DefaultPadder;
        let data = "qwer".to_string();
        assert_result!(Ok("qwer333333".to_string()), padder.pad(data.clone(), 10, &"3".to_string(), PaddingDirection::Right));
        let data = "qwer".to_string();
        assert_result!(Ok("333333qwer".to_string()), padder.pad(data.clone(), 10, &"3".to_string(), PaddingDirection::Left));
        assert_result!(Err(Error(ref e)) if match (*e).downcast_ref::<PaddingError>() {
            Some(&PaddingError::PaddingLongerThanOne(2)) => true,
            _ => false
        }, padder.pad(data.clone(), 10, &"33".to_string(), PaddingDirection::Left));
        let data = "qwer333333".to_string();
        assert_result!(Ok("qwer".to_string()), padder.unpad(data.clone(), &"3".to_string(), PaddingDirection::Right));
        let data = "333333qwer".to_string();
        assert_result!(Ok("qwer".to_string()), padder.unpad(data.clone(), &"3".to_string(), PaddingDirection::Left));
        assert_result!(Err(Error(ref e)) if match (*e).downcast_ref::<PaddingError>() {
            Some(&PaddingError::PaddingLongerThanOne(2)) => true,
            _ => false
        }, padder.unpad(data.clone(), &"33".to_string(), PaddingDirection::Left));
    }

    #[test]
    fn identity_padder() {
        let padder = IdentityPadder;
        let data = "qwer".to_string();
        assert_result!(Ok(data.clone()), padder.pad(data.clone(), 10, &"3".to_string(), PaddingDirection::Right));
        assert_result!(Ok(data.clone()), padder.pad(data.clone(), 10, &"3".to_string(), PaddingDirection::Left));
        assert_result!(Ok(data.clone()), padder.unpad(data.clone(), &"3".to_string(), PaddingDirection::Right));
        assert_result!(Ok(data.clone()), padder.unpad(data.clone(), &"3".to_string(), PaddingDirection::Left));
    }

    #[test]
    fn padder_reference() {
        let padder = IdentityPadder;
        let data = "qwer".to_string();
        assert_result!(Ok(data.clone()), Padder::pad(&&padder, data.clone(), 10, &"3".to_string(), PaddingDirection::Right));
        assert_result!(Ok(data.clone()), UnPadder::unpad(&&padder, data.clone(), &"3".to_string(), PaddingDirection::Right));
    }
}