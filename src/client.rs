use ws;
use serde_json;
use std;
use types;

use ws::{Handler, Result as WSResult, Message as WSMessage};
use serde_json::Value as JSON;
use std::sync::mpsc::{Sender, channel};
use error::PoloniexError;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use futures::*;
use futures::sync::oneshot;
use futures::sync::oneshot::Sender as FSender;

use types::{Response, Order, Trade, Tick};

const API_PLX: &'static str = "wss://api2.poloniex.com";

type SyncCon = Arc<Mutex<Connection>>;
type ConnectionResult = Result<SyncCon, PoloniexError>;
type Subscribers = HashMap<Subscribtion, Vec<Subscriber>>;

pub struct CallbackWrapper {
    callback: Box<FnMut()>,
}

unsafe impl <'a> Send for CallbackWrapper {}
unsafe impl<'a> Sync for CallbackWrapper {}

#[derive(Debug)]
pub enum Message {
    Heathbeat,
    Subscribe(u32),
    Unsubscribe(u32),
    Trade(u32, Trade), // Pair, Trade
    Order(u32, Order), // Pair, Order
}

pub struct Subscriber {
    future: Option<FSender<()>>,
    callback: CallbackWrapper,
}

pub struct Poloniex {
    connection: SyncCon,
}

pub struct PoloniexHandler {
    connection: SyncCon,
    rx: Sender<ConnectionResult>,
}

pub struct Connection {
    socket: ws::Sender,
    subscribers: Subscribers,
}

#[derive(Hash, Eq, PartialEq)]
pub enum Subscribtion {
    Unknown, // 1000 - something linked to account
    TrollBox, // 1001
    Ticker, // 1002
    Hearthbeat, // 1010
    Pair(String), // 0..1000 - market channels for single currency
    BtcEth, // 148
}

impl Subscribtion {
    pub fn from_int(id: u32) -> Subscribtion {
        match id {
            1000 => Subscribtion::Unknown,
            1001 => Subscribtion::TrollBox,
            1002 => Subscribtion::Ticker,
            1010 => Subscribtion::Hearthbeat,
            _ => Subscribtion::Unknown,
        }
    }
}

impl PoloniexHandler {
    pub fn new(rx: Sender<ConnectionResult>, connection: SyncCon) -> PoloniexHandler {
        PoloniexHandler {
            rx,
            connection,
        }
    }
}

impl Handler for PoloniexHandler {
    fn on_open(&mut self, h: ws::Handshake) -> WSResult<()> {
        debug!("sockets are opened");
        self.rx.send(Ok(self.connection.clone())).unwrap();
        Ok(())
    }

    fn on_message(&mut self, msg: WSMessage) -> WSResult<()> {
        if let Ok(text) = msg.into_text() {
            if let Ok(response) = serde_json::from_str::<Response>(&text) {
                let data = response.get_data();
                match response.get_event() {
                    1000 => unimplemented!(),
                    1001 => unimplemented!(),
                    1002 => {
                        if let Ok(tick) = serde_json::from_value(data.clone()) {
                            self.handle_tick(tick);
                        }
                    },
                    1003 => unimplemented!(),
                    id @ 1...1000 => {
                        for item in data.as_array().unwrap() {
                            if let Ok(trade) = serde_json::from_value(item.clone()) {
                                self.handle_trade(trade);
                            } else if let Ok(order) = serde_json::from_value(item.clone()) {
                                self.handle_order(order);
                            }
                        }
                    },
                    _ => (),
                }
                // TODO: subscribtions
            } else if let Ok(response) = serde_json::from_str::<Vec<u32>>(&text) {
                match response[0] {
                    1010 => (),
                    id @ _ => {
                        let mut con = self.connection.lock().unwrap();
                        let sub = Subscribtion::from_int(id);

                        if let Some(ref mut list) = con.subscribers.get_mut(&sub) {
                            for item in list.iter_mut() {
                                if let Some(mut tx) = item.future.take() {
                                    tx.send(()).unwrap();
                                }
                            }
                        }
                    },
                }
            } else {
                debug!("something wrong ... {:?}", text);
            }
        }

        Ok(())
    }
}

impl PoloniexHandler {
    // TODO: parse as normal tick
    fn handle_tick(&mut self, tick: Tick) {
//        debug!("new tick: {:?}", tick);
    }

    fn handle_trade(&mut self, trade: Trade) {
//        debug!("new trade: {:?}", trade);
    }

    fn handle_order(&mut self, order: Order) {
//        debug!("new order: {:?}", order);
    }

    fn send_msg(&self, msg: &JSON) {
        let con = self.connection.lock().unwrap();
        send_message(&con, msg).unwrap();
    }
}

impl Connection {
    pub fn subscribe(&mut self, sub: Subscribtion, tx: FSender<()>, callback: CallbackWrapper) {
        let subscriber = Subscriber {
            future: Some(tx),
            callback,
        };

        if self.subscribers.contains_key(&sub) {
            self.subscribers.get_mut(&sub).unwrap();
        } else {
            self.subscribers.insert(sub, vec![subscriber]);
        }
    }
}

impl Poloniex {
    pub fn subscribe(&mut self, sub: Subscribtion, cb: Box<FnMut()>) -> sync::oneshot::Receiver<()> {
        let request = json!({
            "command": "subscribe",
            "channel": match sub {
                Subscribtion::Ticker => "1002",
                Subscribtion::Pair(ref pair) => pair.as_str(),
                _ => "1002",
            }
        });

        let (tx, rx) = oneshot();

        let callback = CallbackWrapper {
            callback: cb,
        };

        let mut con = self.connection.lock().unwrap();

        con.subscribe(sub, tx, callback);

        send_message(&con, &request);

        rx
    }
}

pub fn connect() -> Result<Poloniex, PoloniexError> {
    debug!("starting connection");

    let (tx, rx) = channel();

    std::thread::spawn(move || {
        let connection_res = ws::connect(API_PLX, |out| {
            let connection = Arc::new(Mutex::new(Connection {
                socket: out,
                subscribers: HashMap::new(),
            }));

            let handler = PoloniexHandler::new(tx.clone(), connection);

            handler
        });

        match connection_res {
            Ok(_) => (),
            Err(e) => tx.send(Err(PoloniexError::WSError(e))).unwrap(),
        }
    });

    match rx.recv().unwrap() {
        Ok(con) => Ok(Poloniex {
            connection: con,
        }),
        Err(e) => Err(e),
    }
}

fn send_message(con: &Connection, msg: &JSON) -> WSResult<()> {
    con.socket.send(serde_json::to_string(msg).unwrap())
}