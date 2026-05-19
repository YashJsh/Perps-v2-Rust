use serde::{Serialize, Deserialize};
use uuid::Uuid;

use crate::engine::types::{OrderSide, OrderType};

#[derive(Serialize, Deserialize)]
pub struct AuthData{
    pub username : String,
    pub password : String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct IncomingOrder{
    pub user_id : String,
    pub order_type : OrderType,
    pub order_side : OrderSide,
    pub symbol : String,
    pub size : u64,
    pub price : Option<u64>,
    pub leverage : u64
}

#[derive(Serialize, Deserialize)]
pub struct GetBalance{
    pub user_id : Uuid,
}

//Below are types that will be used in engine.
pub struct MarkPriceData{
    pub symbol : String,
    pub price : u64
}

pub struct BalanceUpdateData{
    pub user_id : String,
    pub symbol : String
}
