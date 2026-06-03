use crate::{
    engine::types::EngineRequest, types::types::MarkPriceData,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Message},
};

#[derive(Serialize, Deserialize, Debug)]
struct IncomingStreamData{
    e : String,
    E : u64,
    i : String,
    p : String
}

pub fn connect_stream(tx: Sender<EngineRequest>) {
    tokio::spawn(async move {
        let url = "wss://fstream.binance.com/market/ws";
        let url2 = "wss://dstream.binance.com/ws/btcusd@indexPrice";
        println!("Connecting with the websocket server with url : {}", url);
        let (stream, response_http) = connect_async(url2).await.expect("Error in connecting");
        println!("Connected to binance");
        println!("Handshake response HTTP code: {}", response_http.status());

        let (mut write, mut read) = stream.split();

        // let message = Message::Text(
        //     json!({
        //         "method": "SUBSCRIBE",
        //         "params": ["!markPrice@arr"],
        //         "id": 1
        //     })
        //     .to_string()
        //     .into(),
        // );
        // write
        //     .send(message)
        //     .await
        //     .expect("Error in sending the messgae");

        while let Some(message) = read.next().await {
            match message {
                Ok(msg) => {
                    println!("Message is : {}", msg.to_text().unwrap());
                    let text  = msg.to_text().unwrap();
                    let parsed : IncomingStreamData = serde_json::from_str(text).unwrap();
                    let price = parsed.p.parse::<f64>().expect("Not a valid data");
                    println!("Price is : {}", price as u64);
                    tx.send(EngineRequest::MarkPriceUpdate {
                        data: MarkPriceData {
                            price: price as u64,
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
