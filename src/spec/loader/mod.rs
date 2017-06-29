pub mod yaml;

use ::BoxedErrorResult;
use spec::Spec;

pub trait Loader<T> {
    fn load(&self, resource: T) -> BoxedErrorResult<Spec>;
}