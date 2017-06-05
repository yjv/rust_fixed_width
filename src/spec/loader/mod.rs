pub mod yaml;

use ::Result;
use spec::Spec;

pub trait Loader {
    fn load(&self) -> Result<Spec>;
}
