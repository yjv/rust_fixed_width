extern crate pad;
use self::pad::{PadStr, Alignment};
use spec::PaddingDirection;
use std::fmt::{Display, Formatter, Error as FmtError};

#[derive(Debug)]
pub struct Error {
    repr: Box<::std::error::Error + Send + Sync>
}

impl Clone for Error {
    fn clone(&self) -> Self {
        Error::new("")
    }
}

impl Error {
    pub fn new<E>(error: E) -> Self
        where E: Into<Box<::std::error::Error + Send + Sync>>
    {
        Error { repr: error.into() }
    }

    pub fn downcast<E: ::std::error::Error + Send + Sync + 'static>(self) -> ::std::result::Result<E, Self> {
        Ok(*(self.repr.downcast::<E>().map_err(|e| Error { repr: e })?))
    }

    pub fn downcast_ref<E: ::std::error::Error + Send + Sync + 'static>(&self) -> Option<&E> {
        self.repr.downcast_ref::<E>()
    }
}

impl ::std::error::Error for Error {
    fn description(&self) -> &str {
        self.repr.description()
    }

    fn cause(&self) -> Option<&::std::error::Error> {
        self.repr.cause()
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> ::std::result::Result<(), FmtError> {
        Display::fmt(&*self.repr, f)
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

#[derive(Debug)]
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
                "the padding string was {} chars long it can only be at most 1 char long",
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
        assert_result!(
            Err(PaddingError::PaddingLongerThanOne(2)),
            padder.pad(data.clone(), 10, &"33".to_string(), PaddingDirection::Left).map_err(|e| e.downcast::<PaddingError>().unwrap())
        );
        let data = "qwer333333".to_string();
        assert_result!(Ok("qwer".to_string()), padder.unpad(data.clone(), &"3".to_string(), PaddingDirection::Right));
        let data = "333333qwer".to_string();
        assert_result!(Ok("qwer".to_string()), padder.unpad(data.clone(), &"3".to_string(), PaddingDirection::Left));
        assert_result!(
            Err(PaddingError::PaddingLongerThanOne(2)),
            padder.unpad(data.clone(), &"33".to_string(), PaddingDirection::Left).map_err(|e| e.downcast::<PaddingError>().unwrap())
        );
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

    #[test]
    fn error() {
        let error = Error::new(PaddingError::PaddingLongerThanOne(23));
        assert_option!(Some(&PaddingError::PaddingLongerThanOne(23)), error.downcast_ref::<PaddingError>());
        assert_option!(Some(&PaddingError::PaddingLongerThanOne(23)), error.downcast_ref::<PaddingError>());
        match error.downcast::<PaddingError>() {
            Ok(PaddingError::PaddingLongerThanOne(23)) => (),
            e => panic!("bad result returned {:?}", e)
        }
    }
}