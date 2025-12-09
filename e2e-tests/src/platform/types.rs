// e2e-tests/src/platform/types.rs
//
// MQL5 Type Definitions
//
// MQL5の型や定数をRustで再現するための定義。
// 命名規則は意図的にMQL5のスタイル（CamelCase, PascalCase）に近づけている場合もありますが、
// 基本的にはRustの慣習に従いつつ、MQL5の構造体名を使用します。

#![allow(non_camel_case_types)]
#![allow(dead_code)]

/// MQL5の初期化戻り値コード
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ENUM_INIT_RETCODE {
    INIT_SUCCEEDED = 0,
    INIT_FAILED = 1,
    INIT_PARAMETERS_INCORRECT = 2,
    INIT_AGENT_NOT_SUITABLE = 3,
}

/// MQL5の非初期化理由コード
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ENUM_DEINIT_REASON {
    REASON_PROGRAM = 0,
    REASON_REMOVE = 1,
    REASON_RECOMPILE = 2,
    REASON_CHARTCHANGE = 3,
    REASON_CHARTCLOSE = 4,
    REASON_PARAMETERS = 5,
    REASON_ACCOUNT = 6,
    REASON_TEMPLATE = 7,
    REASON_INITFAILED = 8,
    REASON_CLOSE = 9,
}

/// MQL5のトレードトランザクションタイプ
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ENUM_TRADE_TRANSACTION_TYPE {
    TRADE_TRANSACTION_ORDER_ADD,
    TRADE_TRANSACTION_ORDER_UPDATE,
    TRADE_TRANSACTION_ORDER_DELETE,
    TRADE_TRANSACTION_DEAL_ADD,
    TRADE_TRANSACTION_DEAL_UPDATE,
    TRADE_TRANSACTION_HISTORY_ADD,
    TRADE_TRANSACTION_HISTORY_UPDATE,
    TRADE_TRANSACTION_HISTORY_DELETE,
    TRADE_TRANSACTION_POSITION,
    TRADE_TRANSACTION_REQUEST,
}

/// MQL5のトレードリクエストアクション
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ENUM_TRADE_REQUEST_ACTIONS {
    TRADE_ACTION_DEAL,
    TRADE_ACTION_PENDING,
    TRADE_ACTION_SLTP,
    TRADE_ACTION_MODIFY,
    TRADE_ACTION_REMOVE,
    TRADE_ACTION_CLOSE_BY,
}

/// MQL5の注文タイプ
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ENUM_ORDER_TYPE {
    ORDER_TYPE_BUY,
    ORDER_TYPE_SELL,
    ORDER_TYPE_BUY_LIMIT,
    ORDER_TYPE_SELL_LIMIT,
    ORDER_TYPE_BUY_STOP,
    ORDER_TYPE_SELL_STOP,
    ORDER_TYPE_BUY_STOP_LIMIT,
    ORDER_TYPE_SELL_STOP_LIMIT,
    ORDER_TYPE_CLOSE_BY,
}

/// MQL5のMqlTick構造体
#[derive(Debug, Clone, Default)]
pub struct MqlTick {
    pub time: i64,          // Time of the last prices update
    pub bid: f64,           // Current Bid price
    pub ask: f64,           // Current Ask price
    pub last: f64,          // Price of the last deal (Last)
    pub volume: u64,        // Volume for the current Last price
    pub time_msc: i64,      // Time of a price last update in milliseconds
    pub flags: u32,         // Tick flags
    pub volume_real: f64,   // Volume for the current Last price with greater accuracy
}

/// MQL5のMqlTradeRequest構造体
#[derive(Debug, Clone, Default)]
pub struct MqlTradeRequest {
    pub action: i32,           // ENUM_TRADE_REQUEST_ACTIONS
    pub magic: u64,            // Expert Advisor ID (magic number)
    pub order: u64,            // Order ticket
    pub symbol: String,        // Trade symbol
    pub volume: f64,           // Requested volume for a deal in lots
    pub price: f64,            // Price
    pub stoplimit: f64,        // StopLimit level of the order
    pub sl: f64,               // Stop Loss level of the order
    pub tp: f64,               // Take Profit level of the order
    pub deviation: u64,        // Maximal possible deviation from the requested price
    pub type_: i32,            // ENUM_ORDER_TYPE
    pub type_filling: i32,     // Order execution type
    pub type_time: i32,        // Order expiration type
    pub expiration: i64,       // Order expiration time (for the orders of ORDER_TIME_SPECIFIED type)
    pub comment: String,       // Order comment
    pub position: u64,         // Position ticket
    pub position_by: u64,      // The ticket of an opposite position
}

/// MQL5のMqlTradeResult構造体
#[derive(Debug, Clone, Default)]
pub struct MqlTradeResult {
    pub retcode: u32,          // Operation return code
    pub deal: u64,             // Deal ticket, if it is performed
    pub order: u64,            // Order ticket, if it is placed
    pub volume: f64,           // Deal volume, confirmed by broker
    pub price: f64,            // Deal price, confirmed by broker
    pub bid: f64,              // Current Bid price
    pub ask: f64,              // Current Ask price
    pub comment: String,       // Broker comment to operation (default: 128 chars)
    pub request_id: u32,       // Request ID set by the terminal during dispatch
    pub retcode_external: u32, // External return code
}

/// MQL5のMqlTradeTransaction構造体
#[derive(Debug, Clone, Default)]
pub struct MqlTradeTransaction {
    pub deal: u64,             // Deal ticket
    pub order: u64,            // Order ticket
    pub symbol: String,        // Trade symbol name
    pub type_: i32,            // ENUM_TRADE_TRANSACTION_TYPE
    pub order_type: i32,       // ENUM_ORDER_TYPE
    pub order_state: i32,      // ENUM_ORDER_STATE
    pub deal_type: i32,        // ENUM_DEAL_TYPE
    pub time_type: i32,        // ENUM_ORDER_TYPE_TIME
    pub time_expiration: i64,  // Order expiration time
    pub price: f64,            // Price
    pub price_trigger: f64,    // Stop limit order activation price
    pub price_sl: f64,         // Stop Loss level
    pub price_tp: f64,         // Take Profit level
    pub volume: f64,           // Volume in lots
    pub position: u64,         // Position ticket
    pub position_by: u64,      // Opposite position ticket
}
