# Perps_v1

`perps_v1` is a simple perpetual futures exchange backend written in Rust.

This is a learning project so the engine will not be that fast. But still I am happy to build this. It made me fall in love with rust.

The project is organized around one idea:

- HTTP handlers receive requests.
- Handlers forward exchange work to the engine over channels.
- The engine owns the order book, orders, positions, fills, and balance updates.
- Tests can call the engine directly without going through Actix.



## Current Scope

Right now the app includes:

- sign up / sign in
- wallet on-ramp through the engine
- create order
- delete order
- get depth
- mark-price websocket feed
- liquidation entrypoint
- engine-only tests for matching and position flow

This project is still a learning-oriented implementation, so some parts are intentionally simple and some behaviors are still evolving.

## Tech Stack

- Rust 2024
- `actix-web` for HTTP
- `tokio` for async runtime and channels
- `tokio-tungstenite` for websocket market feed
- `serde` / `serde_json` for request and response types
- `uuid` for IDs
- `jsonwebtoken` for auth token creation

## Project Structure

Important folders and files:

- [src/main.rs](/Users/yash/Developer/S30/perps_v1/src/main.rs:1): app entrypoint, route wiring, engine startup
- [src/lib.rs](/Users/yash/Developer/S30/perps_v1/src/lib.rs:1): exports modules for tests and the binary
- [src/controllers/auth.rs](/Users/yash/Developer/S30/perps_v1/src/controllers/auth.rs:1): sign up and sign in handlers
- [src/controllers/exchange.rs](/Users/yash/Developer/S30/perps_v1/src/controllers/exchange.rs:1): on-ramp, create order, delete order, depth handlers
- [src/store/store.rs](/Users/yash/Developer/S30/perps_v1/src/store/store.rs:1): shared app state for HTTP layer
- [src/types/types.rs](/Users/yash/Developer/S30/perps_v1/src/types/types.rs:1): request payloads and balance request messages
- [src/engine/engine.rs](/Users/yash/Developer/S30/perps_v1/src/engine/engine.rs:1): engine router and engine worker threads
- [src/engine/create_order.rs](/Users/yash/Developer/S30/perps_v1/src/engine/create_order.rs:1): main order creation and matching entrypoint
- [src/engine/core_matching.rs](/Users/yash/Developer/S30/perps_v1/src/engine/core_matching.rs:1): matching logic for buys and sells
- [src/engine/position.rs](/Users/yash/Developer/S30/perps_v1/src/engine/position.rs:1): position creation and updates after fills
- [src/engine/check_balance.rs](/Users/yash/Developer/S30/perps_v1/src/engine/check_balance.rs:1): in-memory balance actor
- [src/engine/delete_order.rs](/Users/yash/Developer/S30/perps_v1/src/engine/delete_order.rs:1): cancel flow
- [src/engine/get_depth.rs](/Users/yash/Developer/S30/perps_v1/src/engine/get_depth.rs:1): depth snapshot logic
- [src/engine/liquidation.rs](/Users/yash/Developer/S30/perps_v1/src/engine/liquidation.rs:1): liquidation trigger path
- [src/websocket/connection.rs](/Users/yash/Developer/S30/perps_v1/src/websocket/connection.rs:1): external price stream hookup
- [tests/auth.rs](/Users/yash/Developer/S30/perps_v1/tests/auth.rs:1): auth integration tests
- [tests/engine_flow.rs](/Users/yash/Developer/S30/perps_v1/tests/engine_flow.rs:1): engine-only flow tests

## Runtime Architecture

At startup:

1. `main()` creates a Tokio channel for engine requests.
2. `connect_stream()` starts a websocket task for mark-price updates.
3. `run_engine()` starts the internal engine workers.
4. Actix starts listening on `127.0.0.1:8080`.

There are two main layers:

- HTTP layer
  - validates/parses requests
  - sends `EngineRequest` messages
  - waits on a oneshot reply

- Engine layer
  - owns exchange state
  - processes matching and balances
  - updates positions and fills

## Shared App State

The HTTP layer stores a very small shared state in [src/store/store.rs](/Users/yash/Developer/S30/perps_v1/src/store/store.rs:1):

- `users: Mutex<HashMap<String, User>>`
- `sender: Sender<EngineRequest>`

So the web server does not own the order book or positions.
It only owns:

- the in-memory user map
- a channel into the engine

## HTTP Endpoints

Current routes are defined in [src/main.rs](/Users/yash/Developer/S30/perps_v1/src/main.rs:1).

### Auth

- `POST /api/signup`
- `POST /api/signin`

Handled by [src/controllers/auth.rs](/Users/yash/Developer/S30/perps_v1/src/controllers/auth.rs:1).

`signup`:

- reads `username` and `password`
- checks the in-memory user map
- creates a `User` with a generated UUID

`signin`:

- checks user existence
- checks password equality
- returns a JWT and `user_id`

### Exchange

- `POST /onramp/`
- `POST /order/create`
- `POST /order/delete`
- `POST /depth/`

