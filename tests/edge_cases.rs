use std::{
    collections::{BTreeMap, HashMap},
    thread,
};
use tokio::sync::mpsc;

use perps_v1::{
    engine::{
        check_balance::{
            handle_add_balance, handle_get_balance, lock_margin, reduce_balance, release_margin,
        },
        create_order::create_order,
        delete_order::delete_order_func,
        get_depth::get_depth,
        helper::risk_engine,
        liquidation::liquidation,
        types::{
            EngineError, Fill, Order, OrderBook, OrderSide, OrderStatus, Position,
            RestingOrder,
        },
    },
    types::{BalanceRequest, Balances, DeleteOrderData, IncomingOrder},
};

// ── Helpers ──────────────────────────────────────────────────────────

fn seed_balances(
    seeded: impl IntoIterator<Item = (u64, u64)>,
) -> HashMap<u64, Balances> {
    let mut balances = HashMap::new();
    for (user_id, amount) in seeded {
        balances.insert(
            user_id,
            Balances {
                available: amount,
                locked: 0,
                user_id,
            },
        );
    }
    balances
}

fn spawn_balance_actor(
    seeded: impl IntoIterator<Item = (u64, u64)>,
) -> (mpsc::Sender<BalanceRequest>, thread::JoinHandle<()>) {
    let (tx, mut rx) = mpsc::channel::<BalanceRequest>(32);
    let mut balances = seed_balances(seeded);

    let handle = thread::spawn(move || {
        while let Some(req) = rx.blocking_recv() {
            match req {
                BalanceRequest::AddBalance {
                    user_id,
                    amount,
                    response_tx,
                } => {
                    let _ = response_tx.send(handle_add_balance(user_id, amount, &mut balances));
                }
                BalanceRequest::LockMargin {
                    user_id,
                    amount,
                    response_tx,
                } => {
                    let _ = response_tx.send(lock_margin(user_id, &mut balances, amount));
                }
                BalanceRequest::ReleaseMargin {
                    user_id,
                    amount,
                    response_tx,
                } => {
                    let _ = response_tx.send(release_margin(user_id, &mut balances, amount));
                }
                BalanceRequest::GetBalance {
                    user_id,
                    response_tx,
                } => {
                    let _ = response_tx.send(handle_get_balance(user_id, &mut balances));
                }
                BalanceRequest::ReduceBalance {
                    user_id,
                    amount,
                    response_tx,
                } => {
                    let _ = response_tx.send(reduce_balance(user_id, &mut balances, amount));
                }
            }
        }
    });

    (tx, handle)
}

fn spawn_balance_actor_with_locked(
    seeded: impl IntoIterator<Item = (u64, u64, u64)>,
) -> (mpsc::Sender<BalanceRequest>, thread::JoinHandle<()>) {
    let (tx, mut rx) = mpsc::channel::<BalanceRequest>(32);
    let mut balances: HashMap<u64, Balances> = HashMap::new();
    for (user_id, available, locked) in seeded {
        balances.insert(
            user_id,
            Balances {
                available,
                locked,
                user_id,
            },
        );
    }

    let handle = thread::spawn(move || {
        while let Some(req) = rx.blocking_recv() {
            match req {
                BalanceRequest::AddBalance {
                    user_id,
                    amount,
                    response_tx,
                } => {
                    let _ = response_tx.send(handle_add_balance(user_id, amount, &mut balances));
                }
                BalanceRequest::LockMargin {
                    user_id,
                    amount,
                    response_tx,
                } => {
                    let _ = response_tx.send(lock_margin(user_id, &mut balances, amount));
                }
                BalanceRequest::ReleaseMargin {
                    user_id,
                    amount,
                    response_tx,
                } => {
                    let _ = response_tx.send(release_margin(user_id, &mut balances, amount));
                }
                BalanceRequest::GetBalance {
                    user_id,
                    response_tx,
                } => {
                    let _ = response_tx.send(handle_get_balance(user_id, &mut balances));
                }
                BalanceRequest::ReduceBalance {
                    user_id,
                    amount,
                    response_tx,
                } => {
                    let _ = response_tx.send(reduce_balance(user_id, &mut balances, amount));
                }
            }
        }
    });

    (tx, handle)
}

