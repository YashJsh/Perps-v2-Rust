use tokio::sync::mpsc::{self, Receiver};

use crate::{
    engine::{
        create_order::create_order,
        delete_order::delete_order_func,
        get_depth::get_depth,
        liquidation::liquidation,
        types::{BalanceResponse, EngineRequest, Fill, Order, OrderBook, Position},
    },
    types::types::Balances,
};

use std::{
    collections::{BTreeMap, HashMap},
};

pub async fn run_engine(mut rx: Receiver<EngineRequest>) {
    let (btx, mut brx) = mpsc::channel(100);
    let (sol_tx, sol_rx) = mpsc::channel(100);
    let mut BALANCES: HashMap<String, Balances> = HashMap::new();

    let engine_thread = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                EngineRequest::UpdateBalance {
                    user_id,
                    amount,
                    response_tx,
                } => {
                    let balance = BALANCES.entry(user_id.to_string()).or_insert(Balances {
                        available: amount,
                        locked: 0,
                        currency: String::from("USD"),
                    });

                    balance.available += amount;

                    let _ = response_tx.send(Ok(BalanceResponse {
                        user_id: user_id.to_string(),
                        balance: balance.available,
                    }));
                }

                other_event => match &other_event {
                    EngineRequest::CreateOrder { order, .. } => match order.symbol.as_str() {
                        "BTC" => {
                            let _ = btx.send(other_event).await;
                        }

                        "SOL" => {
                            let _ = sol_tx.send(other_event).await;
                        }

                        _ => {
                            println!("This symbol is not supported");
                        }
                    },

                    EngineRequest::MarkPriceUpdate { data } => match data.symbol.as_str() {
                        "BTC" => {
                            let _ = btx.send(other_event).await;
                        }

                        "SOL" => {
                            let _ = sol_tx.send(other_event).await;
                        }

                        _ => {
                            println!("This symbol is not supported");
                        }
                    },

                    EngineRequest::CheckBalance(data) => match data.symbol.as_str() {
                        "BTC" => {
                            let _ = btx.send(other_event).await;
                        }

                        "SOL" => {
                            let _ = sol_tx.send(other_event).await;
                        }

                        _ => {
                            println!("This symbol is not supported");
                        }
                    },

                    EngineRequest::DeleteOrderData { data, .. } => match data.symbol.as_str() {
                        "BTC" => {
                            let _ = btx.send(other_event).await;
                        }

                        "SOL" => {
                            let _ = sol_tx.send(other_event).await;
                        }

                        _ => {
                            println!("This symbol is not supported");
                        }
                    },

                    EngineRequest::GetDepth { symbol, .. } => match symbol.as_str() {
                        "BTC" => {
                            let _ = btx.send(other_event).await;
                        }

                        "SOL" => {
                            let _ = sol_tx.send(other_event).await;
                        }

                        _ => {
                            println!("This symbol is not supported");
                        }
                    },

                    _ => {}
                },    
            }
        }
    });

    //BTC_Thread
    let Btc_thread = tokio::spawn(async move {
        let mut order_book = OrderBook {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
        };
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
                        current_index_price,
                    );
                }

                EngineRequest::DeleteOrderData { data, response_tx } => {
                    let data = delete_order_func(data, &mut orders, &mut order_book);
                    let _ = response_tx.send(data);
                }

                EngineRequest::GetDepth {
                    symbol,
                    response_tx,
                } => {
                    let depth = get_depth(&order_book);
                    let _ = response_tx.send(Ok(depth));
                }

                EngineRequest::UpdateBalance {
                    user_id,
                    amount,
                    response_tx,
                } => {}
            }
        }
    });
}
