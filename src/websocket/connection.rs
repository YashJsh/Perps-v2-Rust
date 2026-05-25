use crate::{
    engine::types::EngineRequest, types::types::MarkPriceData,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Message},
};

pub fn connect_stream(tx: Sender<EngineRequest>) {
    tokio::spawn(async move {
        let url = "wss://fstream.binance.com/market/ws";
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
                    //println!("Message is : {}", msg.to_text().unwrap());
                    tx.send(EngineRequest::MarkPriceUpdate {
                        data: MarkPriceData {
                            price: 10000,
                            symbol: String::from("BTC"),
                        },
                    })
                    .await
                    .expect("Error in sending price to the engine thread");
                }
                Err(e) => println!("Error is : {}", e),
            }
        }
    });
}