fn empty_engine_state() -> (
    HashMap<u64, Order>,
    OrderBook,
    HashMap<u64, Position>,
    HashMap<u64, Vec<Fill>>,
) {
    (
        HashMap::new(),
        OrderBook {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
        },
        HashMap::new(),
        HashMap::new(),
    )
}

fn limit_order(user_id: u64, side: OrderSide, price: u64, size: u64) -> IncomingOrder {
    IncomingOrder {
        user_id,
        order_type: perps_v1::engine::types::OrderType::Limit,
        order_side: side,
        symbol: "BTC".to_string(),
        size,
        price,
        leverage: 10,
        slippage: 0,
    }
}

fn market_order(user_id: u64, side: OrderSide, price: u64, size: u64) -> IncomingOrder {
    IncomingOrder {
        user_id,
        order_type: perps_v1::engine::types::OrderType::Market,
        order_side: side,
        symbol: "BTC".to_string(),
        size,
        price,
        leverage: 10,
        slippage: 0,
    }
}

fn create_order_or_panic(
    order: IncomingOrder,
    orders: &mut HashMap<u64, Order>,
    book: &mut OrderBook,
    positions: &mut HashMap<u64, Position>,
    fills: &mut HashMap<u64, Vec<Fill>>,
    balance_tx: &mpsc::Sender<BalanceRequest>,
    context: &str,
) -> perps_v1::engine::types::CreateOrderResponse {
    match create_order(order, orders, book, positions, fills, balance_tx) {
        Ok(response) => response,
        Err(e) => panic!("{context}: {e:?}"),
    }
}

fn order_id_for_user(orders: &HashMap<u64, Order>, user_id: u64) -> u64 {
    orders
        .iter()
        .find(|(_, order)| order.user_id == user_id)
        .map(|(order_id, _)| *order_id)
        .expect("expected order for user")
}

fn position_for_user<'a>(
    positions: &'a HashMap<u64, Position>,
    user_id: u64,
) -> &'a Position {
    positions
        .get(&user_id)
        .expect("expected position for user")
}

// ── A. Risk Engine Unit Tests ────────────────────────────────────────

#[test]
fn risk_engine_no_position_returns_true() {
    let positions: HashMap<u64, Position> = HashMap::new();
    assert!(risk_engine(&positions, 5, 5));
}

#[test]
fn risk_engine_same_side_increase_returns_true() {
    let mut positions = HashMap::new();
    positions.insert(
        5,
        Position {
            order_id: 100,
            average_entry_price: 100,
            symbol: "BTC".into(),
            margin: 50,
            size: 5,
            liquidation_price: 90,
            realized_pnl: None,
            time: "".into(),
            leverage: 10,
        },
    );
    assert!(risk_engine(&positions, 3, 5));
}

#[test]
fn risk_engine_opposite_side_reduces_returns_false() {
    let mut positions = HashMap::new();
    positions.insert(
        5,
        Position {
            order_id: 100,
            average_entry_price: 100,
            symbol: "BTC".into(),
            margin: 50,
            size: 5,
            liquidation_price: 90,
            realized_pnl: None,
            time: "".into(),
            leverage: 10,
        },
    );
    assert!(!risk_engine(&positions, -2, 5));
}

#[test]
fn risk_engine_exact_close_returns_false() {
    let mut positions = HashMap::new();
    positions.insert(
        5,
        Position {
            order_id: 100,
            average_entry_price: 100,
            symbol: "BTC".into(),
            margin: 50,
            size: 5,
            liquidation_price: 90,
            realized_pnl: None,
            time: "".into(),
            leverage: 10,
        },
    );
    assert!(!risk_engine(&positions, -5, 5));
}