Handled by [src/controllers/exchange.rs](/Users/yash/Developer/S30/perps_v1/src/controllers/exchange.rs:1).

Each handler:

1. creates a `oneshot` channel
2. sends an `EngineRequest`
3. waits for the engine reply
4. converts the result into an HTTP response

## Engine Model

The engine communicates through the `EngineRequest` enum in [src/engine/types.rs](/Users/yash/Developer/S30/perps_v1/src/engine/types.rs:1).

Main request types:

- `CreateOrder`
- `DeleteOrderData`
- `GetDepth`
- `UpdateBalance`
- `MarkPriceUpdate`

The engine keeps these in-memory structures:

- `OrderBook`
  - `bids: BTreeMap<u64, VecDeque<RestingOrder>>`
  - `asks: BTreeMap<u64, VecDeque<RestingOrder>>`
- `orders: HashMap<String, Order>`
- `positions: HashMap<String, Position>`
- `fills: HashMap<String, Vec<Fill>>`

### Why `BTreeMap + VecDeque`?

This gives the engine:

- price ordering from `BTreeMap`
- FIFO queue behavior at the same price with `VecDeque`

So the intended matching model is:

- best price first
- older orders first at the same price

## Engine Flow

The top-level engine flow lives in [src/engine/engine.rs](/Users/yash/Developer/S30/perps_v1/src/engine/engine.rs:1).

There are three important pieces:

### 1. Engine router task

This async task receives `EngineRequest` messages from the HTTP layer and websocket layer.

It:

- handles `UpdateBalance` by talking to the balance actor
- routes symbol-specific requests to a market worker

Right now:

- BTC requests go to the BTC worker
- SOL route exists in structure, but the SOL worker path is not fully built out

### 2. Balance actor

Defined in [src/engine/check_balance.rs](/Users/yash/Developer/S30/perps_v1/src/engine/check_balance.rs:1).

It owns a private `HashMap<String, Balances>` and processes:

- `AddBalance`
- `LockMargin`
- `ReleaseMargin`
- `GetBalance`
- `ReduceBalance`

This is important because the matching engine does not mutate balances directly. It sends balance messages to this actor.

### 3. BTC engine worker

The BTC worker owns:

- the BTC order book
- BTC orders
- BTC fills
- BTC positions

This worker is the actual matching engine for BTC requests.

## Create Order Flow

The main entrypoint is [src/engine/create_order.rs](/Users/yash/Developer/S30/perps_v1/src/engine/create_order.rs:1).

High-level flow:

1. Generate `order_id`
2. Convert incoming order side into signed exposure
3. Run `risk_engine(...)`
4. If needed, fetch user balance from balance actor
5. Check required margin
6. Insert the order into `orders`
7. Dispatch to:
   - `handle_limit_order(...)`
   - `handle_market_order(...)`

### Risk check

The helper is in [src/engine/helper.rs](/Users/yash/Developer/S30/perps_v1/src/engine/helper.rs:1).

The engine checks whether the order increases exposure enough that margin validation should run.

### Margin check

Required margin is calculated as:

`size * price / leverage`

If available balance is not enough, the engine returns:

- `EngineError::NotEnoughBalance`

### Order insertion

Before matching, the engine stores a full `Order` entry with:

- side
- size
- price
- leverage
- remaining quantity
- filled quantity
- created time

## Limit Order Flow

Limit order handling is split by side.

### Incoming buy limit order

Inside `handle_limit_order(...)`:

1. Try to match against best asks through `core_buy_logic(...)`
2. If any fill happened, update positions through `check_positions(...)`
3. Build a `RestingOrder`
4. Add remaining quantity to bids
5. Return `CreateOrderResponse`

### Incoming sell limit order

Inside `handle_limit_order(...)`:

1. Try to match against best bids through `core_sell_logic(...)`
2. If any fill happened, update positions through `check_positions(...)`
3. Build a `RestingOrder`
4. Add remaining quantity to asks
5. Return `CreateOrderResponse`

### Status meanings

Responses return one of:

- `Open`
- `PartiallyFilled`
- `Filled`

based on:

- total incoming size
- remaining quantity after matching

## Matching Logic

Matching code lives in [src/engine/core_matching.rs](/Users/yash/Developer/S30/perps_v1/src/engine/core_matching.rs:1).

### `core_buy_logic(...)`

Incoming buy order:

- looks at the best ask price
- matches while `ask_price <= incoming_price`
- creates fill records for both sides
- updates `filled_qty` and `remaining_qty`
- removes exhausted resting asks

### `core_sell_logic(...)`

Incoming sell order:

- looks at the best bid price
- matches while `bid_price >= incoming_price`
- creates fill records for both sides
- updates `filled_qty` and `remaining_qty`
- removes exhausted resting bids

Each match generates `Fill` records that later feed position updates.

## Position Flow

Position logic lives in [src/engine/position.rs](/Users/yash/Developer/S30/perps_v1/src/engine/position.rs:1).

