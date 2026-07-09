use std::{collections::HashMap, sync::Mutex};

use actix_web::{self, App, HttpServer, web};
use perps_v1::{
    controllers::{
        auth::{sign_in, sign_up},
        exchange::{create_order, delete_order, get_depth, on_ramp},
    },
    engine::{engine::run_engine, types::EngineRequest},
    store::AppState, websocket::connection::connect_stream,
};

use tokio::sync::mpsc;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    let (tx, mut rx) = mpsc::channel::<EngineRequest>(100);
    let tx1 = tx.clone();

    connect_stream(tx1);
    run_engine(rx).await; 

    println!("Server is starting ");
    let app_state = web::Data::new(AppState {
        users: Mutex::new(HashMap::new()),
        sender: tx.clone(),
    });
    let _ = HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(
                web::scope("/api")
                    .route("/signup", web::post().to(sign_up))
                    .route("/signin", web::post().to(sign_in)),
            )
            .service(web::scope("/onramp").route("/", web::post().to(on_ramp)))
            .service(web::scope("/order")
                .route("/create", web::post().to(create_order))
                .route("/delete", web::post().to(delete_order))
            )
            .service(web::scope("/depth").route("/", web::post().to(get_depth)))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await;

    Ok(())
}
