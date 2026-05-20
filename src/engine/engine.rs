use tokio::sync::mpsc::{self, Receiver};

use crate::engine::{
    create_order::create_order,
    liquidation::liquidation,
    types::{EngineRequest, Fill, Order, OrderBook, Position},
};
use std::collections::HashMap;

pub async fn run_engine(mut rx: Receiver<EngineRequest>) {
    let (btx, mut brx) = mpsc::channel(100);
    let (sol_tx, sol_rx) = mpsc::channel(100);
    let engine_thread = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            match &event {
                EngineRequest::CreateOrder { order, response_tx } => {
                    //According to data we have to forward it to the according thread.
                    match order.symbol.as_str() {
                        "BTC" => {
                            btx.send(event);
                        }
                        "SOL" => {
                            sol_tx.send(event);
                        }
                        _ => {
                            println!("This symbol is not supported");
                        }
                    }
                }
                EngineRequest::MarkPriceUpdate { data, response_tx } => {
                    match data.symbol.as_str() {
                        "BTC" => {
                            btx.send(event);
                        }
                        "SOL" => {
                            sol_tx.send(event);
                        }
                        _ => {
                            println!("This symbol is not supported");
                        }
                    }
                }
                EngineRequest::CheckBalance(data) => match data.symbol.as_str() {
                    "BTC" => {
                        btx.send(event).await.expect("Error in sending to the BTC_Thread");
                    }
                    "SOL" => {
                        sol_tx.send(event).await.expect("Error in sending to the Sol_thread");
                    }
                    _ => {
                        println!("This symbol is not supported");
                    }
                },
            }
        }
    });

    //BTC_Thread
    let Btc_thread = tokio::spawn(async move {
        let mut order_book: HashMap<u64, OrderBook> = HashMap::new();
        let mut orders: HashMap<String, Order> = HashMap::new();
        let mut positions: HashMap<String, Position> = HashMap::new();
        let mut fills: HashMap<String, Vec<Fill>> = HashMap::new();
        let mut current_index_price: u64;

        while let Some(data) = brx.recv().await {
            match data {
                EngineRequest::CreateOrder { order, response_tx } => {
                    //Whatever will be the returning data we will forward it from here only.
                    create_order(
                        order,
                        &mut orders,
                        &mut order_book,
                        &mut positions,
                        &mut fills,
                    );
                }
                EngineRequest::CheckBalance(data) => {}
                
                EngineRequest::MarkPriceUpdate { data, response_tx } => {
                    current_index_price = data.price;
                    liquidation(
                        current_index_price,
                        &mut positions,
                        &mut orders,
                        &mut fills,
                        &mut order_book,
                    );
                }
            }
        }
    });
}
