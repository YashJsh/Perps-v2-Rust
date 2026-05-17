use std::collections::HashMap;

use crate::{
    engine::types::{Order, OrderBook, OrderType, Position},
    types::types::IncomingOrder,
};

pub fn create_order(
    data: IncomingOrder,
    orders: &mut HashMap<u64, Order>,
    book: &mut HashMap<u64, OrderBook>,
    positions : &mut Vec<Position>
) {
    let order_id = 12;
    let order_type = data.order_type.clone();
    let new_order = Order {
        order_id: order_id.to_string(),
        order_type: data.order_type,
        order_side: data.order_side,
        symbol: data.symbol,
        size: data.size,
        price: data.price,
        leverage: data.leverage,
    };
    orders.insert(order_id, new_order);

    match order_type{
        OrderType::Market => {
            handleMarketOrder(order_id, book, positions);
        }
        OrderType::Limit => {
            handleLimitOrder(order_id, book, positions);
        }
    }
}

//What are the things i need to do for limit order;
fn handleLimitOrder(order_id : u64, book :  &mut HashMap<u64, OrderBook>, positions : &mut Vec<Position>){
   
}


fn handleMarketOrder(order_id : u64, book :  &mut HashMap<u64, OrderBook>, positions : &mut Vec<Position>){
   
}
