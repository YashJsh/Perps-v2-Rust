use chrono::Local;
use tokio::sync::{mpsc::{self, Sender}, oneshot};
use std::{collections::{HashMap, VecDeque}};
use uuid::Uuid;

use crate::{
    engine::types::{
        CreateOrderResponse, EngineError, Fill, Order, OrderBook, OrderSide, OrderStatus,
        OrderType, Position, RestingOrder,
    },
    types::types::{BalanceRequest, IncomingOrder},
};

pub fn create_order(
    data: IncomingOrder,
    orders: &mut HashMap<String, Order>,
    book: &mut OrderBook,
    positions: &mut HashMap<String, Position>,
    fills: &mut HashMap<String, Vec<Fill>>,
    balance_tx : &Sender<BalanceRequest>
) -> Result<CreateOrderResponse, EngineError> {
    let (tx, rx) = oneshot::channel();
    let order_id = Uuid::new_v4().to_string();

    let incmoing_order_signed_size = match &data.order_side {
        OrderSide::Buy => data.size as i64,
        OrderSide::Sell => -(data.size as i64),
    };

    println!("Checking with risk engine");
    let check = risk_engine(
        &positions,
        incmoing_order_signed_size,
        &data.user_id.clone(),
    );

    if check {
        let _ = balance_tx.blocking_send(BalanceRequest::GetBalance { user_id: data.user_id.clone(), response_tx: tx });
        let balance = rx.blocking_recv();
        let user_balance = match balance{
            Ok(data)=>{
                match data{
                    Ok(d)=>d,
                    Err(e)=> return Err(e),
                }
            },
            Err(_)=> return Err(EngineError::BalanceThreadDead),
        };
        
        let required_margin = check_required_margin(data.size, data.price, data.leverage);
        if user_balance.balance < required_margin{
            return Err(EngineError::NotEnoughBalance)
        }
    }


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
        status: super::types::OrderStatus::Open,
        slippage: data.slippage,
        filled_qty: 0,
        remaining_qty: data.size,
        created_at: get_time(),
    };
    orders.insert(order_id.clone(), new_order);
    println!("Order inserted in orders with order_id : {}", order_id);

    match order_type {
        OrderType::Market => handle_market_order(order_id, book, positions, orders, fills),
        OrderType::Limit => handle_limit_order(order_id, book, positions, orders, fills),
    }
}

