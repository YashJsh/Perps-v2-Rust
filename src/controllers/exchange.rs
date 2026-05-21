use actix_web::{App, HttpResponse, Responder, web};
use tokio::sync::oneshot;

use crate::{
    engine::types::{EngineRequest, EngineResponse},
    store::store::AppState,
    types::types::{DeleteOrderData, IncomingOrder, OnRamp},
};

pub async fn on_ramp(body: web::Json<OnRamp>, data: web::Data<AppState>) -> impl Responder {
    let input_data = body.0;
    let user_id = input_data.user_id;
    let amount = input_data.amount;
    let mut balances = data
        .balances
        .try_lock()
        .expect("Unable to get the balances");
    let bal = balances
        .entry(user_id)
        .or_insert(crate::types::types::Balances {
            available: 0,
            locked: 0,
            currency: String::from("USD"),
        });
    bal.available += amount;
    HttpResponse::Ok().body("Balance updated successfully")
}

pub async fn create_order(
    body: web::Json<IncomingOrder>,
    data: web::Data<AppState>,
) -> impl Responder {
    let (tx, rx) = oneshot::channel();
    let incoming_data = body.0;
    let users = data.users.lock().expect("Error in getting lock on usres");

    println!("Users are : {:?}", users);
    //Checking if the user_id is matching with the user_id send;
    // match users.get(&incoming_data.user_id) {
    //     Some(_) => {}
    //     None => {
    //         println!("{:?}", users);
    //         return HttpResponse::BadRequest().body("User does not exist");
    //     }
    // };

    drop(users);

    let _ = data
        .sender
        .send(EngineRequest::CreateOrder {
            order: incoming_data,
            response_tx: tx,
        })
        .await;

    match rx.await {
        Ok(data) => match data {
            EngineResponse::CreateOrderResponse(res) => {
                return HttpResponse::Ok().json(res);
            }
            _ => HttpResponse::InternalServerError().body("Invalid response type"),
        },
        Err(_) => {
            return HttpResponse::BadGateway().body("No response from engine");
        }
    }
}

pub async fn delete_order(
    body: web::Json<DeleteOrderData>,
    data: web::Data<AppState>,
) -> impl Responder {
    let (tx, rx) = oneshot::channel();

    let _ = data
        .sender
        .send(EngineRequest::DeleteOrderData { data: body.0, response_tx: tx })
        .await;

    match rx.await {
        Ok(data) => match data {
            EngineResponse::CreateOrderResponse(res) => {
                return HttpResponse::Ok().json(res);
            }
            _ => HttpResponse::InternalServerError().body("Invalid response type"),
        },
        Err(_) => {
            return HttpResponse::BadGateway().body("No response from engine");
        }
    }
}
