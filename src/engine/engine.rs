use crate::{
    engine::{create_order::create_order, liquidation::liquidation, types::{EngineRequest, Fill, Order, OrderBook, Position}},
};
use std::{
    collections::HashMap,
    sync::mpsc::{self, Receiver},
    thread,
};

pub fn run_engine(rx: Receiver<EngineRequest>) {
    let (btx, brx) = mpsc::channel();
    let (sol_tx, sol_rx) = mpsc::channel();
    let engine_thread = thread::spawn(move || {
        for event in rx {
            match &event{ 
                EngineRequest::CreateOrder(data) => {
                    //According to data we have to forward it to the according thread.
                    match data.symbol.as_str() {
                        "BTC" => {
                            btx.send(event).unwrap();
                        }
                        "SOL" => {
                            sol_tx.send(event).unwrap();
                        }
                        _ => {
                            println!("This symbol is not supported");
                        }
                    }
                }
                EngineRequest::MarkPriceUpdate(data)=> {
                    match data.symbol.as_str(){
                        "BTC" => {
                            btx.send(event).unwrap();
                        },
                        "SOL" => {
                            sol_tx.send(event).unwrap();
                        }
                        _ => {
                            println!("This symbol is not supported");
                        }
                    }
                }
                EngineRequest::CheckBalance(data) => {
                    match data.symbol.as_str() {
                        "BTC" => {
                            btx.send(event).unwrap();
                        }
                        "SOL" => {
                            sol_tx.send(event).unwrap();
                        }
                        _ => {
                            println!("This symbol is not supported");
                        }
                    }  
                } 
            }
        }
    });

    //BTC_Thread
    let Btc_thread = thread::spawn(move || {
        let mut order_book: HashMap<u64, OrderBook> = HashMap::new();
        let mut orders: HashMap<String, Order> = HashMap::new();
        let mut positions: HashMap<String, Position> = HashMap::new();
        let mut fills : HashMap<String, Vec<Fill>> = HashMap::new();
        let mut current_index_price : u64;

        let data = match brx.recv() {
            Ok(d) => d,
            Err(_) => {
                println!("Error in recieving data");
                return;
            }
        };

        match data{
            EngineRequest::CreateOrder(data) => {
                create_order(data, &mut orders, &mut order_book, &mut positions, &mut fills);
            },
            EngineRequest::CheckBalance(data) => {

            },
            EngineRequest::MarkPriceUpdate(data)=> {
                current_index_price = data.price;
                liquidation(current_index_price, &mut positions, &mut orders, &mut fills, &mut order_book);
            }
        }
    });
}

