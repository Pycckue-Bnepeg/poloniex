extern crate poloniex;
extern crate env_logger;
extern crate futures;

#[macro_use]
extern crate log;

use poloniex::{Message, Subscribtion};
use futures::Future;

#[test]
fn connection() {
    env_logger::init().unwrap();

    match poloniex::connect() {
        Ok(mut client) => {
            client.subscribe(Subscribtion::BtcEth, Box::new(|msg| {
                match msg {
                    &Message::Trade(pair, _) => println!("trade for {}", pair),
                    &Message::Order(pair, _) => println!("order for {}", pair),
                    _ => println!("unknown message"),
                }
            })).wait();

            println!("subscribed");

            loop {}
        }
        Err(e) => debug!("error: {:#?}", e),
    }
}