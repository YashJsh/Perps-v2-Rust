use std::collections::HashMap;

use crate::engine::{
    helper::get_time,
    types::{Fill, Order, OrderSide, Position},
};

pub fn check_positions(
    positions: &mut HashMap<String, Position>,
    fills: &mut HashMap<String, Vec<Fill>>,
    order_id: String,
    orders: &mut HashMap<String, Order>,
) {
    let order_fills = match fills.get_mut(&order_id) {
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
    let order_size;
    match &order.order_side {
        OrderSide::Buy => order_size = order.size as i64,
        OrderSide::Sell => order_size = -(order.size as i64),
    };

    let position = match positions.get_mut(&order.user_id) {
        Some(pos) => pos,
        None => {
            //Create new fresh position
            create_fresh_position(positions, order_fills, order);
            return;
        }
    };

    let mut notional_value = 0;
    let mut total_qty = 0;

    for fill in order_fills.iter_mut() {
        notional_value += fill.price * fill.qty;
        total_qty += fill.qty;
        fill.consumed = true;
    }

    //Same side order
    if order_size.signum() == position.size.signum() {
        //Same side buy same side sell.
        let new_positional_notional_value =
            (position.average_entry_price * position.size.abs() as u64) + (notional_value);
        let den = position.size.abs() as u64 + total_qty;
        let new_average_entry = new_positional_notional_value / den;
        position.average_entry_price = new_average_entry;
        position.size += order_size;
        position.margin = new_positional_notional_value / order.leverage;
        let liquidation_price = match order.order_side {
            OrderSide::Buy => {
                new_average_entry - new_average_entry / order.leverage
            }
            OrderSide::Sell => {
                new_average_entry + new_average_entry / order.leverage
            }
        };
        position.liquidation_price = liquidation_price;
        position.time = get_time();
    } else {
        //No same side order
        //Here there are there cases :
        // 1. Partial Reduction.
        // 2. Full Close
        // 3. Flip position
    }
}

fn create_fresh_position(
    positions: &mut HashMap<String, Position>,
    order_fills: &mut Vec<Fill>,
    order: &Order,
) {
    let mut notional_value = 0;
    let mut total_qty = 0;

    for fill in order_fills.iter_mut() {
        notional_value += fill.price * fill.qty;
        total_qty += fill.qty;
        fill.consumed = true;
    }

    let average_entry_price = notional_value / total_qty;
    let margin = notional_value / order.leverage;

    let liquidation_price = match order.order_side {
        OrderSide::Buy => average_entry_price - average_entry_price / order.leverage,
        OrderSide::Sell => average_entry_price + average_entry_price / order.leverage,
    };

    let total_signed_qty;

    match order.order_side {
        OrderSide::Buy => total_signed_qty = total_qty as i64,
        OrderSide::Sell => total_signed_qty = -(total_qty as i64),
    };

    positions.insert(
        order.user_id.clone(),
        Position {
            order_id: order.order_id.clone(),
            average_entry_price,
            symbol: order.symbol.clone(),
            margin,
            size: total_signed_qty,
            liquidation_price,
            realized_pnl: None,
            time: get_time(),
            leverage: order.leverage,
        },
    );
}

fn same_side_fills() {}