#[test]
fn risk_engine_flip_returns_true() {
    let mut positions = HashMap::new();
    positions.insert(
        5,
        Position {
            order_id: 100,
            average_entry_price: 100,
            symbol: "BTC".into(),
            margin: 50,
            size: 5,
            liquidation_price: 90,
            realized_pnl: None,
            time: "".into(),
            leverage: 10,
        },
    );
    assert!(risk_engine(&positions, -7, 5));
}

#[test]
fn risk_engine_short_position_reduce_returns_false() {
    let mut positions = HashMap::new();
    positions.insert(
        5,
        Position {
            order_id: 100,
            average_entry_price: 100,
            symbol: "BTC".into(),
            margin: 50,
            size: -5,
            liquidation_price: 110,
            realized_pnl: None,
            time: "".into(),
            leverage: 10,
        },
    );
    assert!(!risk_engine(&positions, 2, 5));
}

#[test]
fn risk_engine_short_flip_returns_true() {
    let mut positions = HashMap::new();
    positions.insert(
        5,
        Position {
            order_id: 100,
            average_entry_price: 100,
            symbol: "BTC".into(),
            margin: 50,
            size: -5,
            liquidation_price: 110,
            realized_pnl: None,
            time: "".into(),
            leverage: 10,
        },
    );
    assert!(risk_engine(&positions, 7, 5));
}

// ── B. Market Orders ─────────────────────────────────────────────────

#[test]
fn market_buy_fills_against_resting_asks() {
    let (balance_tx, balance_thread) =
        spawn_balance_actor([(2, 10_000), (1, 10_000)]);
    let (mut orders, mut book, mut positions, mut fills) = empty_engine_state();

    create_order_or_panic(
        limit_order(2, OrderSide::Sell, 100, 3),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "resting sell should be accepted",
    );

    let response = create_order_or_panic(
        market_order(1, OrderSide::Buy, 110, 3),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "market buy should fill",
    );

    assert!(response.success);
    assert_eq!(response.filled_qty, 3);
    assert_eq!(response.remaining_qty, 0);

    let buyer_pos = position_for_user(&positions, 1);
    assert_eq!(buyer_pos.size, 3);
    assert_eq!(buyer_pos.average_entry_price, 100);

    drop(balance_tx);
    balance_thread.join().unwrap();
}

#[test]
fn market_sell_fills_against_resting_bids() {
    let (balance_tx, balance_thread) =
        spawn_balance_actor([(1, 10_000), (2, 10_000)]);
    let (mut orders, mut book, mut positions, mut fills) = empty_engine_state();

    create_order_or_panic(
        limit_order(1, OrderSide::Buy, 100, 3),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "resting bid should be accepted",
    );

    let response = create_order_or_panic(
        market_order(2, OrderSide::Sell, 90, 3),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "market sell should fill",
    );

    assert!(response.success);
    assert_eq!(response.filled_qty, 3);
    assert_eq!(response.remaining_qty, 0);

    let seller_pos = position_for_user(&positions, 2);
    assert_eq!(seller_pos.size, -3);
    assert_eq!(seller_pos.average_entry_price, 100);

    drop(balance_tx);
    balance_thread.join().unwrap();
}

#[test]
fn market_buy_fills_partially_when_insufficient_liquidity() {
    let (balance_tx, balance_thread) =
        spawn_balance_actor([(2, 10_000), (1, 10_000)]);
    let (mut orders, mut book, mut positions, mut fills) = empty_engine_state();

    create_order_or_panic(
        limit_order(2, OrderSide::Sell, 100, 2),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "resting sell should be accepted",
    );

    let response = create_order_or_panic(
        market_order(1, OrderSide::Buy, 110, 5),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "market buy should partially fill",
    );

    assert!(response.success);
    assert_eq!(response.filled_qty, 2);
    assert_eq!(response.remaining_qty, 3);

    let buyer_pos = position_for_user(&positions, 1);
    assert_eq!(buyer_pos.size, 2);

    drop(balance_tx);
    balance_thread.join().unwrap();
}

// ── C. Multi-level Matching ──────────────────────────────────────────

