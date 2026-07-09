use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::{mpsc::Sender, oneshot};

use crate::{
    engine::{
        core_matching::{core_buy_logic, core_sell_logic},
        helper::{add_in_bids, add_in_sorts, check_required_margin, get_time, risk_engine},
        position::check_positions,
        types::{
            CreateOrderResponse, EngineError, Fill, Order, OrderBook, OrderSide, OrderStatus,
            OrderType, Position, RestingOrder,
        },
    },
    types::{BalanceRequest, IncomingOrder},
};

static NEXT_ORDER_ID: AtomicU64 = AtomicU64::new(1);

pub fn create_order(
    data: IncomingOrder,
    orders: &mut HashMap<u64, Order>,
    book: &mut OrderBook,
    positions: &mut HashMap<u64, Position>,
    fills: &mut HashMap<u64, Vec<Fill>>,
    balance_tx: &Sender<BalanceRequest>,
) -> Result<CreateOrderResponse, EngineError> {
    let (tx, rx) = oneshot::channel();
    let (tx2, rx2) = oneshot::channel();
    let order_id = NEXT_ORDER_ID.fetch_add(1, Ordering::Relaxed);

    let incmoing_order_signed_size = match &data.order_side {
        OrderSide::Buy => data.size as i64,
        OrderSide::Sell => -(data.size as i64),
    };

    println!("Checking with risk engine");
    let check = risk_engine(
        &positions,
        incmoing_order_signed_size,
        data.user_id,
    );

    if check {
        let _ = balance_tx.blocking_send(BalanceRequest::GetBalance {
            user_id: data.user_id,
            response_tx: tx,
        });
        let balance = rx.blocking_recv();
        let user_balance = match balance {
            Ok(data) => match data {
                Ok(d) => d,
                Err(e) => return Err(e),
            },
            Err(_) => return Err(EngineError::BalanceThreadDead),
        };

        let required_margin = check_required_margin(data.size, data.price, data.leverage);
        if user_balance.balance < required_margin {
            return Err(EngineError::NotEnoughBalance);
        }

        //Lock the margin
        let _ = balance_tx.blocking_send(BalanceRequest::LockMargin {
            user_id: data.user_id,
            amount: required_margin,
            response_tx: tx2,
        });
        let _ = rx2
            .blocking_recv()
            .map(|d| d.map(|r| println!("Margin Locked Successfully {:?}", r)));
    }

    let order_type = data.order_type.clone();
    let new_order = Order {
        user_id: data.user_id,
        order_id,
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
    orders.insert(order_id, new_order);
    println!("Order inserted in orders with order_id : {}", order_id);

    match order_type {
        OrderType::Market => {
            handle_market_order(order_id, book, positions, orders, fills, balance_tx)
        }
        OrderType::Limit => {
            handle_limit_order(order_id, book, positions, orders, fills, balance_tx)
        }
    }
}

//What are the things i need to do for limit order;
fn handle_limit_order(
    order_id: u64,
    book: &mut OrderBook,
    positions: &mut HashMap<u64, Position>,
    orders: &mut HashMap<u64, Order>,
    fills: &mut HashMap<u64, Vec<Fill>>,
    balance_tx: &Sender<BalanceRequest>,
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
            ord.user_id,
        ),
        None => {
            return Err(EngineError::OrderNotFound);
        }
    };
    let incoming_ord_id = order_id;
    match incoming_ord_side {
        OrderSide::Buy => {
            let incoming_remaining_qty = match core_buy_logic(
                incmoing_ord_size,
                incoming_ord_price,
                book,
                orders,
                fills,
                order_id,
            ) {
                Ok(e) => e,
                Err(err) => return Err(err),
            };

            if incoming_remaining_qty != incmoing_ord_size {
                check_positions(
                    positions,
                    fills,
                    incoming_ord_id,
                    orders,
                    balance_tx,
                );
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
            if resting_order.remaining_qty != 0 {
                add_in_bids(book, resting_order);
            }
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
            let incoming_remaining_qty = match core_sell_logic(
                incmoing_ord_size,
                incoming_ord_price,
                book,
                orders,
                fills,
                order_id,
            ) {
                Ok(e) => e,
                Err(err) => return Err(err),
            };

            if incoming_remaining_qty != incmoing_ord_size {
                println!("Checking Positions");
                check_positions(
                    positions,
                    fills,
                    incoming_ord_id,
                    orders,
                    balance_tx,
                );
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
            if resting_order.remaining_qty != 0 {
                add_in_sorts(book, resting_order);
            }
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

fn handle_market_order(
    order_id: u64,
    book: &mut OrderBook,
    positions: &mut HashMap<u64, Position>,
    orders: &mut HashMap<u64, Order>,
    fills: &mut HashMap<u64, Vec<Fill>>,
    balance_tx: &Sender<BalanceRequest>,
) -> Result<CreateOrderResponse, EngineError> {
    let (incoming_qty, incoming_side, _incoming_leverage, incoming_user_id, incoming_price) =
        match orders.get(&order_id) {
            Some(ord) => (
                ord.size,
                ord.order_side.clone(),
                ord.leverage,
                ord.user_id,
                ord.price,
            ),
            None => {
                println!("Order not found");
                return Err(EngineError::OrderNotFound);
            }
        };

    let incoming_order_id = order_id;

    let incoming_order_signed_size = match &incoming_side {
        OrderSide::Buy => incoming_qty as i64,
        OrderSide::Sell => -(incoming_qty as i64),
    };

    let check = risk_engine(positions, incoming_order_signed_size, incoming_user_id);

    //Sort the book first;
    if check {
        //Check the balances;
    }

    match incoming_side {
        OrderSide::Buy => {
            let incoming_remaining_qty =
                match core_buy_logic(incoming_qty, incoming_price, book, orders, fills, order_id) {
                    Ok(d) => d,
                    Err(e) => return Err(e),
                };
            //Sort the book prices first;
            check_positions(positions, fills, incoming_order_id, orders, balance_tx);

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
                let incoming_remaining_qty = match core_sell_logic(
                    incoming_qty,
                    incoming_price,
                    book,
                    orders,
                    fills,
                    order_id,
                ) {
                    Ok(d) => d,
                    Err(e) => return Err(e),
                };

                //Sort the book prices first;
                check_positions(positions, fills, incoming_order_id, orders, balance_tx);

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
