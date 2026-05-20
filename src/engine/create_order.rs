use std::collections::HashMap;

use chrono::Local;
use uuid::Uuid;

use crate::{
    engine::types::{Fill, Order, OrderBook, OrderSide, OrderType, Position, RestingOrder},
    types::types::IncomingOrder,
};

pub fn create_order(
    data: IncomingOrder,
    orders: &mut HashMap<String, Order>,
    book: &mut HashMap<u64, OrderBook>,
    positions: &mut HashMap<String, Position>,
    fills: &mut HashMap<String, Vec<Fill>>,
) {
    let order_id = Uuid::new_v4().to_string();
    let order_type = data.order_type.clone();
    let new_order = Order {
        user_id: data.user_id,
        order_id: order_id.to_string(),
        order_type: data.order_type,
        order_side: data.order_side,
        symbol: data.symbol,
        size: data.size as u64,
        price: data.price,
        leverage: data.leverage,
    };
    orders.insert(order_id.clone(), new_order);

    match order_type {
        OrderType::Market => {
            handleMarketOrder(order_id, book, positions, orders, fills);
        }
        OrderType::Limit => {
            handleLimitOrder(order_id, book, positions, orders, fills);
        }
    }
}

//What are the things i need to do for limit order;
fn handleLimitOrder(
    order_id: String,
    book: &mut HashMap<u64, OrderBook>,
    positions: &mut HashMap<String, Position>,
    orders: &mut HashMap<String, Order>,
    fills: &mut HashMap<String, Vec<Fill>>,
) {
    let (
        incoming_ord_price,
        incmoing_ord_size,
        incoming_ord_symbol,
        incoming_ord_side,
        incoming_order_user_id,
    ) = match orders.get(&order_id) {
        Some(ord) => (
            ord.price.unwrap(),
            ord.size,
            ord.symbol.clone(),
            ord.order_side.clone(),
            ord.user_id.clone(),
        ),
        None => {
            println!("No order found");
            return;
        }
    };

    let incoming_ord_id = order_id.clone();

    //Here risk engine will run. And according to the bool result
    //It will check or not check the collateral, balance for the new order.

    match incoming_ord_side {
        OrderSide::Buy => {
            fills.insert(order_id, Vec::new());

            let mut prices: Vec<u64> = book.keys().copied().collect();
            prices.sort();

            let mut incoming_remaining_qty = incmoing_ord_size;

            for i in 0..prices.len() {
                let price = prices[i];
                let price_orders = match book.get_mut(&price) {
                    Some(ord) => ord,
                    None => {
                        continue;
                    }
                };
                if incoming_remaining_qty == 0 {
                    println!("Order is matched fully nothing is left to match");
                    return;
                }
                if &price <= &incoming_ord_price {
                    //Loop through the orders;
                    for p in price_orders.asks.iter_mut() {
                        let matching_qty = std::cmp::min(p.remaining_qty, incoming_remaining_qty);
                        incoming_remaining_qty -= matching_qty;
                        let filled_data = Fill {
                            order_id: incoming_ord_id.clone(),
                            maker_id: p.order_id.clone(),
                            taker_id: incoming_ord_id.clone(),
                            price: p.price.unwrap(),
                            qty: matching_qty,
                            symbol: incoming_ord_symbol.clone(),
                            time: get_time(),
                        };
                        //Add to the fills array;
                        match fills.get_mut(&incoming_ord_id) {
                            Some(fill) => {
                                fill.push(filled_data);
                            }
                            None => {
                                //I think here i need to create a fresh one and insert it to this.
                                let fills_id =
                                    fills.entry(incoming_ord_id.clone()).or_insert(Vec::new());
                                fills_id.push(filled_data);
                            }
                        };

                        p.filled_qty += matching_qty;
                        p.remaining_qty -= matching_qty;

                        if incoming_remaining_qty == 0 {
                            break;
                        }

                        if (p.remaining_qty == 0) {
                            match fills.get_mut(&p.order_id) {
                                Some(fill) => fill.push(Fill {
                                    order_id: p.order_id.clone(),
                                    maker_id: p.order_id.clone(),
                                    taker_id: incoming_ord_id.clone(),
                                    price: p.price.unwrap(),
                                    qty: matching_qty,
                                    symbol: incoming_ord_symbol.clone(),
                                    time: get_time(),
                                }),
                                None => {
                                    let fills_id =
                                        fills.entry(p.order_id.clone()).or_insert(Vec::new());
                                    fills_id.push(Fill {
                                        order_id: p.order_id.clone(),
                                        maker_id: p.order_id.clone(),
                                        taker_id: incoming_ord_id.clone(),
                                        price: p.price.unwrap(),
                                        qty: matching_qty,
                                        symbol: incoming_ord_symbol.clone(),
                                        time: get_time(),
                                    });
                                }
                            }
                            check_positions(positions, fills, p.order_id.clone(), orders);
                        }
                    }
                    price_orders.asks.retain(|order| order.remaining_qty > 0);
                }
            }
            check_positions(positions, fills, incoming_ord_id.clone(), orders);
            //Add the order in the book;
            let resting_order: RestingOrder = RestingOrder {
                order_id: incoming_ord_id,
                user_id: incoming_order_user_id,
                qty: incmoing_ord_size,
                price: Some(incoming_ord_price),
                filled_qty: incmoing_ord_size - incoming_remaining_qty,
                remaining_qty: incoming_remaining_qty,
                symbol: incoming_ord_symbol,
            };
            add_in_bids(book, resting_order);
        }

        OrderSide::Sell => {}
    }
}

fn handleMarketOrder(
    order_id: String,
    book: &mut HashMap<u64, OrderBook>,
    positions: &mut HashMap<String, Position>,
    orders: &mut HashMap<String, Order>,
    fills: &mut HashMap<String, Vec<Fill>>,
) {
}

fn get_time() -> String {
    let local = Local::now();
    local.to_string()
}

fn add_in_bids(book: &mut HashMap<u64, OrderBook>, resting_order: RestingOrder) {
    let price = resting_order.price.unwrap();
    let order_book = book.entry(price).or_insert(OrderBook {
        asks: Vec::new(),
        bids: Vec::new(),
    });
    order_book.bids.push(resting_order);
    return;
}

fn check_positions(
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


fn risk_engine(positions: &mut HashMap<String, Position>, order : IncomingOrder)-> bool{
    //Check if position exists 
    //If positions exists: 
        //Check if it is increasing the risk -> 
        //If yes -> return True; In this case we have to check collateral, balance and all.
        //If no -> return False; In this case we don't have to check collateral.
    //If no : 
        //Check then simple return false;
    let mut output = false;
    match positions.get(&order.user_id){
        Some(pos) => {
            let risk =  order.size + pos.size;
            if risk.abs() < pos.size.abs() {
                output = false;
            }else{
                output = true;
            }
        }
        None => {
            output = false;
        }
    };
    output
}

