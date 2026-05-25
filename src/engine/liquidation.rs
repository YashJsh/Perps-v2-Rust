use std::collections::HashMap;
use tokio::sync::mpsc::Sender;

use crate::{
    engine::{
        create_order::create_order,
        types::{Fill, Order, OrderBook, OrderSide, OrderType, Position},
    },
    types::types::{BalanceRequest, IncomingOrder},
};

pub fn liquidation(
    index_price: u64,
    positions: &mut HashMap<String, Position>,
    orders: &mut HashMap<String, Order>,
    fills: &mut HashMap<String, Vec<Fill>>,
    book: &mut OrderBook,
    market_price : u64,
    balance_tx : &Sender<BalanceRequest>
) {
    let mut liquid_orders = Vec::new();
    for (user_id, position) in positions.iter() {
        let mut side: OrderSide;
        if position.size <= 0 {
            side = OrderSide::Sell;
        } else {
            side = OrderSide::Buy;
        }
        if should_liquidate(position, index_price) {
            liquid_orders.push(IncomingOrder {
                user_id: user_id.clone(),
                leverage: position.leverage,
                order_side: side,
                order_type: OrderType::Market,
                price: market_price,
                size: position.size as u64,
                symbol: position.symbol.clone(),
                slippage : 2
            });
        }
    }
    for i in liquid_orders {
        create_order(i, orders, book, positions, fills, balance_tx);
    }
}

fn should_liquidate(position: &Position, index_price: u64) -> bool {
    if position.size <= 0 {
        if position.liquidation_price >= index_price {
            return true;
        } else {
            return false;
        }
    } else {
        if position.liquidation_price <= index_price {
            return true;
        } else {
            return false;
        }
    }
}
