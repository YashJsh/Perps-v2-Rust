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
        status : super::types::OrderStatus::Open
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

    let incmoing_order_signed_size = match &incoming_ord_side {
        OrderSide::Buy => incmoing_ord_size as i64,
        OrderSide::Sell => -(incmoing_ord_size as i64),
    };

    //Here risk engine will run. And according to the bool result
    //It will check or not check the collateral, balance for the new order.
    let check = risk_engine(
        &positions,
        incmoing_order_signed_size,
        &incoming_order_user_id,
    );

    if check {
        //Check balances margin collateral and all here.
    }
    match incoming_ord_side {
        OrderSide::Buy => {
            {
                fills.entry(order_id).or_insert(Vec::new());
            }

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
                                let fills_id = fills.entry(incoming_ord_id.clone()).or_default();
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
                                        fills.entry(incoming_ord_id.clone()).or_default();
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
            if incoming_remaining_qty != incmoing_ord_size{
                check_positions(positions, fills, incoming_ord_id.clone(), orders);
            }
            //Add the order in the book;
            let resting_order: RestingOrder = RestingOrder {
                order_id: incoming_ord_id,
                user_id: incoming_order_user_id,
                qty: incmoing_ord_size as u64,
                price: Some(incoming_ord_price),
                filled_qty: incmoing_ord_size - incoming_remaining_qty,
                remaining_qty: incoming_remaining_qty,
                symbol: incoming_ord_symbol,
            };
            add_in_bids(book, resting_order);
        }

        OrderSide::Sell => {
            {
                let fill_vec = fills.entry(order_id).or_insert(Vec::new());
            }

            let mut prices: Vec<u64> = book.keys().copied().collect();
            prices.sort_by(|a, b| b.cmp(a));

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
                if &price >= &incoming_ord_price {
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
                                let fills_id = fills.entry(incoming_ord_id.clone()).or_default();
                                fills_id.push(filled_data);
                            }
                        };

                        p.filled_qty += matching_qty;
                        p.remaining_qty -= matching_qty;

                        if incoming_remaining_qty == 0 {
                            break;
                        }

                        if (matching_qty > 0) {
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
                                        fills.entry(incoming_ord_id.clone()).or_default();
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
            
            if incoming_remaining_qty != incmoing_ord_size{
                println!("Checking Positions");
                check_positions(positions, fills, incoming_ord_id.clone(), orders);
            }
            //Add the order in the book;
            let resting_order: RestingOrder = RestingOrder {
                order_id: incoming_ord_id,
                user_id: incoming_order_user_id,
                qty: incmoing_ord_size as u64,
                price: Some(incoming_ord_price),
                filled_qty: incmoing_ord_size - incoming_remaining_qty,
                remaining_qty: incoming_remaining_qty,
                symbol: incoming_ord_symbol,
            };
            add_in_sorts(book, resting_order);
        }
    }
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
    println!("Added in orderbook ");
    println!("Book looks like : {:?}", order_book);
    return;
}