//What are the things i need to do for limit order;
fn handle_limit_order(
    order_id: String,
    book: &mut OrderBook,
    positions: &mut HashMap<String, Position>,
    orders: &mut HashMap<String, Order>,
    fills: &mut HashMap<String, Vec<Fill>>,
) -> Result<CreateOrderResponse, EngineError> {
    let (
        incoming_ord_price,
        incmoing_ord_size,
        incoming_ord_symbol,
        incoming_ord_side,
        incoming_order_user_id,
    ) = match orders.get(&order_id) {
        Some(ord) => (
            ord.price,
            ord.size,
            ord.symbol.clone(),
            ord.order_side.clone(),
            ord.user_id.clone(),
        ),
        None => {
            return Err(EngineError::OrderNotFound);
        }
    };
    let incoming_ord_id = order_id.clone();
    match incoming_ord_side {
        OrderSide::Buy => {
            println!("Buy Limit Order");
            {
                fills.entry(order_id).or_insert(Vec::new());
            }
            let mut incoming_remaining_qty = incmoing_ord_size;
            while incoming_remaining_qty > 0 {
                let mut entry = match book.asks.first_entry() {
                    Some(entry) => entry,
                    None => {
                        break;
                    }
                };
                let price = *entry.key();
                let asks: &mut VecDeque<RestingOrder> = entry.get_mut();

                if (price <= incoming_ord_price) {
                    while let Some(front) = asks.front_mut() {
                        let selling_order = front;

                        let matched_qty =
                            std::cmp::min(incoming_remaining_qty, selling_order.remaining_qty);

                        let buyer_fills = fills
                            .entry(incoming_ord_id.clone())
                            .or_insert(Vec::new())
                            .push(Fill {
                                order_id: incoming_ord_id.clone(),
                                maker_id: selling_order.order_id.clone(),
                                taker_id: incoming_ord_id.clone(),
                                price: selling_order.price,
                                qty: matched_qty,
                                symbol: selling_order.symbol.clone(),
                                time: get_time(),
                            });

                        let seller_fills = fills
                            .entry(selling_order.order_id.clone())
                            .or_insert(Vec::new())
                            .push(Fill {
                                order_id: selling_order.order_id.clone(),
                                maker_id: selling_order.order_id.clone(),
                                taker_id: incoming_ord_id.clone(),
                                price: selling_order.price,
                                qty: matched_qty,
                                symbol: selling_order.symbol.clone(),
                                time: get_time(),
                            });

                        let buy_order = match orders.get_mut(&incoming_ord_id) {
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

            if incoming_remaining_qty != incmoing_ord_size {
                check_positions(positions, fills, incoming_ord_id.clone(), orders);
            }
            //Add the order in the book;
            let resting_order: RestingOrder = RestingOrder {
                order_id: incoming_ord_id,
                user_id: incoming_order_user_id,
                qty: incmoing_ord_size as u64,
                price: incoming_ord_price,
                remaining_qty: incoming_remaining_qty,
                symbol: incoming_ord_symbol,
            };
            add_in_bids(book, resting_order);
            let status: OrderStatus;
            if incoming_remaining_qty == incmoing_ord_size {
                status = OrderStatus::Open
            } else if incoming_remaining_qty > 0 {
                status = OrderStatus::PartiallyFilled
            } else {
                status = OrderStatus::Filled
            }
            return Ok(CreateOrderResponse {
                success: true,
                filled_qty: incmoing_ord_size - incoming_remaining_qty,
                remaining_qty: incoming_remaining_qty,
                order_status: status,
            });
        }

        OrderSide::Sell => {
            {
                let fill_vec = fills.entry(order_id).or_insert(Vec::new());
            }

            let mut incoming_remaining_qty = incmoing_ord_size;
            while incoming_remaining_qty > 0 {
                    let mut entry = match book.bids.last_entry() {
                        Some(entry) => entry,
                        None => {
                            break;
                        }
                    };
                    let price = *entry.key();
                    let asks: &mut VecDeque<RestingOrder> = entry.get_mut();

                    if price >= incoming_ord_price {
                        while let Some(front) = asks.front_mut() {
                            let selling_order = front;

                            let matched_qty =
                                std::cmp::min(incoming_remaining_qty, selling_order.remaining_qty);

                            let buyer_fills = fills
                                .entry(incoming_ord_id.clone())
                                .or_insert(Vec::new())
                                .push(Fill {
                                    order_id: incoming_ord_id.clone(),
                                    maker_id: selling_order.order_id.clone(),
                                    taker_id: incoming_ord_id.clone(),
                                    price: selling_order.price,
                                    qty: matched_qty,
                                    symbol: selling_order.symbol.clone(),
                                    time: get_time(),
                                });

                            let seller_fills = fills
                                .entry(selling_order.order_id.clone())
                                .or_insert(Vec::new())
                                .push(Fill {
                                    order_id: selling_order.order_id.clone(),
                                    maker_id: selling_order.order_id.clone(),
                                    taker_id: incoming_ord_id.clone(),
                                    price: selling_order.price,
                                    qty: matched_qty,
                                    symbol: selling_order.symbol.clone(),
                                    time: get_time(),
                                });

                            let buy_order = match orders.get_mut(&incoming_ord_id) {
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

            if incoming_remaining_qty != incmoing_ord_size {
                println!("Checking Positions");
                check_positions(positions, fills, incoming_ord_id.clone(), orders);
            }
            //Add the order in the book;
            let resting_order: RestingOrder = RestingOrder {
                order_id: incoming_ord_id,
                user_id: incoming_order_user_id,
                qty: incmoing_ord_size as u64,
                price: incoming_ord_price,
                remaining_qty: incoming_remaining_qty,
                symbol: incoming_ord_symbol,
            };
            
            add_in_sorts(book, resting_order);
            let status: OrderStatus;
            if incoming_remaining_qty == incmoing_ord_size {
                status = OrderStatus::Open
            } else if incoming_remaining_qty > 0 {
                status = OrderStatus::PartiallyFilled
            } else {
                status = OrderStatus::Filled
            }
            return Ok(CreateOrderResponse {
                success: true,
                filled_qty: incmoing_ord_size - incoming_remaining_qty,
                remaining_qty: incoming_remaining_qty,
                order_status: status,
            });
        }
    }
}

fn get_time() -> String {
    let local = Local::now();
    local.to_string()
}

fn add_in_bids(book: &mut OrderBook, resting_order: RestingOrder) {
    let price = resting_order.price;
    let bid_side = book.bids.entry(price).or_insert(VecDeque::new());
    bid_side.push_back(resting_order);
    println!("Added in orderbook ");
    println!("Bid Side Book looks like : {:?}", bid_side);
    return;
}

fn add_in_sorts(book: &mut OrderBook, resting_order: RestingOrder) {
    let price = resting_order.price;
    let ask_side = book.asks.entry(price).or_insert(VecDeque::new());
    ask_side.push_back(resting_order);
    println!("Added in orderbook ");
    println!("Ask side looks like : {:?}", ask_side);
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

fn check_required_margin(size : u64, price : u64, leverage : u64)-> u64{
    let notional_value = size * price;
    let amount_needed = notional_value / leverage;
    amount_needed
}

fn handle_market_order(
    order_id: String,
    book: &mut OrderBook,
    positions: &mut HashMap<String, Position>,
    orders: &mut HashMap<String, Order>,
    fills: &mut HashMap<String, Vec<Fill>>,
) -> Result<CreateOrderResponse, EngineError> {
    let (incoming_qty, incoming_side, incoming_leverage, incoming_user_id, incoming_price) =
        match orders.get(&order_id) {
            Some(ord) => (
                ord.size,
                ord.order_side.clone(),
                ord.leverage,
                ord.user_id.clone(),
                ord.price,
            ),
            None => {
                println!("Order not found");
                return Err(EngineError::OrderNotFound);
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

    let mut incoming_remaining_qty = incoming_qty;
    match incoming_side {
        OrderSide::Buy => {
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

            //Sort the book prices first;
            check_positions(positions, fills, incoming_order_id, orders);

            let status: OrderStatus;
            if incoming_remaining_qty == incoming_qty {
                status = OrderStatus::Open
            } else if incoming_remaining_qty > 0 {
                status = OrderStatus::PartiallyFilled
            } else {
                status = OrderStatus::Filled
            }
            return Ok(CreateOrderResponse {
                success: true,
                filled_qty: incoming_qty - incoming_remaining_qty,
                remaining_qty: incoming_remaining_qty,
                order_status: status,
            });
        }

        OrderSide::Sell => {
            {
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

                //Sort the book prices first;
                check_positions(positions, fills, incoming_order_id, orders);

                let status: OrderStatus;
                if incoming_remaining_qty == incoming_qty {
                    status = OrderStatus::Open
                } else if incoming_remaining_qty > 0 {
                    status = OrderStatus::PartiallyFilled
                } else {
                    status = OrderStatus::Filled
                }
                return Ok(CreateOrderResponse {
                    success: true,
                    filled_qty: incoming_qty - incoming_remaining_qty,
                    remaining_qty: incoming_remaining_qty,
                    order_status: status,
                });
            }
        }
    }
}