#[test]
fn buy_walks_ask_book_across_price_levels() {
    let (balance_tx, balance_thread) = spawn_balance_actor([
        (2, 10_000),
        (3, 10_000),
        (1, 10_000),
    ]);
    let (mut orders, mut book, mut positions, mut fills) = empty_engine_state();

    create_order_or_panic(
        limit_order(2, OrderSide::Sell, 100, 2),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "first resting sell",
    );
    create_order_or_panic(
        limit_order(3, OrderSide::Sell, 110, 3),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "second resting sell",
    );

    let response = create_order_or_panic(
        limit_order(1, OrderSide::Buy, 120, 4),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "buy should walk the book",
    );

    assert!(response.success);
    assert_eq!(response.filled_qty, 4);
    assert_eq!(response.remaining_qty, 0);

    assert!(book.asks.contains_key(&110));
    let remaining_at_110 = &book.asks[&110];
    assert_eq!(remaining_at_110.front().unwrap().remaining_qty, 1);
    assert!(!book.bids.contains_key(&120), "fully filled order should not be in bids");

    let buyer_pos = position_for_user(&positions, 1);
    let expected_avg = (100 * 2 + 110 * 2) / 4;
    assert_eq!(buyer_pos.average_entry_price, expected_avg);
    assert_eq!(buyer_pos.size, 4);

    drop(balance_tx);
    balance_thread.join().unwrap();
}

#[test]
fn sell_walks_bid_book_across_price_levels() {
    let (balance_tx, balance_thread) = spawn_balance_actor([
        (1, 10_000),
        (4, 10_000),
        (2, 10_000),
    ]);
    let (mut orders, mut book, mut positions, mut fills) = empty_engine_state();

    create_order_or_panic(
        limit_order(1, OrderSide::Buy, 110, 2),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "first resting bid",
    );
    create_order_or_panic(
        limit_order(4, OrderSide::Buy, 100, 3),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "second resting bid",
    );

    let response = create_order_or_panic(
        limit_order(2, OrderSide::Sell, 90, 4),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "sell should walk the book",
    );

    assert!(response.success);
    assert_eq!(response.filled_qty, 4);
    assert_eq!(response.remaining_qty, 0);

    assert!(book.bids.contains_key(&100));
    let remaining_at_100 = &book.bids[&100];
    assert_eq!(remaining_at_100.front().unwrap().remaining_qty, 1);
    assert!(!book.asks.contains_key(&90), "fully filled order should not be in asks");

    let seller_pos = position_for_user(&positions, 2);
    let expected_avg = (110 * 2 + 100 * 2) / 4;
    assert_eq!(seller_pos.average_entry_price, expected_avg);
    assert_eq!(seller_pos.size, -4);

    drop(balance_tx);
    balance_thread.join().unwrap();
}

#[test]
fn fifo_same_price_level_execution() {
    let (balance_tx, balance_thread) = spawn_balance_actor([
        (2, 10_000),
        (3, 10_000),
        (1, 10_000),
    ]);
    let (mut orders, mut book, mut positions, mut fills) = empty_engine_state();

    create_order_or_panic(
        limit_order(2, OrderSide::Sell, 100, 3),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "first resting sell",
    );

    let seller_1_order_id = order_id_for_user(&orders, 2);

    create_order_or_panic(
        limit_order(3, OrderSide::Sell, 100, 2),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "second resting sell",
    );

    let response = create_order_or_panic(
        limit_order(1, OrderSide::Buy, 100, 4),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "buy 4 against 2 sellers",
    );

    assert!(response.success);
    assert_eq!(response.filled_qty, 4);
    assert_eq!(response.remaining_qty, 0);

    let seller_1 = orders.get(&seller_1_order_id).unwrap();
    assert_eq!(seller_1.filled_qty, 3);
    assert_eq!(seller_1.remaining_qty, 0);

    let seller_2_order_id = order_id_for_user(&orders, 3);
    let seller_2 = orders.get(&seller_2_order_id).unwrap();
    assert_eq!(seller_2.filled_qty, 1);
    assert_eq!(seller_2.remaining_qty, 1);

    assert!(book.asks.contains_key(&100));
    let remaining_asks = &book.asks[&100];
    assert_eq!(remaining_asks.len(), 1);
    assert_eq!(remaining_asks.front().unwrap().remaining_qty, 1);
    assert_eq!(remaining_asks.front().unwrap().order_id, seller_2_order_id);

    drop(balance_tx);
    balance_thread.join().unwrap();
}

