use std::collections::{BTreeMap, HashMap};
use tokio::sync::{
    mpsc::{self, Receiver},
};

use crate::{
    engine::{
        check_balance::balance_actor,
        create_order::create_order,
        delete_order::delete_order_func,
        get_depth::get_depth,
        liquidation::liquidation,
        types::{EngineRequest, Fill, Order, OrderBook, Position},
    },
    types::BalanceRequest,
};

pub async fn run_engine(mut rx: Receiver<EngineRequest>) {
    //BTC Thread
    let (btc_tx, mut btc_rx) = mpsc::channel(100);

    //Balance Thread transmitter and receiver
    let (balance_tx, balance_rx) = mpsc::channel::<BalanceRequest>(100);

    let btc_balance_tx = balance_tx.clone();

    //Balance thread;
    let _ = tokio::spawn(async move {
        balance_actor(balance_rx).await;
    });

    //router thread
    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            let symbol = event.symbol();
            match symbol {
                Some("BTC") => {
                    let _ = btc_tx.send(event);
                }
                None => match event {
                    EngineRequest::UpdateBalance {
                        user_id, amount, response_tx
                    } => {
                        let _ = balance_tx.send(BalanceRequest::AddBalance {
                            user_id,
                            amount,
                            response_tx
                        });
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    });

    //BTC_Thread
    let _ = std::thread::spawn(move || {
        let mut order_book = OrderBook {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
        };
        let mut orders: HashMap<u64, Order> = HashMap::new();
        let mut positions: HashMap<u64, Position> = HashMap::new();
        let mut fills: HashMap<u64, Vec<Fill>> = HashMap::new();
        let mut current_index_price: u64;

        while let Some(data) = btc_rx.blocking_recv() {
            match data {
                EngineRequest::CreateOrder { order, response_tx } => {
                    //Whatever will be the returning data we will forward it from here only.
                    let response = create_order(
                        order,
                        &mut orders,
                        &mut order_book,
                        &mut positions,
                        &mut fills,
                        &btc_balance_tx,
                    );
                    match response {
                        Ok(res) => {
                            let _ = response_tx.send(Ok(res));
                        }
                        Err(error) => {
                            let _ = response_tx.send(Err(error));
                        }
                    }
                }

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
                        &btc_balance_tx,
                    );
                }

                EngineRequest::DeleteOrderData { data, response_tx } => {
                    let data =
                        delete_order_func(data, &mut orders, &mut order_book, &btc_balance_tx);
                    let _ = response_tx.send(data);
                }

                EngineRequest::GetDepth { response_tx, .. } => {
                    let depth = get_depth(&order_book);
                    let _ = response_tx.send(Ok(depth));
                }

                _ => {}
            }
        }
    });
}
