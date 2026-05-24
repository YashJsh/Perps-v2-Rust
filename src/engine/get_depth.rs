use std::collections::HashMap;

use crate::engine::types::{DepthResponse, OrderBook};

pub fn get_depth(book: &OrderBook) -> DepthResponse {
    let asks = &book.asks;
    let bids = &book.bids;

    let mut asks_hash: HashMap<u64, u64> = HashMap::new();
    let mut bids_hash: HashMap<u64, u64> = HashMap::new();

    for (price, orders) in asks.iter().take(10) {
        let mut total_qty = 0;

        for order in orders {
            total_qty += order.remaining_qty;
        }

        asks_hash.insert(*price, total_qty);
    }

    for (price, orders) in bids.iter().rev().take(10) {
        let mut total_qty = 0;

        for order in orders {
            total_qty += order.remaining_qty;
        }

        bids_hash.insert(*price, total_qty);
    }

    println!("Asks: {:?}", asks_hash);
    println!("Bids: {:?}", bids_hash);

    DepthResponse {
        success: true,
        bids : bids_hash,
        asks : asks_hash
    }
}
