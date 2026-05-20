use actix_web::{HttpResponse, Responder, web};

use crate::{store::store::AppState, types::types::OnRamp};

pub async fn on_ramp(body: web::Data<OnRamp>, data: web::Data<AppState>) -> impl Responder {
    let user_id = body.user_id.clone();
    let amount = body.amount;
    let mut balances = data
        .balances
        .try_lock()
        .expect("Unable to get the balances");
    let bal = balances
        .entry(user_id)
        .or_insert(crate::types::types::Balances {
            available: 0,
            locked: 0,
            currency : String::from("USD"),
        });

    bal.available += amount;
    HttpResponse::Ok().body("Balance updated successfully")
}


