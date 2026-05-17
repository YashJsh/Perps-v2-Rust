use std::sync::mpsc::Sender;
use std::{collections::HashMap, sync::Mutex};
use queues::*;

use crate::utils::user::User;
use crate::types::types::{IncomingOrder, GetBalance};

#[derive(Clone)]
pub enum RequestType{
    CreateOrder,
    CheckBalance
}

pub enum DataTypes{
    IncomingOrder,
    GetBalance
}

#[derive(Clone)]
pub struct EngineRequest{
    pub request_type : RequestType,
    pub data : IncomingOrder
}

pub struct AppState{
    pub users : Mutex<HashMap<String, User>>,
    pub sender : Sender<EngineRequest>
}