use std::sync::mpsc;

use actix_web::mime::JSON;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::net::unix::pipe::Sender;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Message, client::IntoClientRequest},
};

use crate::{engine::types::EngineRequest, store::store::RequestType};

pub fn connect_stream(tx: mpsc::Sender<EngineRequest>) {
    let new_thread = tokio::spawn(async {
        let url = "wss://fstream.binance.com/market/ws";
        let client_request = url.into_client_request().expect("Wrong URL");
        println!("Connecting with the websocket server with url : {}", url);
        let (stream, response_http) = connect_async(url).await.expect("Error in connecting");
        println!("Connected to binance");
        println!("Handshake response HTTP code: {}", response_http.status());

        let (mut write, mut read) = stream.split();

        let message = Message::Text(
            json!({
                "method": "SUBSCRIBE",
                "params": ["!markPrice@arr"],
                "id": 1
            })
            .to_string()
            .into(),
        );
        write
            .send(message)
            .await
            .expect("Error in sending the messgae");

        while let Some(message) = read.next().await {
            match message {
                Ok(msg) => {
                    println!("Message is : {}", msg.to_text().unwrap());
                    // tx.send(EngineRequest { 
                    //     request_type: RequestType::UpateMarkPrice, 
                    //     data: "Hello"
                    // })
                }
                Err(e) => println!("Error is : {}", e),
            }
        }
    });
}