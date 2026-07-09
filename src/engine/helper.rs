use std::collections::{HashMap, VecDeque};

use chrono::Local;

use crate::engine::types::{OrderBook, Position, RestingOrder};

pub fn risk_engine(positions: &HashMap<u64, Position>, order_size: i64, user_id: u64) -> bool {
    let mut output;
    match positions.get(&user_id) {
        Some(pos) => {
            let risk = order_size + pos.size;
            if risk.abs() < pos.size.abs() {
                output = false;
            } else {
                output = true;
            }
            if pos.size.signum() != risk.signum(){
                output = true;
            }
            if risk == 0{
                output = false;
            }
        }
        None => {
            output = true;
        }
    };
    output
}


pub fn get_time() -> String {
    let local = Local::now();
    local.to_string()
}

pub fn add_in_bids(book: &mut OrderBook, resting_order: RestingOrder) {
    let price = resting_order.price;
    let bid_side = book.bids.entry(price).or_insert(VecDeque::new());
    bid_side.push_back(resting_order);
    println!("Added in orderbook ");
    println!("Bid Side Book looks like : {:?}", bid_side);
    return;
}

pub fn add_in_sorts(book: &mut OrderBook, resting_order: RestingOrder) {
    let price = resting_order.price;
    let ask_side = book.asks.entry(price).or_insert(VecDeque::new());
    ask_side.push_back(resting_order);
    println!("Added in orderbook ");
    println!("Ask side looks like : {:?}", ask_side);
    return;
}

pub fn check_required_margin(size : u64, price : u64, leverage : u64)-> u64{
    let notional_value = size * price;
    let amount_needed = notional_value / leverage;
    amount_needed
}