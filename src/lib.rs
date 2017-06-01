extern crate serde;
extern crate ws;
extern crate futures;

#[macro_use]
extern crate log;

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate serde_json;

pub mod client;
pub mod error;
pub use client::{connect, Message, Subscribtion};
pub mod types;