use crate::{
    store::AppState, types::{AuthData, LogInResponse}, utils::{token::create_token, user::User},
};
use actix_web::{
    HttpResponse, Responder,
    web::{self},
};
pub async fn sign_up(body: web::Json<AuthData>, data: web::Data<AppState>) -> impl Responder {
    let user = body.0;
    let mut users = data.users.lock().unwrap();

    match users.get(&user.user_id) {
        Some(_) => {
            return HttpResponse::BadRequest().json("User already exists");
        }
        None => {
            let new_user = User {
                id: user.user_id,
                username: user.user_id.to_string(),
                password: user.password,
            };
            users.insert(user.user_id, new_user);
            return HttpResponse::Created().json("User created succcessfully");
        }
    }
}

pub async fn sign_in(body: web::Json<AuthData>, data: web::Data<AppState>) -> impl Responder {
    let user = body.0;
    let users = data.users.try_lock().unwrap();
    println!("Users are : {:?}", users);
    match users.get(&user.user_id) {
        Some(u) => {
            if u.password.eq(&user.password) {
                return HttpResponse::Ok().json(LogInResponse {
                    success: true,
                    token: create_token(u.id),
                    user_id: u.id,
                });
            } else {
                return HttpResponse::Unauthorized().json("Incorrect Password");
            }
        }
        None => return HttpResponse::NotFound().json("User not found"),
    }
}
