use std::{cmp::min, collections::HashMap};

use tokio::sync::{mpsc::Sender, oneshot};

use crate::{
    engine::{
        helper::get_time,
        types::{Fill, Order, OrderSide, Position},
    },
    types::BalanceRequest,
};

pub fn check_positions(
    positions: &mut HashMap<u64, Position>,
    fills: &mut HashMap<u64, Vec<Fill>>,
    order_id: u64,
    orders: &mut HashMap<u64, Order>,
    balance_tx: &Sender<BalanceRequest>,
) {
    let (tx, rx) = oneshot::channel();
    let (tx2, rx2) = oneshot::channel();

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
    let user_id = order.user_id;
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

    let signed_total_qty;
    match &order.order_side {
        OrderSide::Buy => signed_total_qty = total_qty as i64,
        OrderSide::Sell => signed_total_qty = -(total_qty as i64),
    };

    let position_side;
    if position.size > 0 {
        position_side = OrderSide::Buy;
    } else {
        position_side = OrderSide::Sell;
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
            OrderSide::Buy => new_average_entry - new_average_entry / order.leverage,
            OrderSide::Sell => new_average_entry + new_average_entry / order.leverage,
        };
        position.liquidation_price = liquidation_price;
        position.time = get_time();
    } else {
        //No same side order
        //Here there are there cases :
        // 1. Partial Reduction.
        // 2. Full Close
        // 3. Flip position

        //Determin closed quantity :
        let closed_qty = min(position.size.abs() as u64, total_qty);

        let average_exit_price = notional_value / total_qty;
        //Calculate pnl
        let _pnl = match position_side {
            OrderSide::Buy => {
                (average_exit_price as i64 - position.average_entry_price as i64)
                    * closed_qty as i64
            }
            OrderSide::Sell => {
                (position.average_entry_price as i64 - average_exit_price as i64)
                    * position.average_entry_price as i64
            }
        };

        let new_position = position.size + signed_total_qty;

        if new_position == 0 {
            //Fully close the position
            let pnl = match position_side {
                OrderSide::Buy => {
                    (average_exit_price as i64 - position.average_entry_price as i64)
                        * closed_qty as i64
                }
                OrderSide::Sell => {
                    (position.average_entry_price as i64 - average_exit_price as i64)
                        * position.average_entry_price as i64
                }
            };

            //Add it to wallet balance;
            if pnl > 0 {
                let _ = balance_tx.send(BalanceRequest::AddBalance {
                    user_id,
                    amount: pnl as u64,
                    response_tx: tx,
                });
                let res = rx.blocking_recv();
                match res {
                    Ok(d) => match d {
                        Ok(_r) => {
                            println!("Balance added successfully");
                        }
                        Err(_e) => {
                            println!("Error");
                        }
                    },
                    Err(_) => {
                        println!("Error in balance thread");
                    }
                }

                let _ = balance_tx.send(BalanceRequest::ReleaseMargin {
                    user_id,
                    amount: position.margin,
                    response_tx: tx2,
                });
                let res = rx2.blocking_recv();
                match res {
                    Ok(d) => match d {
                        Ok(_b) => {
                            println!("Margin released");
                        }
                        Err(_e) => {
                            println!("Error");
                        }
                    },
                    Err(_e) => {
                        println!("Error in balance thread");
                    }
                }
                positions.remove(&user_id);
            } else {
                let _ = balance_tx.send(BalanceRequest::ReduceBalance {
                    user_id,
                    amount: pnl as u64,
                    response_tx: tx,
                });
                let res = rx.blocking_recv();
                match res {
                    Ok(d) => match d {
                        Ok(_r) => {
                            println!("Balance removed successfully");
                        }
                        Err(_e) => {
                            println!("Error");
                        }
                    },
                    Err(_) => {
                        println!("Error in balance thread");
                    }
                }
            }
        } else if new_position.signum() == position.size.signum() {
        
            let new_position_size = position.size + signed_total_qty;
            //PNL
            let _ = match position_side {
               OrderSide::Buy => {
                    (average_exit_price as i64 - position.average_entry_price as i64)
                        * closed_qty as i64
                }
                OrderSide::Sell => {
                    (position.average_entry_price as i64 - average_exit_price as i64)
                        * position.average_entry_price as i64
                }
            };
            let remaining_notional = position.average_entry_price * new_position_size.abs() as u64;
            let new_margin = remaining_notional / order.leverage;
            position.margin = new_margin;

            let pos_side;
            if new_position_size > 0 {
                pos_side = OrderSide::Buy
            } else {
                pos_side = OrderSide::Sell
            }

            let liquidation_price = match pos_side {
                OrderSide::Buy => {
                    position.average_entry_price - position.average_entry_price / order.leverage
                }
                OrderSide::Sell => {
                    position.average_entry_price + position.average_entry_price / order.leverage
                }
            };
            position.size = new_position_size;
            position.liquidation_price = liquidation_price;
            position.time = get_time();
        } else {
            //Hard case
            // Here i need to flip the position.
            let _ = balance_tx.blocking_send(BalanceRequest::ReleaseMargin {
                user_id,
                amount: position.margin,
                response_tx: tx,
            });
            let res = rx.blocking_recv();
            match res {
                Ok(d) => match d {
                    Ok(_) => {
                        println!("Margin removed successfully");
                    }
                    Err(_) => {
                        println!("Error");
                    }
                },
                Err(_) => {
                    println!("Error in balance thread");
                }
            };

            position.average_entry_price = average_exit_price;
            position.size = position.size + signed_total_qty;
            position.liquidation_price = match order.order_side{
                OrderSide::Buy => average_exit_price - average_exit_price / order.leverage,
                OrderSide::Sell => average_exit_price + average_exit_price / order.leverage
            };
            position.margin = position.average_entry_price * position.size.abs() as u64 / order.leverage;
            position.time = get_time();
            let _ = balance_tx.blocking_send(BalanceRequest::LockMargin { user_id, amount: position.margin, response_tx: tx2 });
            let res = rx2.blocking_recv();
            match res {
                Ok(d) => match d {
                    Ok(_) => {
                        println!("New Margin Added Successfully");
                    }
                    Err(_) => {
                        println!("Error");
                    }
                },
                Err(_) => {
                    println!("Error in balance thread");
                }
            };
        }
    }
}

fn create_fresh_position(
    positions: &mut HashMap<u64, Position>,
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
        order.user_id,
        Position {
            order_id: order.order_id,
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
