extern crate pad;
use self::pad::{PadStr, Alignment};
use std::fmt::Debug;
use spec::PaddingDirection;

pub trait Padder {
    type Error: Debug;
    fn pad(&self, data: String, length: usize, padding: &String, direction: PaddingDirection) -> Result<String, Self::Error>;
}

impl<'a, T> Padder for &'a T where T: 'a + Padder {
    type Error = T::Error;
    fn pad(&self, data: String, length: usize, padding: &String, direction: PaddingDirection) -> Result<String, Self::Error> {
        (**self).pad(data, length, padding, direction)
    }
}

pub trait UnPadder {
    type Error: Debug;
    fn unpad(&self, data: String, padding: &String, direction: PaddingDirection) -> Result<String, Self::Error>;
}

impl<'a, T> UnPadder for &'a T where T: 'a + UnPadder {
    type Error = T::Error;
    fn unpad(&self, data: String, padding: &String, direction: PaddingDirection) -> Result<String, Self::Error> {
        (**self).unpad(data, padding, direction)
    }
}

pub struct DefaultPadder;

#[derive(Debug, PartialEq, Eq)]
pub enum PaddingError {
    PaddingLongerThanOne
}

impl DefaultPadder {
    fn get_char(padding: &String) -> Result<char, PaddingError> {
        if padding.len() > 1 {
            Err(PaddingError::PaddingLongerThanOne)
        } else {
            Ok(padding.chars().next().or(Some(' ')).expect("should have a some no matter what"))
        }
    }
}

impl Padder for DefaultPadder {
    type Error = PaddingError;
    fn pad(&self, data: String, length: usize, padding: &String, direction: PaddingDirection) -> Result<String, Self::Error> {
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
    type Error = PaddingError;
    fn unpad(&self, data: String, padding: &String, direction: PaddingDirection) -> Result<String, Self::Error> {
        Ok(match direction {
            PaddingDirection::Left => data.trim_left_matches(Self::get_char(padding)?).to_string(),
            PaddingDirection::Right => data.trim_right_matches(Self::get_char(padding)?).to_string(),
        })
    }
}

pub struct IdentityPadder;

impl Padder for IdentityPadder {
    type Error = ();
    fn pad(&self, data: String, _: usize, _: &String, _: PaddingDirection) -> Result<String, Self::Error> {
        Ok(data)
    }
}

impl UnPadder for IdentityPadder {
    type Error = ();
    fn unpad(&self, data: String, _: &String, _: PaddingDirection) -> Result<String, Self::Error> {
        Ok(data)
    }
}


#[cfg(test)]
mod test {
    use super::*;
    use spec::*;
    use std::collections::{HashMap, BTreeMap};
    use super::super::test::test_spec;

    #[test]
    fn default_padder() {
        let padder = DefaultPadder;
        let data = "qwer".to_string();
        assert_eq!(Ok("qwer333333".to_string()), padder.pad(data.clone(), 10, &"3".to_string(), PaddingDirection::Right));
        let data = "qwer".to_string();
        assert_eq!(Ok("333333qwer".to_string()), padder.pad(data.clone(), 10, &"3".to_string(), PaddingDirection::Left));
        assert_eq!(Err(PaddingError::PaddingLongerThanOne), padder.pad(data.clone(), 10, &"33".to_string(), PaddingDirection::Left));
        let data = "qwer333333".to_string();
        assert_eq!(Ok("qwer".to_string()), padder.unpad(data.clone(), &"3".to_string(), PaddingDirection::Right));
        let data = "333333qwer".to_string();
        assert_eq!(Ok("qwer".to_string()), padder.unpad(data.clone(), &"3".to_string(), PaddingDirection::Left));
        assert_eq!(Err(PaddingError::PaddingLongerThanOne), padder.unpad(data.clone(), &"33".to_string(), PaddingDirection::Left));
    }

    #[test]
    fn identity_padder() {
        let padder = IdentityPadder;
        let data = "qwer".to_string();
        assert_eq!(Ok(data.clone()), padder.pad(data.clone(), 10, &"3".to_string(), PaddingDirection::Right));
        assert_eq!(Ok(data.clone()), padder.pad(data.clone(), 10, &"3".to_string(), PaddingDirection::Left));
        assert_eq!(Ok(data.clone()), padder.unpad(data.clone(), &"3".to_string(), PaddingDirection::Right));
        assert_eq!(Ok(data.clone()), padder.unpad(data.clone(), &"3".to_string(), PaddingDirection::Left));
    }

    #[test]
    fn padder_reference() {
        let padder = IdentityPadder;
        let data = "qwer".to_string();
        assert_eq!(Ok(data.clone()), Padder::pad(&&padder, data.clone(), 10, &"3".to_string(), PaddingDirection::Right));
        assert_eq!(Ok(data.clone()), UnPadder::unpad(&&padder, data.clone(), &"3".to_string(), PaddingDirection::Right));
    }
}