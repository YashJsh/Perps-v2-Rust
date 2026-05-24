use crate::{
    store::store::AppState,
    types::types::{AuthData, LogInResponse},
    utils::{token::create_token, user::User},
};
use actix_web::{
    HttpResponse, Responder,
    web::{self},
};
use uuid::Uuid;

pub async fn sign_up(body: web::Json<AuthData>, data: web::Data<AppState>) -> impl Responder {
    let user = body.0;
    //Check if user already exists;
    //How to get this data.
    let mut users = data.users.lock().unwrap();

    match users.get(&user.username) {
        Some(_) => {
            return HttpResponse::BadRequest().json("User already exists");
        }
        None => {
            let id = Uuid::new_v4().to_string();
            let new_user = User {
                id: id.clone(),
                username: user.username,
                password: user.password,
            };
            users.insert(id, new_user);
            return HttpResponse::Created().json("User created succcessfully");
        }
    }
}

pub async fn sign_in(body: web::Json<AuthData>, data: web::Data<AppState>) -> impl Responder {
    let user = body.0;
    let users = data.users.try_lock().unwrap();
    println!("Users are : {:?}", users);
    match users.get(&user.username) {
        Some(u) => {
            if u.password.eq(&user.password) {
                return HttpResponse::Ok().json(LogInResponse {
                    success: true,
                    token: create_token(u.id.clone()),
                    user_id: u.id.clone(),
                });
            } else {
                return HttpResponse::Unauthorized().json("Incorrect Password");
            }
        }
        None => return HttpResponse::NotFound().json("User not found"),
    }
}
