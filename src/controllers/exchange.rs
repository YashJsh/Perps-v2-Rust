use actix_web::{App, HttpResponse, Responder, web};
use tokio::sync::oneshot;

use crate::{
    engine::types::{CreateOrderResponse, EngineError, EngineRequest},
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
    let (tx, rx) = oneshot::channel::<Result<CreateOrderResponse, EngineError>>();
    let incoming_data = body.0;
    let users = data.users.lock().expect("Error in getting lock on usres");

    println!("Users are : {:?}", users);

    drop(users);

    let _ = data
        .sender
        .send(EngineRequest::CreateOrder {
            order: incoming_data,
            response_tx: tx,
        })
        .await;

    match rx.await{
        Ok(response)=> {
            match response{
                Ok(res)=> {
                    return HttpResponse::Ok().json(res);
                },
                Err(err)=>{
                    return HttpResponse::BadRequest().json(err);
                }
            }
        },
        Err(_) => {
            println!("Error in recieving message from the engine");
            return HttpResponse::BadRequest().finish();
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
        .send(EngineRequest::DeleteOrderData {
            data: body.0,
            response_tx: tx,
        })
        .await;

    match rx.await {
        Ok(data) => match data {
            Ok(d)=>{
                return HttpResponse::Ok().json(d);
            }
            Err(err) => HttpResponse::BadRequest().json(err)
        },
        Err(_) => {
            return HttpResponse::BadGateway().body("No response from engine");
        }
    }
}
