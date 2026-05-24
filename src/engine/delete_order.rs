use std::collections::HashMap;

use crate::{
    engine::types::{DeleteOrderRes, EngineError, Order, OrderBook, OrderSide, OrderStatus},
    types::types::DeleteOrderData,
};

pub fn delete_order_func(
    data: DeleteOrderData,
    orders: &mut HashMap<String, Order>,
    order_book: &mut HashMap<u64, OrderBook>,
) -> Result<DeleteOrderRes, EngineError> {
    let order_id = data.order_id;
    let (price, order_side, status) = match orders.get(&order_id) {
        Some(ord) => (
            ord.price.unwrap(),
            ord.order_side.clone(),
            ord.status.clone(),
        ),

        None => {
            return Err(EngineError::OrderNotFound);
        }
    };
    match status {
        OrderStatus::Open | OrderStatus::PartiallyFilled => {}
        _ => return Err(EngineError::OrderFilledAlready),
    }

    let book = match order_book.get_mut(&price) {
        Some(book) => book,
        None => return Err(EngineError::OrderBookNotFound),
    };
    let orders_vec = match order_side {
        OrderSide::Buy => &mut book.bids,
        OrderSide::Sell => &mut book.asks,
    };

    if let Some(pos) = orders_vec
        .iter()
        .position(|o| o.order_id == order_id && o.order_id == data.user_id)
    {
        orders_vec.remove(pos);
    } else {
        return Err(EngineError::OrderNotFound);
    }
    let id = order_id.clone();
    if let Some(ord) = orders.get_mut(&order_id) {
        ord.status = OrderStatus::Cancelled;
    }

    Ok(DeleteOrderRes {
        success: true,
        order_status: status.clone(),
        order_id: id,
        data: String::from("Order deleted Successfully"),
    })
}
