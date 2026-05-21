use std::collections::HashMap;

use crate::{
    engine::types::{DeleteOrderRes, EngineResponse, Order, OrderBook, OrderSide, OrderStatus},
    types::types::DeleteOrderData,
};

pub fn delete_order_func(
    data: DeleteOrderData,
    orders: &mut HashMap<String, Order>,
    order_book: &mut HashMap<u64, OrderBook>,
) -> EngineResponse {
    let order_id = data.order_id;
    let ord = match orders.get(&order_id) {
        Some(ord) => ord,
        None => {
            return EngineResponse::DeleteOrderResponse(DeleteOrderRes {
                status: false,
                error: Some("Order not found".to_string()),
            });
        }
    };
    match ord.status {
        OrderStatus::Open | OrderStatus::PartiallyFilled => {}
        _ => {
            return EngineResponse::DeleteOrderResponse(DeleteOrderRes {
                status: false,
                error: Some("Order already filled or cancelled".to_string()),
            });
        }
    }

    let price = match ord.price {
        Some(p) => p,
        None => {
            return EngineResponse::DeleteOrderResponse(DeleteOrderRes {
                status: false,
                error: Some("Order has no price".to_string()),
            });
        }
    };

    let book = match order_book.get_mut(&price) {
        Some(book) => book,
        None => {
            return EngineResponse::DeleteOrderResponse(DeleteOrderRes {
                status: false,
                error: Some("Book not found".to_string()),
            });
        }
    };
    let orders_vec = match ord.order_side {
        OrderSide::Buy => &mut book.bids,
        OrderSide::Sell => &mut book.asks,
    };

    if let Some(pos) = orders_vec.iter().position(|o| o.order_id == ord.order_id && o.order_id == data.user_id) {
        orders_vec.remove(pos);
    } else {
        return EngineResponse::DeleteOrderResponse(DeleteOrderRes {
            status: false,
            error: Some("Order not found in book".to_string()),
        });
    }

    if let Some(ord) = orders.get_mut(&order_id) {
        ord.status = OrderStatus::Cancelled;
    }

    EngineResponse::DeleteOrderResponse(DeleteOrderRes {
        status: true,
        error: None,
    })

}
