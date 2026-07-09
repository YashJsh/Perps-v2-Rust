use std::collections::HashMap;

use tokio::sync::{mpsc::Sender, oneshot};

use crate::{
    engine::types::{DeleteOrderRes, EngineError, Order, OrderBook, OrderSide, OrderStatus},
    types::{BalanceRequest, DeleteOrderData},
};

pub fn delete_order_func(
    data: DeleteOrderData,
    orders: &mut HashMap<String, Order>,
    order_book: &mut OrderBook,
    btc_balance_tx: &Sender<BalanceRequest>,
) -> Result<DeleteOrderRes, EngineError> {
    let (tx, rx) = oneshot::channel();
    let order_id = data.order_id;
    let (price, order_side, status, leverage, remaining_qty) = match orders.get(&order_id) {
        Some(ord) => (
            ord.price,
            ord.order_side.clone(),
            ord.status.clone(),
            ord.leverage,
            ord.remaining_qty,
        ),

        None => {
            return Err(EngineError::OrderNotFound);
        }
    };
    match status {
        OrderStatus::Open | OrderStatus::PartiallyFilled => {
            let positional_price = price * remaining_qty;
            let final_amount_to_reclaim = positional_price / leverage;
            let _ = btc_balance_tx.blocking_send(BalanceRequest::ReleaseMargin {
                user_id: data.user_id,
                amount: final_amount_to_reclaim,
                response_tx: tx,
            });
            let data = rx.blocking_recv();
            match data {
                Ok(d) => match d {
                    Ok(_) => {
                        println!("Balance restored successfully");
                    }
                    Err(e) => return Err(e),
                },
                Err(_) => {
                    return Err(EngineError::BalanceThreadDead);
                }
            };
        }
        _ => return Err(EngineError::OrderFilledAlready),
    }
    let id = order_id.clone();
    if let Some(ord) = orders.get_mut(&order_id) {
        ord.status = OrderStatus::Cancelled;
    }

    let orders_vec = match order_side {
        OrderSide::Buy => &mut order_book.bids,
        OrderSide::Sell => &mut order_book.asks,
    };

    match orders_vec.get_mut(&price) {
        Some(order) => {
            if let Some(pos) = order.iter().position(|p| p.order_id == order_id) {
                order.remove(pos)
            } else {
                return Err(EngineError::OrderNotFound);
            }
        }
        None => {
            return Err(EngineError::OrderNotFound);
        }
    };

    Ok(DeleteOrderRes {
        success: true,
        order_status: status.clone(),
        order_id: id,
        data: String::from("Order deleted Successfully"),
    })
}
