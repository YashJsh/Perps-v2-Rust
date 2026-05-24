use std::collections::HashMap;

use crate::{
    engine::types::{DeleteOrderRes, EngineError, Order, OrderBook, OrderSide, OrderStatus},
    types::types::DeleteOrderData,
};

pub fn delete_order_func(
    data: DeleteOrderData,
    orders: &mut HashMap<String, Order>,
    order_book: &mut OrderBook,
) -> Result<DeleteOrderRes, EngineError> {
    let order_id = data.order_id;
    let (price, order_side, status) = match orders.get(&order_id) {
        Some(ord) => (
            ord.price,
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


    let orders_vec = match order_side {
        OrderSide::Buy => &mut order_book.bids,
        OrderSide::Sell => &mut order_book.asks,
    };

    match orders_vec.get_mut(&price){
        Some(order) => {
            for i in 0..order.len(){      
                if order[i].order_id == order_id{
                    //Remove from the queue;
                    order.remove(i);
                }
            }
        }
        None => {
            return Err(EngineError::OrderNotFound);
        }
    };
    
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
