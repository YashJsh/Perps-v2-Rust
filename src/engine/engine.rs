use tokio::sync::mpsc::{self, Receiver};

use crate::engine::{
        create_order::create_order,
        delete_order::delete_order_func,
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
                            let _ = btx.send(event).await;
                        }
                        "SOL" => {
                            let _ = sol_tx.send(event).await;
                        }
                        _ => {
                            println!("This symbol is not supported");
                        }
                    }
                }
                EngineRequest::MarkPriceUpdate { data } => match data.symbol.as_str() {
                    "BTC" => {
                        let _ = btx.send(event).await;
                    }
                    "SOL" => {
                        let _ = sol_tx.send(event).await;
                    }
                    _ => {
                        println!("This symbol is not supported");
                    }
                },
                EngineRequest::CheckBalance(data) => match data.symbol.as_str() {
                    "BTC" => {
                        btx.send(event)
                            .await
                            .expect("Error in sending to the BTC_Thread");
                    }
                    "SOL" => {
                        sol_tx
                            .send(event)
                            .await
                            .expect("Error in sending to the Sol_thread");
                    }
                    _ => {
                        println!("This symbol is not supported");
                    }
                },
                EngineRequest::DeleteOrderData { data, response_tx } => {
                    match data.symbol.as_str() {
                        // let data = delete_order_func(data, &mut orders, &mut order_book);
                        // response_tx.send(data);
                        "BTC" => {
                            btx.send(event)
                                .await
                                .expect("Error in sending to the BTC_Thread");
                        }
                        "SOL" => {}
                        _ => {}
                    }
                }
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
                    let response = create_order(
                        order,
                        &mut orders,
                        &mut order_book,
                        &mut positions,
                        &mut fills,
                    );
                    match response {
                        Ok(res) => {
                            response_tx.send(Ok(res));
                        }
                        Err(error) => {
                            response_tx.send(Err(error));
                        }
                    }
                }
                EngineRequest::CheckBalance(data) => {}

                EngineRequest::MarkPriceUpdate { data } => {
                    current_index_price = data.price;
                    println!("Mark price recieved is : {}", current_index_price);
                    liquidation(
                        current_index_price,
                        &mut positions,
                        &mut orders,
                        &mut fills,
                        &mut order_book,
                    );
                }

                EngineRequest::DeleteOrderData { data, response_tx } => {
                    let data = delete_order_func(data, &mut orders, &mut order_book);
                    let _ = response_tx.send(data);
                }
            }
        }
    });
}
