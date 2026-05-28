use perps_v1::{
    engine::{
        check_balance::{
            handle_add_balance, handle_get_balance, lock_margin, reduce_balance, release_margin,
        },
        create_order::create_order,
        types::{EngineError, Fill, Order, OrderBook, OrderSide, OrderStatus, Position},
    },
    types::types::{BalanceRequest, Balances, IncomingOrder},
};
use std::{
    collections::{BTreeMap, HashMap},
    thread,
};
use tokio::sync::mpsc;

fn seed_balances(
    seeded: impl IntoIterator<Item = (&'static str, u64)>,
) -> HashMap<String, Balances> {
    let mut balances = HashMap::new();
    for (user_id, amount) in seeded {
        balances.insert(
            user_id.to_string(),
            Balances {
                available: amount,
                locked: 0,
                user_id: user_id.to_string(),
            },
        );
    }
    balances
}

fn spawn_balance_actor(
    seeded: impl IntoIterator<Item = (&'static str, u64)>,
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

fn empty_engine_state() -> (
    HashMap<String, Order>,
    OrderBook,
    HashMap<String, Position>,
    HashMap<String, Vec<Fill>>,
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

fn limit_order(
    user_id: &str,
    side: OrderSide,
    price: u64,
    size: u64,
) -> IncomingOrder {
    IncomingOrder {
        user_id: user_id.to_string(),
        order_type: perps_v1::engine::types::OrderType::Limit,
        order_side: side,
        symbol: "BTC".to_string(),
        size,
        price,
        leverage: 10,
        slippage: 0,
    }
}

fn order_status_is(status: &OrderStatus, expected: OrderStatus) -> bool {
    matches!(
        (status, expected),
        (OrderStatus::Open, OrderStatus::Open)
            | (OrderStatus::Filled, OrderStatus::Filled)
            | (OrderStatus::PartiallyFilled, OrderStatus::PartiallyFilled)
            | (OrderStatus::Cancelled, OrderStatus::Cancelled)
    )
}

fn create_order_or_panic(
    order: IncomingOrder,
    orders: &mut HashMap<String, Order>,
    book: &mut OrderBook,
    positions: &mut HashMap<String, Position>,
    fills: &mut HashMap<String, Vec<Fill>>,
    balance_tx: &mpsc::Sender<BalanceRequest>,
    context: &str,
) -> perps_v1::engine::types::CreateOrderResponse {
    match create_order(order, orders, book, positions, fills, balance_tx) {
        Ok(response) => response,
        Err(_) => panic!("{context}"),
    }
}

fn order_id_for_user(orders: &HashMap<String, Order>, user_id: &str) -> String {
    orders
        .iter()
        .find(|(_, order)| order.user_id == user_id)
        .map(|(order_id, _)| order_id.clone())
        .expect("expected order for user")
}

#[test]
fn limit_order_with_no_match_sits_in_book() {
    let (balance_tx, balance_thread) = spawn_balance_actor([("buyer-1", 10_000)]);
    let (mut orders, mut book, mut positions, mut fills) = empty_engine_state();

    let response = create_order_or_panic(
        limit_order("buyer-1", OrderSide::Buy, 100, 2),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "limit order should be accepted",
    )
    ;

    assert!(response.success);
    assert_eq!(response.filled_qty, 0);
    assert_eq!(response.remaining_qty, 2);
    assert!(order_status_is(&response.order_status, OrderStatus::Open));

    let bid_level = book.bids.get(&100).expect("resting bid should be in book");
    assert_eq!(bid_level.len(), 1);
    assert_eq!(bid_level.front().unwrap().remaining_qty, 2);
    assert!(book.asks.is_empty());
    assert_eq!(orders.len(), 1);
    assert_eq!(fills.len(), 1);
    assert!(fills.values().next().unwrap().is_empty());
    assert!(positions.is_empty());

    drop(balance_tx);
    balance_thread.join().unwrap();
}

#[test]
fn incoming_limit_buy_matches_resting_sell_order() {
    let (balance_tx, balance_thread) =
        spawn_balance_actor([("seller-1", 10_000), ("buyer-1", 10_000)]);
    let (mut orders, mut book, mut positions, mut fills) = empty_engine_state();

    let resting_response = create_order_or_panic(
        limit_order("seller-1", OrderSide::Sell, 100, 3),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "resting sell should be accepted",
    )
    ;

    assert!(order_status_is(&resting_response.order_status, OrderStatus::Open));
    let resting_order_id = orders.keys().next().unwrap().clone();

    let response = create_order_or_panic(
        limit_order("buyer-1", OrderSide::Buy, 100, 3),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "incoming buy should match resting ask",
    )
    ;

    assert!(response.success);
    assert_eq!(response.filled_qty, 3);
    assert_eq!(response.remaining_qty, 0);
    assert!(order_status_is(&response.order_status, OrderStatus::Filled));
    assert!(book.asks.is_empty());

    let maker_order = orders.get(&resting_order_id).unwrap();
    assert_eq!(maker_order.filled_qty, 3);
    assert_eq!(maker_order.remaining_qty, 0);

    let taker_fills = fills
        .values()
        .find(|order_fills| !order_fills.is_empty() && order_fills[0].taker_id != resting_order_id)
        .expect("incoming order should record fills");
    assert_eq!(taker_fills[0].qty, 3);
    assert_eq!(taker_fills[0].price, 100);

    drop(balance_tx);
    balance_thread.join().unwrap();
}

#[test]
fn incoming_limit_sell_should_remove_fully_matched_resting_bid() {
    let (balance_tx, balance_thread) =
        spawn_balance_actor([("buyer-1", 10_000), ("seller-1", 10_000)]);
    let (mut orders, mut book, mut positions, mut fills) = empty_engine_state();

    let resting_response = create_order_or_panic(
        limit_order("buyer-1", OrderSide::Buy, 100, 4),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "resting bid should be accepted",
    )
    ;

    assert!(order_status_is(&resting_response.order_status, OrderStatus::Open));

    let response = create_order_or_panic(
        limit_order("seller-1", OrderSide::Sell, 100, 4),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "incoming sell should match resting bid",
    )
    ;

    assert!(response.success);
    assert_eq!(response.filled_qty, 4);
    assert_eq!(response.remaining_qty, 0);
    assert!(order_status_is(&response.order_status, OrderStatus::Filled));
    assert!(
        book.bids.is_empty(),
        "fully matched bid levels should be removed from the book"
    );

    drop(balance_tx);
    balance_thread.join().unwrap();
}

#[test]
fn incoming_limit_buy_partial_fill_keeps_remaining_qty_in_book() {
    let (balance_tx, balance_thread) =
        spawn_balance_actor([("seller-1", 10_000), ("buyer-1", 10_000)]);
    let (mut orders, mut book, mut positions, mut fills) = empty_engine_state();

    create_order_or_panic(
        limit_order("seller-1", OrderSide::Sell, 100, 2),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "resting sell should be accepted",
    );
    let seller_order_id = order_id_for_user(&orders, "seller-1");

    let response = create_order_or_panic(
        limit_order("buyer-1", OrderSide::Buy, 100, 5),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "incoming buy should be partially filled",
    );

    assert!(response.success);
    assert_eq!(response.filled_qty, 2);
    assert_eq!(response.remaining_qty, 3);
    assert!(order_status_is(
        &response.order_status,
        OrderStatus::PartiallyFilled
    ));

    let bid_level = book
        .bids
        .get(&100)
        .expect("remaining buy quantity should rest in bid book");
    assert_eq!(bid_level.front().unwrap().remaining_qty, 3);
    assert!(book.asks.is_empty());

    let seller_order = orders.get(&seller_order_id).unwrap();
    assert_eq!(seller_order.filled_qty, 2);
    assert_eq!(seller_order.remaining_qty, 0);

    drop(balance_tx);
    balance_thread.join().unwrap();
}

#[test]
fn create_order_rejects_when_margin_is_not_enough() {
    let (balance_tx, balance_thread) = spawn_balance_actor([("buyer-1", 20)]);
    let (mut orders, mut book, mut positions, mut fills) = empty_engine_state();

    let result = create_order(
        limit_order("buyer-1", OrderSide::Buy, 100, 5),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
    );

    assert!(matches!(result, Err(EngineError::NotEnoughBalance)));
    assert!(orders.is_empty());
    assert!(book.bids.is_empty());
    assert!(book.asks.is_empty());
    assert!(positions.is_empty());
    assert!(fills.is_empty());

    drop(balance_tx);
    balance_thread.join().unwrap();
}

#[test]
fn first_match_creates_a_new_position_for_the_taker() {
    let (balance_tx, balance_thread) =
        spawn_balance_actor([("seller-1", 10_000), ("buyer-1", 10_000)]);
    let (mut orders, mut book, mut positions, mut fills) = empty_engine_state();

    create_order_or_panic(
        limit_order("seller-1", OrderSide::Sell, 100, 3),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "resting sell should be accepted",
    );

    create_order_or_panic(
        limit_order("buyer-1", OrderSide::Buy, 100, 3),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "incoming buy should create a position",
    );

    let buyer_position = positions.get("buyer-1").expect("buyer position should exist");
    assert_eq!(buyer_position.size, 3);
    assert_eq!(buyer_position.average_entry_price, 100);
    assert_eq!(buyer_position.margin, 30);
    assert_eq!(buyer_position.symbol, "BTC");

    drop(balance_tx);
    balance_thread.join().unwrap();
}

#[test]
fn matching_same_side_again_increases_existing_position() {
    let (balance_tx, balance_thread) = spawn_balance_actor([
        ("seller-1", 10_000),
        ("seller-2", 10_000),
        ("buyer-1", 10_000),
    ]);
    let (mut orders, mut book, mut positions, mut fills) = empty_engine_state();

    create_order_or_panic(
        limit_order("seller-1", OrderSide::Sell, 100, 3),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "first resting sell should be accepted",
    );
    create_order_or_panic(
        limit_order("buyer-1", OrderSide::Buy, 100, 3),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "first buy should create a position",
    );

    create_order_or_panic(
        limit_order("seller-2", OrderSide::Sell, 120, 2),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "second resting sell should be accepted",
    );
    create_order_or_panic(
        limit_order("buyer-1", OrderSide::Buy, 120, 2),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "second buy should extend the position",
    );

    let buyer_position = positions.get("buyer-1").expect("buyer position should exist");
    assert_eq!(buyer_position.size, 5);
    assert_eq!(buyer_position.average_entry_price, 108);
    assert_eq!(buyer_position.margin, 54);
    assert_eq!(buyer_position.liquidation_price, 98);

    drop(balance_tx);
    balance_thread.join().unwrap();
}

#[test]
fn opposite_side_match_reduces_existing_position() {
    let (balance_tx, balance_thread) = spawn_balance_actor([
        ("seller-1", 10_000),
        ("buyer-1", 10_000),
        ("buyer-2", 10_000),
    ]);
    let (mut orders, mut book, mut positions, mut fills) = empty_engine_state();

    create_order_or_panic(
        limit_order("seller-1", OrderSide::Sell, 100, 5),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "resting sell should be accepted",
    );
    create_order_or_panic(
        limit_order("buyer-1", OrderSide::Buy, 100, 5),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "first buy should create a long position",
    );

    create_order_or_panic(
        limit_order("buyer-2", OrderSide::Buy, 100, 2),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "resting bid should be accepted",
    );
    create_order_or_panic(
        limit_order("buyer-1", OrderSide::Sell, 100, 2),
        &mut orders,
        &mut book,
        &mut positions,
        &mut fills,
        &balance_tx,
        "sell should reduce the existing long position",
    );

    let buyer_position = positions
        .get("buyer-1")
        .expect("buyer should still have a reduced long position");
    assert_eq!(buyer_position.size, 3);
    assert_eq!(buyer_position.average_entry_price, 100);
    assert_eq!(buyer_position.margin, 30);
    assert_eq!(buyer_position.liquidation_price, 90);

    drop(balance_tx);
    balance_thread.join().unwrap();
}
