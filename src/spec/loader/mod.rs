pub mod yaml;

use ::Result;
use spec::Spec;

pub trait Loader<T> {
    fn load(&self, resource: T) -> Result<Spec>;
}

pub trait MultiLoader<T> {
    fn multi_load(&self, resource: T) -> Result<Vec<Spec>>;
}