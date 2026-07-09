use std::collections::{HashMap, VecDeque};

use crate::engine::{
    helper::get_time,
    types::{EngineError, Fill, Order, OrderBook, OrderStatus, RestingOrder},
};

pub fn core_buy_logic(
    incoming_qty: u64,
    incoming_price: u64,
    book: &mut OrderBook,
    orders: &mut HashMap<u64, Order>,
    fills: &mut HashMap<u64, Vec<Fill>>,
    order_id: u64,
) -> Result<u64, EngineError> {
    let incoming_order_id = order_id;
    let mut incoming_remaining_qty = incoming_qty;
    {
        fills.entry(order_id).or_insert(Vec::new());
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

        if price <= incoming_price {
            while let Some(front) = asks.front_mut() {
                let selling_order = front;

                let matched_qty =
                    std::cmp::min(incoming_remaining_qty, selling_order.remaining_qty);

                let _buyer_fills = fills
                    .entry(incoming_order_id)
                    .or_insert(Vec::new())
                    .push(Fill {
                        order_id: incoming_order_id,
                        maker_id: selling_order.order_id,
                        taker_id: incoming_order_id,
                        price: selling_order.price,
                        qty: matched_qty,
                        symbol: selling_order.symbol.clone(),
                        consumed: false,
                        time: get_time(),
                    });

                let _seller_fills = fills
                    .entry(selling_order.order_id)
                    .or_insert(Vec::new())
                    .push(Fill {
                        order_id: selling_order.order_id,
                        maker_id: selling_order.order_id,
                        taker_id: incoming_order_id,
                        price: selling_order.price,
                        qty: matched_qty,
                        symbol: selling_order.symbol.clone(),
                        consumed: false,
                        time: get_time(),
                    });

                //This is resting order update;
                selling_order.remaining_qty -= matched_qty;
                incoming_remaining_qty -= matched_qty;

                let _buy_order = match orders.get_mut(&order_id) {
                    Some(ord) => {
                        ord.filled_qty += matched_qty;
                        ord.remaining_qty -= matched_qty;
                        if ord.remaining_qty < ord.size {
                            ord.status = OrderStatus::PartiallyFilled;
                        }
                        if ord.remaining_qty == 0 {
                            ord.status = OrderStatus::Filled;
                        }
                    }
                    None => return Err(EngineError::OrderNotFound),
                };

                let _sell_order = match orders.get_mut(&selling_order.order_id) {
                    Some(ord) => {
                        ord.filled_qty += matched_qty;
                        ord.remaining_qty -= matched_qty;
                        if ord.remaining_qty < ord.size {
                            ord.status = OrderStatus::PartiallyFilled;
                        }
                        if ord.remaining_qty == 0 {
                            ord.status = OrderStatus::Filled;
                        }
                    }
                    None => return Err(EngineError::OrderNotFound),
                };

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

        if asks.is_empty() {
            book.asks.remove(&price);
        }
    }
    Ok(incoming_remaining_qty)
}

pub fn core_sell_logic(
    incoming_qty: u64,
    incoming_price: u64,
    book: &mut OrderBook,
    orders: &mut HashMap<u64, Order>,
    fills: &mut HashMap<u64, Vec<Fill>>,
    order_id: u64,
) -> Result<u64, EngineError> {
    let incoming_order_id = order_id;
    let mut incoming_remaining_qty = incoming_qty;
    {
        fills.entry(order_id).or_insert(Vec::new());
    }

    while incoming_remaining_qty > 0 {
        let mut entry = match book.bids.last_entry() {
            Some(entry) => entry,
            None => {
                break;
            }
        };
        let price = *entry.key();
        let bids: &mut VecDeque<RestingOrder> = entry.get_mut();

        if price >= incoming_price {
            while let Some(front) = bids.front_mut() {
                let buying_order = front;

                let matched_qty = std::cmp::min(incoming_remaining_qty, buying_order.remaining_qty);

                let _buyer_fills = fills
                    .entry(incoming_order_id)
                    .or_insert(Vec::new())
                    .push(Fill {
                        order_id: incoming_order_id,
                        maker_id: buying_order.order_id,
                        taker_id: incoming_order_id,
                        price: buying_order.price,
                        qty: matched_qty,
                        symbol: buying_order.symbol.clone(),
                        consumed: false,
                        time: get_time(),
                    });

                let _seller_fills = fills
                    .entry(buying_order.order_id)
                    .or_insert(Vec::new())
                    .push(Fill {
                        order_id: buying_order.order_id,
                        maker_id: buying_order.order_id,
                        taker_id: incoming_order_id,
                        price: buying_order.price,
                        qty: matched_qty,
                        symbol: buying_order.symbol.clone(),
                        consumed: false,
                        time: get_time(),
                    });

                buying_order.remaining_qty -= matched_qty;
                incoming_remaining_qty -= matched_qty;

                let _sell_order = match orders.get_mut(&order_id) {
                    Some(ord) => {
                        ord.filled_qty += matched_qty;
                        ord.remaining_qty -= matched_qty;
                        if ord.remaining_qty < ord.size {
                            ord.status = OrderStatus::PartiallyFilled
                        }
                        if ord.remaining_qty == 0 {
                            ord.status = OrderStatus::Filled
                        }
                    }
                    None => return Err(EngineError::OrderNotFound),
                };

                let _sell_order = match orders.get_mut(&buying_order.order_id) {
                    Some(ord) => {
                        ord.filled_qty += matched_qty;
                        ord.remaining_qty -= matched_qty;
                        if ord.remaining_qty < ord.size {
                            ord.status = OrderStatus::PartiallyFilled
                        }
                        if ord.remaining_qty == 0 {
                            ord.status = OrderStatus::Filled
                        }
                    }
                    None => return Err(EngineError::OrderNotFound),
                };

                if buying_order.remaining_qty == 0 {
                    bids.pop_front();
                }

                if incoming_remaining_qty == 0 {
                    break;
                }
            }
        } else {
            break;
        }

        if bids.is_empty()  {
            book.bids.remove(&price);
        }
    }
    Ok(incoming_remaining_qty)
}
