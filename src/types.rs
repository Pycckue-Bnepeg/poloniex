use serde_json::Value as JSON;

// Array[ EVENT_ID, SEQUENCE, Array[DATA ...] ]
// SEQUENCE = 1 -> subscribe
// SEQUENCE = 0 -> unsubscribe

#[derive(Debug, Serialize, Deserialize)]
pub struct Response(u32, Option<u32>, JSON);

#[derive(Debug, Serialize, Deserialize)]
pub struct Order(String, pub u32, String, String);

#[derive(Debug, Serialize, Deserialize)]
pub struct Trade(String, String, pub u32, String, String, u32);
/*  type?, id, type, rate, amount, date
    when type is i, there will be next struct:
    [ EVENT_ID, SEQ, [
                        ["i", {
                            currencyPair: "1_2",
                            orderBook: [{ price_1: price_2 }, { price_2: price_1 }] // asks, bids
                         }]
    ]]
*/

#[derive(Debug, Serialize, Deserialize)]
pub struct Tick(pub u32, pub String, pub String, pub String, pub String, pub String, pub String, pub u32, pub String, pub String);
// pair, last, lowestAsk, highestBid, percentChanges, baseVolime, quoteVolume, isFrozen, 24hr_high, 24hr_low
/*
    Number(PosInt(14)),
    String("0.00002564"),
    String("0.00002564"),
    String("0.00002550"),
    String("-0.08656929"),
    String("1151.37154082"),
    String("42898945.60692950"),
    Number(PosInt(0)),
    String("0.00002875"),
    String("0.00002500")
*/

#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseError {
    error: String,
}

pub enum OrderType {
    Ask,
    Bid,
}

pub enum TradeType {
    Buy,
    Sell,
}

impl Response {
    pub fn get_event(&self) -> u32 {
        self.0
    }

    pub fn get_sequence(&self) -> &Option<u32> {
        &self.1
    }

    pub fn get_data(&self) -> &JSON {
        &self.2
    }
}

impl Order {
    pub fn get_type(&self) -> OrderType {
        if self.1 == 1 { OrderType::Bid } else { OrderType::Ask }
    }

    pub fn get_rate(&self) -> f32 {
        self.2.parse().unwrap()
    }

    pub fn get_amount(&self) -> f32 {
        self.3.parse().unwrap()
    }
}

impl Trade {
    pub fn get_id(&self) -> u32 {
        self.1.parse().unwrap()
    }

    pub fn get_type(&self) -> TradeType {
        if self.2 == 1 { TradeType::Buy } else { TradeType::Sell }
    }

    pub fn get_rate(&self) -> f32 {
        self.3.parse().unwrap()
    }

    pub fn get_amount(&self) -> f32 {
        self.4.parse().unwrap()
    }

    pub fn get_date(&self) -> u32 {
        self.5
    }

    pub fn get_total(&self) -> f32 {
        self.get_rate() * self.get_amount()
    }
}