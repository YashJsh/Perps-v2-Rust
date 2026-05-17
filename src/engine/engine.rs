use crate::{
    engine::{create_order::create_order, types::{Order, OrderBook, Position}},
    store::store::{EngineRequest, RequestType},
};
use std::{
    collections::HashMap,
    sync::mpsc::{self, Receiver},
    thread,
};

pub enum EngineCommands {
    CreateOrder,
    CheckBalance,
    CancelOrder,
}

pub fn run_engine(rx: Receiver<EngineRequest>) {
    let (btx, brx) = mpsc::channel();
    let (sol_tx, sol_rx) = mpsc::channel();
    let engine_thread = thread::spawn(move || {
        for data in rx {
            match data.request_type {
                RequestType::CreateOrder => {
                    let sym = &data.data.symbol;
                    //According to data we have to forward it to the according thread.
                    match sym.as_str() {
                        "BTC" => {
                            btx.send(data);
                        }
                        "SOL" => {
                            sol_tx.send(data);
                        }
                        _ => {
                            println!("This symbol is not supported");
                        }
                    }
                }
                RequestType::CheckBalance => {}
            }
        }
    });

    //BTC_Thread
    let Btc_thread = thread::spawn(move || {
        let mut order_book: HashMap<u64, OrderBook> = HashMap::new();
        let mut orders: HashMap<u64, Order> = HashMap::new();
        let mut positions: Vec<Position> = Vec::new();

        let data = match brx.recv() {
            Ok(d) => d,
            Err(_) => {
                println!("Error in recieving data");
                return;
            }
        };

        match data.request_type{
            RequestType::CreateOrder => {
                create_order(data.data, &mut orders, &mut order_book, &mut positions);
            },
            RequestType::CheckBalance => {
                //Function for Checking balance
            }
        }
    });
}
