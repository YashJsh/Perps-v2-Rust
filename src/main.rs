use std::{collections::HashMap, sync::mpsc::{self, Receiver}};

mod controllers;
mod types;
mod store;
mod utils;
mod engine;
mod websocket;

use actix_web::{self, App, HttpServer, web};

use controllers::auth::{sign_in, sign_up};

use crate::{engine::engine::run_engine, store::store::{AppState}, websocket::connection::connect_stream};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let (tx,rx) = mpsc::channel();
    let tx1 = tx.clone();
    connect_stream(tx1);
    // run_engine(rx); //This runs the engine.
    
    
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