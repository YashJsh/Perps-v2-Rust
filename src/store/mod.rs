use std::collections::HashMap;
use std::sync::Mutex;

use tokio::sync::mpsc::Sender;

use crate::engine::types::EngineRequest;
use crate::utils::user::User;

#[derive(Clone)]
pub enum RequestType {
    CreateOrder,
    UpateMarkPrice,
    CheckBalance,
}

pub enum DataTypes {
    IncomingOrder,
    GetBalance,
}

pub struct AppState {
    pub users: Mutex<HashMap<u64, User>>,
    pub sender: Sender<EngineRequest>,
}
