use std::{collections::HashMap};
use tokio::sync::mpsc::Receiver;

use crate::{
    engine::types::{BalanceResponse, EngineError},
    types::{BalanceRequest, Balances},
};


pub async fn balance_actor(mut balance_rx: Receiver<BalanceRequest>) {
    let mut balances: HashMap<u64, Balances> = HashMap::new();

    while let Some(req) = balance_rx.recv().await {
        match req {
            BalanceRequest::AddBalance {
                user_id,
                amount,
                response_tx,
            } => {
                let res = handle_add_balance(user_id, amount, &mut balances);
                match res {
                    Ok(r) => {
                        let _ = response_tx.send(Ok(r));
                    }
                    Err(e) => {
                        let _ = response_tx.send(Err(e));
                    }
                }
            }
            BalanceRequest::GetBalance {
                user_id,
                response_tx,
            } => {
                let res = handle_get_balance(user_id, &mut balances);
                match res {
                    Ok(s) => {
                        let _ = response_tx.send(Ok(s));
                    }
                    Err(e) => {
                        let _ = response_tx.send(Err(e));
                    }
                }
            }
            BalanceRequest::LockMargin {
                user_id,
                amount,
                response_tx,
            } => {
                let res = lock_margin(user_id, &mut balances, amount);
                match res {
                    Ok(s) => {
                        let _ = response_tx.send(Ok(s));
                    }
                    Err(e) => {
                        let _ = response_tx.send(Err(e));
                    }
                }
            }
            BalanceRequest::ReleaseMargin {
                user_id,
                amount,
                response_tx,
            } => {
                let res = release_margin(user_id, &mut balances, amount);
                match res {
                    Ok(s) => {
                        let _ = response_tx.send(Ok(s));
                    }
                    Err(e) => {
                        let _ = response_tx.send(Err(e));
                    }
                }
            }
            BalanceRequest::ReduceBalance {
                user_id,
                amount,
                response_tx,
            } => {
                let res = reduce_balance(user_id, &mut balances, amount);
                match res {
                    Ok(s) => {
                        let _ = response_tx.send(Ok(s));
                    }
                    Err(e) => {
                        let _ = response_tx.send(Err(e));
                    }
                }
            }
        }
    }
}

pub fn handle_add_balance(
    user_id: u64,
    amount: u64,
    balances: &mut HashMap<u64, Balances>,
) -> Result<BalanceResponse, EngineError> {
    let user = balances.entry(user_id).or_insert(Balances {
        available: 0,
        locked: 0,
        user_id,
    });
    user.available += amount;
    Ok(BalanceResponse {
        user_id,
        balance: user.available,
        locked: user.locked,
    })
}

pub fn handle_get_balance(
    user_id: u64,
    balances: &mut HashMap<u64, Balances>,
) -> Result<BalanceResponse, EngineError> {
    match balances.get(&user_id) {
        Some(b) => {
            return Ok(BalanceResponse {
                user_id,
                balance: b.available,
                locked: b.locked,
            });
        }
        None => return Err(EngineError::UserNotFound),
    }
}

pub fn lock_margin(
    user_id: u64,
    balances: &mut HashMap<u64, Balances>,
    amount: u64,
) -> Result<BalanceResponse, EngineError> {
    match balances.get_mut(&user_id) {
        Some(b) => {
            b.available -= amount;
            b.locked += amount;
            return Ok(BalanceResponse {
                user_id,
                balance: b.available,
                locked: b.locked,
            });
        }
        None => return Err(EngineError::UserNotFound),
    }
}

pub fn release_margin(
    user_id: u64,
    balances: &mut HashMap<u64, Balances>,
    amount: u64,
) -> Result<BalanceResponse, EngineError> {
    match balances.get_mut(&user_id) {
        Some(b) => {
            b.available += amount;
            b.locked -= amount;
            return Ok(BalanceResponse {
                user_id,
                balance: b.available,
                locked: b.locked,
            });
        }
        None => return Err(EngineError::UserNotFound),
    }
}

pub fn reduce_balance(
    user_id: u64,
    balances: &mut HashMap<u64, Balances>,
    amount: u64,
) -> Result<BalanceResponse, EngineError> {
    match balances.get_mut(&user_id) {
        Some(b) => {
            b.available -= amount;
            return Ok(BalanceResponse {
                user_id,
                balance: b.available,
                locked: b.locked,
            });
        }
        None => return Err(EngineError::UserNotFound),
    }
}
