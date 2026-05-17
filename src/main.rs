use std::{collections::HashMap, sync::mpsc::{self, Receiver}};

mod controllers;
mod types;
mod store;
mod utils;
mod engine;

use actix_web::{self, App, HttpServer, web};

use controllers::auth::{sign_in, sign_up};
use queues::Queue;

use crate::{engine::engine::run_engine, store::store::{AppState, EngineRequest}};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let (tx,rx) = mpsc::channel();
    run_engine(rx);
    
    println!("Server is starting ");
    let _ = HttpServer::new(move|| {
        App::new()
        .app_data(
            web::Data::new(
                AppState{
                    users : HashMap::new().into(),
                    sender : tx.clone()
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