// ── D. Position: Full Close & Flip ──────────────────────────────────

#[test]
fn opposite_side_fully_closes_position_with_profit() {
    let (balance_tx, balance_thread) = spawn_balance_actor([
        (2, 10_000),
        (1, 10_000),
        (4, 10_000),
    ]);
    let (mut orders, mut book, mut positions, mut fills) = empty_engine_state();

    create_order_or_panic(
        limit_order(2, OrderSide::Sell, 100, 5),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "resting sell",
    );
    create_order_or_panic(
        limit_order(1, OrderSide::Buy, 100, 5),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "buyer builds long 5 at 100",
    );

    create_order_or_panic(
        limit_order(4, OrderSide::Buy, 110, 5),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "resting bid at 110 for close",
    );

    let response = create_order_or_panic(
        limit_order(1, OrderSide::Sell, 110, 5),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "sell 5 at 110 to close with profit",
    );

    assert!(response.success);
    assert_eq!(response.filled_qty, 5);
    assert_eq!(response.remaining_qty, 0);

    assert!(
        !positions.contains_key(&1),
        "position should be removed after full close with profit"
    );

    drop(balance_tx);
    balance_thread.join().unwrap();
}

#[test]
fn opposite_side_flips_long_to_short() {
    let (balance_tx, balance_thread) = spawn_balance_actor([
        (2, 10_000),
        (1, 80),
        (4, 10_000),
    ]);
    let (mut orders, mut book, mut positions, mut fills) = empty_engine_state();

    create_order_or_panic(
        limit_order(2, OrderSide::Sell, 100, 3),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "resting sell",
    );
    create_order_or_panic(
        limit_order(1, OrderSide::Buy, 100, 3),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "buyer builds long 3",
    );

    create_order_or_panic(
        limit_order(4, OrderSide::Buy, 100, 5),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "resting bid for flip",
    );

    let response = create_order_or_panic(
        limit_order(1, OrderSide::Sell, 100, 5),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "sell 5 to flip long 3 -> short 2",
    );

    assert!(response.success);
    assert_eq!(response.filled_qty, 5);
    assert_eq!(response.remaining_qty, 0);

    let pos = position_for_user(&positions, 1);
    assert_eq!(pos.size, -2, "should flip to short 2");
    assert_eq!(pos.average_entry_price, 100);
    assert_eq!(pos.margin, 20);

    drop(balance_tx);
    balance_thread.join().unwrap();
}

// ── E. Liquidation ──────────────────────────────────────────────────

#[test]
fn liquidation_calls_create_order_with_correct_side() {
    let (balance_tx, balance_thread) = spawn_balance_actor([
        (2, 10_000),
        (1, 10_000),
        (4, 10_000),
    ]);
    let (mut orders, mut book, mut positions, mut fills) = empty_engine_state();

    create_order_or_panic(
        limit_order(2, OrderSide::Sell, 100, 3),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "resting sell",
    );
    create_order_or_panic(
        limit_order(1, OrderSide::Buy, 100, 3),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "buyer builds long 3",
    );

    let pos = position_for_user(&positions, 1);
    assert_eq!(pos.liquidation_price, 90);

    // Add bids for the liquidation sell order to match against
    create_order_or_panic(
        limit_order(4, OrderSide::Buy, 100, 3),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "add bids for liquidation",
    );

    // should_liquidate returns true when 90 <= 95 (inverted condition)
    liquidation(95, &mut positions, &mut orders, &mut fills, &mut book, 100, &balance_tx);

    // Verify fills existed for the liquidation order
    let has_liquidation_fill = orders.values().any(|o| o.user_id == 1 && o.filled_qty > 0);
    assert!(has_liquidation_fill, "liquidation should have created fills");

    drop(balance_tx);
    balance_thread.join().unwrap();
}

