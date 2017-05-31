extern crate poloniex;
extern crate env_logger;
extern crate futures;

#[macro_use]
extern crate log;

use poloniex::Subscribtion;
use futures::Future;

#[test]
fn connection() {
    env_logger::init().unwrap();

    match poloniex::connect() {
        Ok(mut client) => {
            client.subscribe(Subscribtion::Ticker, Box::new(|| {

            })).wait();

            loop {}
        }
        Err(e) => debug!("error: {:#?}", e),
    }
}