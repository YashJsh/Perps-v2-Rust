use std::collections::HashMap;

use crate::engine::{helper::get_time, types::{Fill, Order, OrderSide, Position}};

pub fn check_positions(
    positions: &mut HashMap<String, Position>,
    fills: &mut HashMap<String, Vec<Fill>>,
    order_id: String,
    orders: &mut HashMap<String, Order>,
) {
    let order_fills = match fills.get(&order_id) {
        Some(fill) => fill,
        None => {
            println!("No fills exists");
            return;
        }
    };
    let order = match orders.get(&order_id) {
        Some(ord) => ord,
        None => {
            println!("Order doesn't exits");
            return;
        }
    };
    let mut price_and_qty = 0; //This is positional value;
    let mut count = 0;

    //Find the combined average fill price.
    for i in order_fills.iter() {
        count += 1;
        price_and_qty += i.price * i.qty;
    }

    let average_entry_price = price_and_qty / count;

    let margin = price_and_qty / order.leverage; //collateral

    let mut liquidation_price = price_and_qty;

    match order.order_side {
        OrderSide::Buy => liquidation_price = average_entry_price * (1 - 1 / order.leverage),
        OrderSide::Sell => liquidation_price = average_entry_price * (1 + 1 / order.leverage),
    }

    //Check if position already exists, if yes -> update, if no -> Do not update
    match positions.get_mut(&order.user_id) {
        Some(pos) => {
            //Update Position
            pos.average_entry_price = average_entry_price;
            pos.leverage = order.leverage;
            pos.margin = margin;
            pos.liquidation_price = liquidation_price;
            pos.time = get_time();
        }
        None => {
            //Create a new one;
            positions.insert(
                order.user_id.clone(),
                Position {
                    order_id,
                    average_entry_price,
                    symbol: order.symbol.clone(),
                    margin: margin,
                    size: order.size as i64,
                    liquidation_price,
                    realized_pnl: None, // Mark price - positional value;
                    leverage: order.leverage,
                    time: get_time(),
                },
            );
        }
    };
}