`check_positions(...)` runs after fills happen.

It reads:

- the fills created for the order
- the order that caused those fills
- the existing position for that user, if any

Then it applies one of these cases:

### 1. Fresh position

If the user had no position:

- create a new `Position`
- compute average entry
- compute margin
- compute liquidation price

### 2. Same-side increase

If the new order is on the same side as the current position:

- increase size
- compute weighted average entry
- recalculate margin
- recalculate liquidation price

### 3. Opposite-side reduction

If the new order reduces but does not flip the position:

- reduce position size
- keep the side the same
- recalculate remaining margin
- recalculate liquidation price

### 4. Full close

If the new order completely closes the position:

- calculate PnL
- add or reduce balance
- release margin
- remove position

### 5. Flip position

If the new order crosses through zero:

- release old margin
- create the new opposite-side position
- lock new margin

## Delete Order Flow

Cancellation logic is in [src/engine/delete_order.rs](/Users/yash/Developer/S30/perps_v1/src/engine/delete_order.rs:1).

Flow:

1. find the order
2. verify status is cancellable
3. compute remaining margin to reclaim
4. release margin through balance actor
5. remove the resting order from the book
6. mark the order as cancelled

## Depth Flow

Depth logic is in [src/engine/get_depth.rs](/Users/yash/Developer/S30/perps_v1/src/engine/get_depth.rs:1).

It returns:

- top ask levels
- top bid levels
- aggregated remaining quantity per price

The output is built from the in-memory order book.

## Mark Price and Liquidation

The websocket code is in [src/websocket/connection.rs](/Users/yash/Developer/S30/perps_v1/src/websocket/connection.rs:1).

Current shape:

- opens a Binance futures websocket connection
- subscribes to mark price updates
- forwards `EngineRequest::MarkPriceUpdate` into the engine

Liquidation logic is in [src/engine/liquidation.rs](/Users/yash/Developer/S30/perps_v1/src/engine/liquidation.rs:1).

Flow:

1. iterate over positions
2. check whether liquidation price has been crossed
3. create liquidation market orders
4. feed them back into `create_order(...)`

## Tests


### Engine flow tests

[tests/engine_flow.rs](/Users/yash/Developer/S30/perps_v1/tests/engine_flow.rs:1)

These are the best place to understand engine behavior because they bypass HTTP and call the engine directly.

They cover flows like:

- resting limit order
- full match
- partial fill
- insufficient balance
- position creation
- same-side position increase
- opposite-side position reduction

Run with:

```bash
cargo test --test engine_flow -- --nocapture
```

## Local Run

Start the server:

```bash
cargo run
```

The app listens on:

```text
127.0.0.1:8080
```

If you want sign-in token creation to work, set:

```bash
JWT_SECRET=your-secret
```

You can place this in a `.env` file because `dotenv` is loaded in `main()`.

## Example Request Shapes

### Sign up

```json
{
  "username": "alice",
  "password": "password123"
}
```

### Sign in

```json
{
  "username": "alice",
  "password": "password123"
}
```

### On-ramp

```json
{
  "user_id": "user-uuid",
  "amount": 10000
}
```

### Create order

```json
{
  "user_id": "user-uuid",
  "order_type": "Limit",
  "order_side": "Buy",
  "symbol": "BTC",
  "size": 2,
  "price": 100,
  "leverage": 10,
  "slippage": 0
}
```

### Delete order

```json
{
  "order_id": "order-uuid",
  "user_id": "user-uuid",
  "symbol": "BTC"
}
```

### Get depth

```json
{
  "symbol": "BTC"
}
```

## Mental Model For New Readers

If you are trying to understand the code quickly, this order works well:

1. Read [src/main.rs](/Users/yash/Developer/S30/perps_v1/src/main.rs:1)
2. Read [src/controllers/exchange.rs](/Users/yash/Developer/S30/perps_v1/src/controllers/exchange.rs:1)
3. Read [src/engine/types.rs](/Users/yash/Developer/S30/perps_v1/src/engine/types.rs:1)
4. Read [src/engine/engine.rs](/Users/yash/Developer/S30/perps_v1/src/engine/engine.rs:1)
5. Read [src/engine/create_order.rs](/Users/yash/Developer/S30/perps_v1/src/engine/create_order.rs:1)
6. Read [src/engine/core_matching.rs](/Users/yash/Developer/S30/perps_v1/src/engine/core_matching.rs:1)
7. Read [src/engine/position.rs](/Users/yash/Developer/S30/perps_v1/src/engine/position.rs:1)
8. Run [tests/engine_flow.rs](/Users/yash/Developer/S30/perps_v1/tests/engine_flow.rs:1)

That gives you the cleanest top-down view of how an order enters the system, matches, and changes positions.

## Current Notes

A few practical notes while working in this codebase:

- the engine is fully in-memory
- balances are actor-managed, not database-backed
- auth users are also in-memory
- tests are the best way to understand expected engine behavior
- engine-only tests are more useful than HTTP tests for matching logic


