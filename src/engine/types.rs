use serde::{Deserialize, Serialize};
use tokio::sync::oneshot::Sender;

use crate::types::types::{BalanceUpdateData, DeleteOrderData, IncomingOrder, MarkPriceData};

#[derive(serde::Serialize, Deserialize, Clone)]
pub enum OrderType {
    Market,
    Limit,
}

#[derive(serde::Serialize, Deserialize, Clone)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(serde::Serialize, Deserialize, Clone)]
pub enum OrderStatus {
    Filled,
    PartiallyFilled,
    Cancelled,
    Open,
}

pub struct Order {
    pub user_id: String,
    pub order_id: String,
    pub order_type: OrderType,
    pub order_side: OrderSide,
    pub symbol: String,
    pub size: u64,
    pub price: Option<u64>,
    pub leverage: u64,
    pub status: OrderStatus,
}

#[derive(Clone)]
pub struct Position {
    // pub user_id : String,
    pub order_id: String,
    pub average_entry_price: u64, //average of all entry prices.
    pub symbol: String,
    pub margin: u64,
    pub size: i64,
    pub liquidation_price: u64,
    pub realized_pnl: Option<u64>,
    pub time: String,
    pub leverage: u64,
}

#[derive(Clone)]
pub struct Fill {
    pub order_id: String,
    pub maker_id: String,
    pub taker_id: String,
    pub price: u64,
    pub qty: u64,
    pub symbol: String,
    pub time: String,
}

#[derive(Debug)]
pub struct RestingOrder {
    pub order_id: String,
    pub user_id: String,
    pub qty: u64,
    pub price: Option<u64>,
    pub filled_qty: u64,
    pub remaining_qty: u64,
    pub symbol: String,
}

#[derive(Debug)]
pub struct OrderBook {
    pub asks: Vec<RestingOrder>,
    pub bids: Vec<RestingOrder>,
}

pub enum EngineRequest {
    CreateOrder {
        order: IncomingOrder,
        response_tx: Sender<Result<CreateOrderResponse, EngineError>>,
    },
    MarkPriceUpdate {
        data: MarkPriceData,
    },
    CheckBalance(BalanceUpdateData),
    DeleteOrderData {
        data: DeleteOrderData,
        response_tx: Sender<Result<DeleteOrderRes,EngineError>>,
    },
}

#[derive(serde::Serialize)]
pub struct DeleteOrderRes {
    pub success : bool,
    pub order_status: OrderStatus,
    pub data : String,
    pub order_id : String
}

#[derive(serde::Serialize)]
pub struct CreateOrderResponse{
    pub success : bool,
    pub filled_qty : u64,
    pub remaining_qty : u64,
    pub order_status : OrderStatus
}

#[derive(Serialize)]
pub enum EngineError{
    InvalidPrice,
    InsufficientBalance,
    OrderNotFound,
    OrderFilledAlready,
    OrderBookNotFound
}