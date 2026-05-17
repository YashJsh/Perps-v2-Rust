use std::{collections::HashMap, sync::Mutex};

use crate::utils::user::User;

pub struct AppState{
    pub users : Mutex<HashMap<String, User>>
}