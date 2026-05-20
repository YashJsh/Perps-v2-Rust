use std::{collections::HashMap, sync::Mutex};
use tokio::sync::mpsc::Sender;

use crate::engine::types::{ EngineRequest};
use crate::utils::user::User;
use crate::types::types::{Balances, GetBalance, IncomingOrder};

#[derive(Clone)]
pub enum RequestType{
    CreateOrder,
    UpateMarkPrice,
    CheckBalance
}

pub enum DataTypes{
    IncomingOrder,
    GetBalance
}


pub struct AppState{
    pub users : Mutex<HashMap<String, User>>,
    pub balances : Mutex<HashMap<String, Balances>>,
    pub sender : Sender<EngineRequest>,
}