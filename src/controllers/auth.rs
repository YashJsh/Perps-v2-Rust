use actix_web::{HttpResponse, Responder, web::{self}};
use crate::{store::store::AppState, types::types::AuthData, utils::{token::create_token, user::User}};
use uuid::{Uuid};

pub async fn sign_up(body : web::Json<AuthData>, data : web::Data<AppState>)-> impl Responder{
    let user = body.0;
    //Check if user already exists;
    //How to get this data. 
    let mut users = data.users.lock().unwrap();

    match users.get(&user.username){
        Some(_) => {
            return HttpResponse::BadRequest().body("User already exists");
        },
        None => {
            let key = user.username.clone();
            let new_user = User{
                id : Uuid::new_v4().to_string(),
                username : user.username,
                password : user.password
            };
            users.insert(key, new_user);
            return HttpResponse::Created().body("User created succcessfully");
        }
    }
}

pub async fn sign_in(body : web::Json<AuthData>, data : web::Data<AppState>)-> impl Responder{
    let user = body.0;
    let users = data.users.try_lock().unwrap();
    
    match users.get(&user.username){
        Some(u) => {
            if u.password.eq(&user.password){
                return HttpResponse::Ok().body(create_token(u.id.clone()));
            }else{
                return HttpResponse::Unauthorized().body("Incorrect Password");
            }
        }
        None => {
            return HttpResponse::NotFound().body("User not found")
        }
    }
}