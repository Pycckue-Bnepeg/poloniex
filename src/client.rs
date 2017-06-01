// TODO: documentation
use ws;
use serde_json;
use std;
use types;

use ws::{Handler, Result as WSResult, Message as WSMessage};
use ws::util::{Timeout, Token};
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
const TIMEOUT: Token = Token(1000);
const PING: Token = Token(1001);

type SyncCon = Arc<Mutex<Connection>>;
type ConnectionResult = Result<SyncCon, PoloniexError>;
type Subscribers = HashMap<Subscribtion, Vec<Subscriber>>;

pub struct CallbackWrapper {
    callback: Box<FnMut(&Message)>,
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
    Tick(Tick),
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
    timeout: Option<Timeout>,
}

pub struct Connection {
    socket: ws::Sender,
    subscribers: Subscribers,
    // TODO: reason
    event_close: Option<oneshot::Sender<()>>,
}

#[derive(Hash, Eq, PartialEq, Debug)]
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
            148 => Subscribtion::BtcEth,
            _ => Subscribtion::Unknown,
        }
    }
    // TODO: to_int
}

impl PoloniexHandler {
    pub fn new(rx: Sender<ConnectionResult>, connection: SyncCon) -> PoloniexHandler {
        PoloniexHandler {
            rx,
            connection,
            timeout: None,
        }
    }
}

impl Handler for PoloniexHandler {
    fn on_open(&mut self, h: ws::Handshake) -> WSResult<()> {
        debug!("sockets are opened");

        let con = self.connection.lock().unwrap();
        con.socket.timeout(60000, PING);

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
                        if let Some(list) = data.as_array() {
                            // godlike! there is information about new subscribtion (there is an order book, more info in types.rs)
                            if list[0][0] == json!("i") {
                                let sub = Subscribtion::from_int(response.get_event());
                                self.notify_subscribers(sub);
                            }
                            for item in data.as_array().unwrap() {
                                if let Ok(trade) = serde_json::from_value(item.clone()) {
                                    self.handle_trade(id, trade);
                                } else if let Ok(order) = serde_json::from_value(item.clone()) {
                                    self.handle_order(id, order);
                                }
                            }
                        }
                    },
                    _ => (),
                }
            } else if let Ok(response) = serde_json::from_str::<Vec<u32>>(&text) {
                match response[0] {
                    1010 => (),
                    id @ _ => {
                        let sub = Subscribtion::from_int(id);
                        self.notify_subscribers(sub);
                    },
                }
            } else {
                debug!("something wrong ... {:?}", text);
            }
        }

        Ok(())
    }

    fn on_timeout(&mut self, token: Token) -> WSResult<()> {
        if token == PING {
            self.send_raw('.')?;
        }

        // TODO: notify subscribers when close
        if token == TIMEOUT {
            debug!("lost connection");
            let mut con = self.connection.lock().unwrap();
            con.socket.close(ws::CloseCode::Away)?;

            if let Some(tx) = con.event_close.take() {
                tx.send(()).unwrap();
            }
        }

        Ok(())
    }

    fn on_new_timeout(&mut self, token: Token, timeout: Timeout) -> WSResult<()> {
        if token == TIMEOUT {
            if let Some(last_timeout) = self.timeout.take() {
                let con = self.connection.lock().unwrap();
                con.socket.cancel(last_timeout)?;
            }
            self.timeout = Some(timeout);
        }

        Ok(())
    }

    fn on_frame(&mut self, frame: ws::Frame) -> WSResult<Option<ws::Frame>> {
        let con = self.connection.lock().unwrap();

        con.socket.timeout(60000, TIMEOUT)?;

        Ok(Some(frame))
    }

    // TODO: error handler
}

impl PoloniexHandler {
    // TODO: parse as normal tick
    fn handle_tick(&mut self, tick: Tick) {
        let mut con = self.connection.lock().unwrap();

        if let Some(ref mut list) = con.subscribers.get_mut(&Subscribtion::Ticker) {
            let message = Message::Tick(tick);
            for item in list.iter_mut() {
                (item.callback.callback)(&message);
            }
        }
    }

    fn handle_trade(&mut self, pair: u32, trade: Trade) {
        let mut con = self.connection.lock().unwrap();

        if let Some(ref mut list) = con.subscribers.get_mut(&Subscribtion::from_int(pair)) {
            let message = Message::Trade(pair, trade);
            for item in list.iter_mut() {
                (item.callback.callback)(&message);
            }
        }
    }

    fn handle_order(&mut self, pair: u32, order: Order) {
        let mut con = self.connection.lock().unwrap();

        if let Some(ref mut list) = con.subscribers.get_mut(&Subscribtion::from_int(pair)) {
            let message = Message::Order(pair, order);
            for item in list.iter_mut() {
                (item.callback.callback)(&message);
            }
        }
    }

    fn send_msg(&self, msg: &JSON) -> WSResult<()> {
        self.send_raw(serde_json::to_string(&msg).unwrap())
    }

    fn send_raw<T: ToString>(&self, msg: T) -> WSResult<()> {
        let con = self.connection.lock().unwrap();
        con.socket.send(msg.to_string())
    }

    fn notify_subscribers(&mut self, sub: Subscribtion) {
        let mut con = self.connection.lock().unwrap();

        if let Some(ref mut list) = con.subscribers.get_mut(&sub) {
            for item in list.iter_mut() {
                if let Some(mut tx) = item.future.take() {
                    tx.send(()).unwrap();
                }
            }
        }
    }
}

impl Connection {
    pub fn subscribe(&mut self, sub: Subscribtion, tx: FSender<()>, callback: CallbackWrapper) {
        let subscriber = Subscriber {
            future: Some(tx),
            callback,
        };

        if self.subscribers.contains_key(&sub) {
            let mut list = self.subscribers.get_mut(&sub).unwrap();
            list.push(subscriber);
        } else {
            self.subscribers.insert(sub, vec![subscriber]);
        }
    }
}

impl Poloniex {
    pub fn subscribe(&mut self, sub: Subscribtion, cb: Box<FnMut(&Message)>) -> sync::oneshot::Receiver<()> {
        // TODO: remove match in function Subscribion::to_int
        let request = json!({
            "command": "subscribe",
            "channel": match sub {
                Subscribtion::Ticker => "1002",
                Subscribtion::BtcEth => "148",
                Subscribtion::Pair(ref pair) => pair.as_str(),
                _ => "1002",
            }
        });

        let (tx, rx) = oneshot();

        let callback = CallbackWrapper {
            callback: cb,
        };

        let mut con = self.connection.lock().unwrap();

        debug!("subscribed to {:?}", sub);
        con.subscribe(sub, tx, callback);

        send_message(&con, &request);

        rx
    }

    pub fn wait_close(&mut self) -> sync::oneshot::Receiver<()> {
        let mut con = self.connection.lock().unwrap();
        let (tx, rx) = oneshot();
        con.event_close = Some(tx);

        rx
    }

    pub fn shutdown(&mut self) {
        let mut con = self.connection.lock().unwrap();
        con.socket.close(ws::CloseCode::Away);

        if let Some(tx) = con.event_close.take() {
            tx.send(()).unwrap();
        }
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
                event_close: None,
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