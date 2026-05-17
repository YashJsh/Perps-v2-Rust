use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct AuthData{
    pub username : String,
    pub password : String,
}
