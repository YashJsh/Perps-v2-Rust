use std::{
    env,
    time::{SystemTime, UNIX_EPOCH},
};

use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use jwt::Claims;
use serde::{ Serialize};

#[derive(Serialize)]
pub struct Data {
    user_id: u64,
    sub: usize,
}

pub fn create_token(user_id: u64) -> String {
    let key = env::var("JWT_SECRET").expect("JWT secret missing");
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let data = Data {
        user_id : user_id,
        sub: (now + 3600) as usize,
    };
    let token = encode(
        &Header::default(),
        &data,
        &EncodingKey::from_secret(key.as_ref()),
    )
    .unwrap();
    token
}

pub fn verify_token(token: &str) -> bool {
    let decoding = decode::<Claims>(
        token,
        &DecodingKey::from_secret("secret".as_ref()),
        &Validation::new(Algorithm::HS256),
    );
    match decoding {
        Ok(_) => true,
        Err(_) => false,
    }
}

pub fn decode_token(token: &str) -> Option<Claims> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret("secret".as_ref()),
        &Validation::new(Algorithm::HS256),
    )
    .ok()
    .map(|data| data.claims)
}