// ── F. Delete Order ──────────────────────────────────────────────────

#[test]
fn delete_open_order_releases_margin_and_cancels() {
    let (balance_tx, balance_thread) = spawn_balance_actor_with_locked([(1, 10_000, 20)]);
    let (mut orders, mut book, mut positions, mut fills) = empty_engine_state();

    let response = create_order_or_panic(
        limit_order(1, OrderSide::Buy, 100, 2),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "limit order",
    );
    assert!(response.success);

    let order_id = order_id_for_user(&orders, 1);

    let result = delete_order_func(
        DeleteOrderData {
            order_id,
            user_id: 1,
            symbol: "BTC".to_string(),
        },
        &mut orders,
        &mut book,
        &balance_tx,
    );

    assert!(result.is_ok());
    let delete_res = result.unwrap();
    assert!(delete_res.success);
    assert_eq!(delete_res.order_id, order_id);

    let cancelled_order = orders.get(&order_id).unwrap();
    assert!(matches!(cancelled_order.status, OrderStatus::Cancelled));

    assert!(book.bids.get(&100).unwrap().is_empty());

    drop(balance_tx);
    balance_thread.join().unwrap();
}

#[test]
fn delete_filled_order_returns_error() {
    let (balance_tx, balance_thread) =
        spawn_balance_actor([(2, 10_000), (1, 10_000)]);
    let (mut orders, mut book, mut positions, mut fills) = empty_engine_state();

    create_order_or_panic(
        limit_order(2, OrderSide::Sell, 100, 3),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "resting sell",
    );
    create_order_or_panic(
        limit_order(1, OrderSide::Buy, 100, 3),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "buyer fills",
    );

    let buyer_order_id = order_id_for_user(&orders, 1);

    let result = delete_order_func(
        DeleteOrderData {
            order_id: buyer_order_id,
            user_id: 1,
            symbol: "BTC".to_string(),
        },
        &mut orders,
        &mut book,
        &balance_tx,
    );

    assert!(matches!(result, Err(EngineError::OrderFilledAlready)),
        "filled order should not be deletable");

    drop(balance_tx);
    balance_thread.join().unwrap();
}

#[test]
fn delete_nonexistent_order_returns_error() {
    let (balance_tx, balance_thread) = spawn_balance_actor([(1, 10_000)]);
    let (mut orders, mut book, _positions, _fills) = empty_engine_state();

    let result = delete_order_func(
        DeleteOrderData {
            order_id: 999,
            user_id: 1,
            symbol: "BTC".to_string(),
        },
        &mut orders,
        &mut book,
        &balance_tx,
    );

    assert!(matches!(result, Err(EngineError::OrderNotFound)));

    drop(balance_tx);
    balance_thread.join().unwrap();
}

#[test]
fn delete_one_of_multiple_orders_at_same_price() {
    let (balance_tx, balance_thread) = spawn_balance_actor_with_locked([
        (1, 10_000, 20),
        (4, 10_000, 30),
    ]);
    let (mut orders, mut book, mut positions, mut fills) = empty_engine_state();

    create_order_or_panic(
        limit_order(1, OrderSide::Buy, 100, 2),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "buyer-1 limit",
    );
    create_order_or_panic(
        limit_order(4, OrderSide::Buy, 100, 3),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "buyer-2 limit",
    );

    assert_eq!(book.bids.get(&100).unwrap().len(), 2);

    let buyer_2_order_id = order_id_for_user(&orders, 4);

    let result = delete_order_func(
        DeleteOrderData {
            order_id: buyer_2_order_id,
            user_id: 4,
            symbol: "BTC".to_string(),
        },
        &mut orders,
        &mut book,
        &balance_tx,
    );

    assert!(result.is_ok());

    let bid_level = book.bids.get(&100).unwrap();
    assert_eq!(bid_level.len(), 1);
    assert_eq!(bid_level.front().unwrap().order_id, order_id_for_user(&orders, 1));

    drop(balance_tx);
    balance_thread.join().unwrap();
}

