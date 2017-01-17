#[cfg(test)]
#[macro_use]
pub mod test;
pub mod spec;
pub mod record;
pub mod padders;

//!This lib is built to make reading/writing fixed width data alot easier. It uses a
//!the idea of a spec to define the kinds of records youd line to be able to read off
//!of or write a data stream and allows you to read and/or write them to data collections
//!
//! #Example
//!```
//!