use std::collections::{HashMap, VecDeque};

use crate::engine::{helper::get_time, types::{EngineError, Fill, Order, OrderBook, RestingOrder}};

pub fn core_buy_logic(
    incoming_qty: u64,
    incoming_price: u64,
    book: &mut OrderBook,
    orders: &mut HashMap<String, Order>,
    fills: &mut HashMap<String, Vec<Fill>>,
    order_id: String,
) -> Result<u64, EngineError> {
    let incoming_order_id = order_id.clone();
    let mut incoming_remaining_qty = incoming_qty;
    {
        fills.entry(order_id.clone()).or_insert(Vec::new());
    }

    while incoming_remaining_qty > 0 {
        let mut entry = match book.asks.first_entry() {
            Some(entry) => entry,
            None => {
                break;
            }
        };
        let price = *entry.key();
        let asks: &mut VecDeque<RestingOrder> = entry.get_mut();

        if (price <= incoming_price) {
            while let Some(front) = asks.front_mut() {
                let selling_order = front;

                let matched_qty =
                    std::cmp::min(incoming_remaining_qty, selling_order.remaining_qty);

                let buyer_fills = fills
                    .entry(incoming_order_id.clone())
                    .or_insert(Vec::new())
                    .push(Fill {
                        order_id: incoming_order_id.clone(),
                        maker_id: selling_order.order_id.clone(),
                        taker_id: incoming_order_id.clone(),
                        price: selling_order.price,
                        qty: matched_qty,
                        symbol: selling_order.symbol.clone(),
                        consumed : false,
                        time: get_time(),
                    });

                let seller_fills = fills
                    .entry(selling_order.order_id.clone())
                    .or_insert(Vec::new())
                    .push(Fill {
                        order_id: selling_order.order_id.clone(),
                        maker_id: selling_order.order_id.clone(),
                        taker_id: incoming_order_id.clone(),
                        price: selling_order.price,
                        qty: matched_qty,
                        symbol: selling_order.symbol.clone(),
                        consumed : false,
                        time: get_time(),
                    });

                let buy_order = match orders.get_mut(&order_id) {
                    Some(ord) => {
                        ord.filled_qty += matched_qty;
                        ord.remaining_qty -= matched_qty;
                    }
                    None => return Err(EngineError::OrderNotFound),
                };

                let sell_order = match orders.get_mut(&selling_order.order_id) {
                    Some(ord) => {
                        ord.filled_qty += matched_qty;
                        ord.remaining_qty -= matched_qty;
                    }
                    None => return Err(EngineError::OrderNotFound),
                };

                selling_order.remaining_qty -= matched_qty;
                incoming_remaining_qty -= matched_qty;

                if selling_order.remaining_qty == 0 {
                    asks.pop_front();
                }

                if incoming_remaining_qty == 0 {
                    break;
                }
            }
        } else {
            break;
        }

        if (asks.is_empty()) {
            book.asks.remove(&price);
        }
    }
    Ok(incoming_remaining_qty)
}

pub fn core_sell_logic(
    incoming_qty: u64,
    incoming_price: u64,
    book: &mut OrderBook,
    orders: &mut HashMap<String, Order>,
    fills: &mut HashMap<String, Vec<Fill>>,
    order_id: String,
) -> Result<u64, EngineError> {
    let incoming_order_id = order_id.clone();
    let mut incoming_remaining_qty = incoming_qty;
    {
        fills.entry(order_id.clone()).or_insert(Vec::new());
    }

    while incoming_remaining_qty > 0 {
        let mut entry = match book.bids.last_entry() {
            Some(entry) => entry,
            None => {
                break;
            }
        };
        let price = *entry.key();
        let asks: &mut VecDeque<RestingOrder> = entry.get_mut();

        if price >= incoming_price {
            while let Some(front) = asks.front_mut() {
                let selling_order = front;

                let matched_qty =
                    std::cmp::min(incoming_remaining_qty, selling_order.remaining_qty);

                let buyer_fills = fills
                    .entry(incoming_order_id.clone())
                    .or_insert(Vec::new())
                    .push(Fill {
                        order_id: incoming_order_id.clone(),
                        maker_id: selling_order.order_id.clone(),
                        taker_id: incoming_order_id.clone(),
                        price: selling_order.price,
                        qty: matched_qty,
                        symbol: selling_order.symbol.clone(),
                        consumed : false,
                        time: get_time(),
                    });

                let seller_fills = fills
                    .entry(selling_order.order_id.clone())
                    .or_insert(Vec::new())
                    .push(Fill {
                        order_id: selling_order.order_id.clone(),
                        maker_id: selling_order.order_id.clone(),
                        taker_id: incoming_order_id.clone(),
                        price: selling_order.price,
                        qty: matched_qty,
                        symbol: selling_order.symbol.clone(),
                        consumed : false,
                        time: get_time(),
                    });

                let buy_order = match orders.get_mut(&order_id) {
                    Some(ord) => {
                        ord.filled_qty += matched_qty;
                        ord.remaining_qty -= matched_qty;
                    }
                    None => return Err(EngineError::OrderNotFound),
                };

                let sell_order = match orders.get_mut(&selling_order.order_id) {
                    Some(ord) => {
                        ord.filled_qty += matched_qty;
                        ord.remaining_qty -= matched_qty;
                    }
                    None => return Err(EngineError::OrderNotFound),
                };

                selling_order.remaining_qty -= matched_qty;
                incoming_remaining_qty -= matched_qty;

                if selling_order.remaining_qty == 0 {
                    asks.pop_front();
                }

                if incoming_remaining_qty == 0 {
                    break;
                }
            }
        } else {
            break;
        }

        if (asks.is_empty()) {
            book.asks.remove(&price);
        }
    }
    Ok(incoming_remaining_qty)
}
