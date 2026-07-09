use serde::{Deserialize, Serialize};
use serde_json::Number;
use tokio::sync::oneshot::{self, Sender};
use uuid::Uuid;

use crate::engine::types::{BalanceResponse, EngineError, OrderSide, OrderType};

#[derive(Serialize, Deserialize)]
pub struct AuthData {
    pub user_id: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct IncomingOrder {
    pub user_id: String,
    pub order_type: OrderType,
    pub order_side: OrderSide,
    pub symbol: String,
    pub size: u64,
    pub price: u64,
    pub leverage: u64,
    pub slippage : u64
}

#[derive(Serialize, Deserialize, Clone)]
pub struct OnRamp {
    pub user_id: String,
    pub amount: u64,
}

#[derive(Serialize, Deserialize)]
pub struct GetBalance {
    pub user_id: Uuid,
}

#[derive(Serialize, Deserialize)]
pub struct DeleteOrderData {
    pub order_id: String,
    pub user_id: String,
    pub symbol: String,
}

//Below are types that will be used in engine.
pub struct MarkPriceData {
    pub symbol: String,
    pub price: u64,
}

pub struct BalanceUpdateData {
    pub user_id: String,
    pub symbol: String,
}

#[derive(serde::Serialize, Deserialize, Clone)]
pub struct Balances {
    pub available: u64,
    pub locked: u64,
    pub user_id : String
}

#[derive(Serialize)]
pub struct LogInResponse {
    pub success: bool,
    pub token: String,
    pub user_id: String,
}

#[derive(Serialize, Deserialize)]
pub struct GetDepth{
    pub symbol : String
}


pub enum BalanceRequest{
    AddBalance{
        user_id : String,
        amount : u64,
        response_tx : oneshot::Sender<Result<BalanceResponse, EngineError>>
    },
    LockMargin{
        user_id : String,
        amount : u64,
        response_tx : oneshot::Sender<Result<BalanceResponse, EngineError>>
    },
    ReleaseMargin{
        user_id : String,
        amount : u64,
        response_tx : oneshot::Sender<Result<BalanceResponse, EngineError>>
    },
    GetBalance{
        user_id : String,
        response_tx : oneshot::Sender<Result<BalanceResponse, EngineError>>
    },
    ReduceBalance{
        user_id : String,
        amount : u64,
        response_tx : oneshot::Sender<Result<BalanceResponse, EngineError>>
    }
}


