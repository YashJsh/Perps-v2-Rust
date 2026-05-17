use std::collections::HashMap;

use actix_web::{self, App, HttpServer, web};

mod controllers;
mod types;
mod store;
mod utils;

use controllers::auth::{sign_in, sign_up};

use crate::store::store::AppState;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Server is starting ");
    let _ = HttpServer::new(|| {
        App::new()
        .app_data(
            web::Data::new(
                AppState{
                    users : HashMap::new().into()
                }
            )
        )
        .service(
            web::scope("/api")
            .route("/signup", web::post().to(sign_up))
            .route("/signin", web::post().to(sign_in))
        )
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await;
    
    Ok(())
}