fn add_in_sorts(book: &mut HashMap<u64, OrderBook>, resting_order: RestingOrder) {
    let price = resting_order.price.unwrap();
    let orderbook = book.entry(price).or_insert(OrderBook {
        asks: Vec::new(),
        bids: Vec::new(),
    });
    orderbook.asks.push(resting_order);
    println!("Added in orderbook ");
    println!("Book looks like : {:?}", orderbook);
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

fn risk_engine(positions: &HashMap<String, Position>, order_size: i64, user_id: &String) -> bool {
    //Check if position exists
    //If positions exists:
    //Check if it is increasing the risk ->
    //If yes -> return True; In this case we have to check collateral, balance and all.
    //If no -> return False; In this case we don't have to check collateral.
    //If no :
    //Check then simple return false;
    let mut output = false;
    match positions.get(user_id) {
        Some(pos) => {
            let risk = order_size + pos.size;
            if risk.abs() < pos.size.abs() {
                output = false;
            } else {
                output = true;
            }
        }
        None => {
            output = false;
        }
    };
    output
}

fn handleMarketOrder(
    order_id: String,
    book: &mut HashMap<u64, OrderBook>,
    positions: &mut HashMap<String, Position>,
    orders: &mut HashMap<String, Order>,
    fills: &mut HashMap<String, Vec<Fill>>,
) {
    let (incoming_qty, incoming_side, incoming_leverage, incoming_user_id) =
        match orders.get(&order_id) {
            Some(ord) => (
                ord.size,
                ord.order_side.clone(),
                ord.leverage,
                ord.user_id.clone(),
            ),
            None => {
                println!("Order not found");
                return;
            }
        };

    let incoming_order_id = order_id.clone();

    let incoming_order_signed_size = match &incoming_side {
        OrderSide::Buy => incoming_qty as i64,
        OrderSide::Sell => -(incoming_qty as i64),
    };

    let check = risk_engine(positions, incoming_order_signed_size, &incoming_user_id);

    //Sort the book first;
    if check {
        //Check the balances;
    }

    match incoming_side {

        OrderSide::Buy => {
            //Sort the book prices first;
            let mut prices: Vec<u64> = book.keys().copied().collect();
            prices.sort();

            {
                fills.entry(order_id).or_insert(Vec::new());
            }

            let mut incoming_remaining_qty = incoming_qty;
            for i in 0..prices.len() {
                let price = prices[i];

                let price_orders = match book.get_mut(&price) {
                    Some(ask) => ask,
                    None => {
                        continue;
                    }
                };

                if incoming_remaining_qty == 0 {
                    println!("Order is matched fully nothing is left to match");
                    return;
                }

                for selling_order in price_orders.asks.iter_mut() {
                    //Sellable order is :
                    let matching_qty =
                        std::cmp::min(incoming_remaining_qty, selling_order.remaining_qty);
                    let filled_data = Fill {
                        order_id: incoming_order_id.clone(),
                        maker_id: selling_order.order_id.clone(),
                        taker_id: incoming_order_id.clone(),
                        price: selling_order.price.unwrap(),
                        qty: matching_qty,
                        symbol: selling_order.symbol.clone(),
                        time: get_time(),
                    };

                    //Push to fills :
                    match fills.get_mut(&incoming_order_id) {
                        Some(fill) => {
                            fill.push(filled_data);
                        }
                        None => {
                            //I think here i need to create a fresh one and insert it to this.
                            let fills_id = fills.entry(incoming_order_id.clone()).or_default();
                            fills_id.push(filled_data);
                        }
                    };

                    //Remove the current selling order remainign qty;
                    selling_order.remaining_qty -= matching_qty;
                    selling_order.filled_qty += matching_qty;

                    //Incoming order remaining qty is :
                    if incoming_remaining_qty == 0 {
                        break;
                    }

                    if selling_order.remaining_qty == 0 {
                        //Push to the fills array
                        match fills.get_mut(&selling_order.order_id) {
                            Some(fill) => fill.push(Fill {
                                order_id: selling_order.order_id.clone(),
                                maker_id: selling_order.order_id.clone(),
                                taker_id: incoming_order_id.clone(),
                                price: selling_order.price.unwrap(),
                                qty: matching_qty,
                                symbol: selling_order.symbol.clone(),
                                time: get_time(),
                            }),
                            None => {
                                let fills_id = fills.entry(incoming_order_id.clone()).or_default();
                                fills_id.push(Fill {
                                    order_id: selling_order.order_id.clone(),
                                    maker_id: selling_order.order_id.clone(),
                                    taker_id: incoming_order_id.clone(),
                                    price: selling_order.price.unwrap(),
                                    qty: matching_qty,
                                    symbol: selling_order.symbol.clone(),
                                    time: get_time(),
                                });
                            }
                        }
                        check_positions(positions, fills, selling_order.order_id.clone(), orders);
                    }

                }
                price_orders.asks.retain(|s| s.remaining_qty > 0);
            }
            check_positions(positions, fills, incoming_order_id, orders);
        }

        OrderSide::Sell => {
            let mut prices: Vec<u64> = book.keys().copied().collect();
            prices.sort_by(|a,b| a.cmp(b));

            {
                fills.entry(order_id).or_insert(Vec::new());
            }

            let mut incoming_remaining_qty = incoming_qty;
            for i in 0..prices.len() {
                let price = prices[i];

                let price_orders = match book.get_mut(&price) {
                    Some(ask) => ask,
                    None => {
                        continue;
                    }
                };

                if incoming_remaining_qty == 0 {
                    println!("Order is matched fully nothing is left to match");
                    return;
                }

                for buying_order in price_orders.asks.iter_mut() {
                    //Sellable order is :
                    let matching_qty =
                        std::cmp::min(incoming_remaining_qty, buying_order.remaining_qty);
                    let filled_data = Fill {
                        order_id: incoming_order_id.clone(),
                        maker_id: buying_order.order_id.clone(),
                        taker_id: incoming_order_id.clone(),
                        price: buying_order.price.unwrap(),
                        qty: matching_qty,
                        symbol: buying_order.symbol.clone(),
                        time: get_time(),
                    };

                    //Push to fills :
                    match fills.get_mut(&incoming_order_id) {
                        Some(fill) => {
                            fill.push(filled_data);
                        }
                        None => {
                            //I think here i need to create a fresh one and insert it to this.
                            let fills_id = fills.entry(incoming_order_id.clone()).or_default();
                            fills_id.push(filled_data);
                        }
                    };

                    //Remove the current selling order remainign qty;
                    buying_order.remaining_qty -= matching_qty;
                    buying_order.filled_qty += matching_qty;

                    //Incoming order remaining qty is :
                    if incoming_remaining_qty == 0 {
                        break;
                    }

                    if buying_order.remaining_qty == 0 {
                        //Push to the fills array
                        match fills.get_mut(&buying_order.order_id) {
                            Some(fill) => fill.push(Fill {
                                order_id: buying_order.order_id.clone(),
                                maker_id: buying_order.order_id.clone(),
                                taker_id: incoming_order_id.clone(),
                                price: buying_order.price.unwrap(),
                                qty: matching_qty,
                                symbol: buying_order.symbol.clone(),
                                time: get_time(),
                            }),
                            None => {
                                let fills_id = fills.entry(incoming_order_id.clone()).or_default();
                                fills_id.push(Fill {
                                    order_id: buying_order.order_id.clone(),
                                    maker_id: buying_order.order_id.clone(),
                                    taker_id: incoming_order_id.clone(),
                                    price: buying_order.price.unwrap(),
                                    qty: matching_qty,
                                    symbol: buying_order.symbol.clone(),
                                    time: get_time(),
                                });
                            }
                        }
                        check_positions(positions, fills, buying_order.order_id.clone(), orders);
                    }

                }
                price_orders.asks.retain(|s| s.remaining_qty > 0);
            }
            check_positions(positions, fills, incoming_order_id, orders);
        }
    }
}