// ── G. Balance Actor Unit Tests ──────────────────────────────────────

#[test]
fn get_balance_nonexistent_user_returns_error() {
    let mut balances: HashMap<u64, Balances> = HashMap::new();
    let result = handle_get_balance(99, &mut balances);
    assert!(matches!(result, Err(EngineError::UserNotFound)));
}

#[test]
fn add_balance_creates_new_user_entry() {
    let mut balances: HashMap<u64, Balances> = HashMap::new();
    let result = handle_add_balance(5, 500, &mut balances);
    assert!(result.is_ok());
    let resp = result.unwrap();
    assert_eq!(resp.balance, 500);
    assert_eq!(resp.locked, 0);

    let user = balances.get(&5).unwrap();
    assert_eq!(user.available, 500);
    assert_eq!(user.locked, 0);
}

#[test]
fn lock_margin_reduces_available_and_increases_locked() {
    let mut balances = seed_balances([(5, 1000)]);
    let result = lock_margin(5, &mut balances, 300);
    assert!(result.is_ok());
    let resp = result.unwrap();
    assert_eq!(resp.balance, 700);
    assert_eq!(resp.locked, 300);
}

#[test]
fn lock_margin_nonexistent_user_returns_error() {
    let mut balances: HashMap<u64, Balances> = HashMap::new();
    let result = lock_margin(99, &mut balances, 100);
    assert!(matches!(result, Err(EngineError::UserNotFound)));
}

#[test]
fn release_margin_increases_available_and_decreases_locked() {
    let mut balances = seed_balances([(5, 1000)]);
    let _ = lock_margin(5, &mut balances, 300).unwrap();
    let result = release_margin(5, &mut balances, 200);
    assert!(result.is_ok());
    let resp = result.unwrap();
    assert_eq!(resp.balance, 900);
    assert_eq!(resp.locked, 100);
}

#[test]
fn reduce_balance_decreases_available() {
    let mut balances = seed_balances([(5, 1000)]);
    let result = reduce_balance(5, &mut balances, 400);
    assert!(result.is_ok());
    let resp = result.unwrap();
    assert_eq!(resp.balance, 600);
}

// ── H. Get Depth ─────────────────────────────────────────────────────

#[test]
fn get_depth_empty_book_returns_empty_maps() {
    let book = OrderBook {
        bids: BTreeMap::new(),
        asks: BTreeMap::new(),
    };
    let depth = get_depth(&book);
    assert!(depth.success);
    assert!(depth.bids.is_empty());
    assert!(depth.asks.is_empty());
}

#[test]
fn get_depth_with_bids_and_asks_returns_correct_totals() {
    use std::collections::VecDeque;
    let mut book = OrderBook {
        bids: BTreeMap::new(),
        asks: BTreeMap::new(),
    };

    book.asks.insert(
        100,
        VecDeque::from(vec![
            RestingOrder {
                order_id: 101,
                user_id: 10,
                qty: 2,
                price: 100,
                remaining_qty: 2,
                symbol: "BTC".into(),
            },
            RestingOrder {
                order_id: 102,
                user_id: 10,
                qty: 3,
                price: 100,
                remaining_qty: 3,
                symbol: "BTC".into(),
            },
        ]),
    );
    book.asks.insert(
        110,
        VecDeque::from(vec![RestingOrder {
            order_id: 103,
            user_id: 20,
            qty: 5,
            price: 110,
            remaining_qty: 5,
            symbol: "BTC".into(),
        }]),
    );
    book.bids.insert(
        99,
        VecDeque::from(vec![RestingOrder {
            order_id: 104,
            user_id: 30,
            qty: 4,
            price: 99,
            remaining_qty: 4,
            symbol: "BTC".into(),
        }]),
    );

    let depth = get_depth(&book);
    assert!(depth.success);
    assert_eq!(depth.asks.get(&100), Some(&5u64));
    assert_eq!(depth.asks.get(&110), Some(&5u64));
    assert_eq!(depth.bids.get(&99), Some(&4u64));
}
