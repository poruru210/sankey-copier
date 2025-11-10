# ZeroMQ Current Implementation Analysis
## Complete Architecture Assessment

**Created**: 2025-11-06
**Status**: Analysis Complete
**Source**: Deep codebase investigation using Explore agent

---

## Executive Summary

The SANKEY Copier system has a **3-port ZeroMQ architecture** with a dedicated CONFIG channel already implemented. The infrastructure is solid, but the ConfigMessage contains only minimal data. This document provides a complete analysis of what exists and what needs to be enhanced.

---

## 1. ZeroMQ Port Configuration

### Port Assignments (config.toml)

```toml
[zeromq]
receiver_port = 5555      # PULL socket - receives control messages from EAs
sender_port = 5556        # PUB socket - publishes trade signals to slaves
config_sender_port = 5557 # PUB socket - publishes configuration to slaves
timeout_seconds = 30
```

### Architecture Overview

| Port | Type | Direction | Purpose | Messages |
|------|------|-----------|---------|----------|
| **5555** | PULL | EA → Server | Control | Register, Unregister, Heartbeat, TradeSignal |
| **5556** | PUB | Server → Slave | Trades | TradeSignal (topic: master_account) |
| **5557** | PUB | Server → Slave | Config | ConfigMessage (topic: slave account_id) |

---

## 2. CONFIG Channel - Current Implementation

### ConfigMessage Structure (EXISTING)

**File**: `rust-server/src/models/connection.rs:116-124`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigMessage {
    pub account_id: String,        // Slave EA identifier (PUB/SUB topic)
    pub master_account: String,    // Master to copy from
    pub trade_group_id: String,    // Trade group subscription
    pub timestamp: DateTime<Utc>,
}
```

### ✅ What IS Being Sent (4 fields)

```json
{
  "account_id": "SLAVE_001",
  "master_account": "MASTER_001",
  "trade_group_id": "MASTER_001",
  "timestamp": "2025-01-15T10:30:00Z"
}
```

### ❌ What is NOT Being Sent (10+ fields)

From `CopySettings` struct:
- `id: i32`
- `enabled: bool` ← **Critical**
- `lot_multiplier: Option<f64>` ← **Critical**
- `reverse_trade: bool` ← **Critical**
- `symbol_mappings: Vec<SymbolMapping>` ← **Critical**
- `filters.allowed_symbols: Option<Vec<String>>` ← **Critical**
- `filters.blocked_symbols: Option<Vec<String>>` ← **Critical**
- `filters.allowed_magic_numbers: Option<Vec<i32>>` ← **Critical**
- `filters.blocked_magic_numbers: Option<Vec<i32>>` ← **Critical**

**Impact**: EA cannot make filtering decisions. Server must transform all signals.

---

## 3. CONFIG Distribution Trigger Points

### ✅ Currently Implemented

**File**: `rust-server/src/api/mod.rs:48-58`

```rust
async fn send_config_to_ea(state: &AppState, settings: &CopySettings) {
    let config = ConfigMessage {
        account_id: settings.slave_account.clone(),
        master_account: settings.master_account.clone(),
        trade_group_id: settings.master_account.clone(),
        timestamp: chrono::Utc::now(),
    };
    state.config_sender.send_config(&config).await;
}
```

**Triggered by**:
1. **POST** `/api/settings` - Create new copy settings (line 145)
2. **PUT** `/api/settings/:id` - Update existing settings (line 179)

### ❌ NOT Implemented

**EA Registration Flow**:
```rust
// In message_handler.rs handle_register()
// Currently: Just registers connection
// Missing: Look up settings and send CONFIG
```

**EA should receive CONFIG when**:
1. ✅ Admin creates settings via Web UI
2. ✅ Admin updates settings via Web UI
3. ❌ EA connects/reconnects (Registration)
4. ❌ EA sends heartbeat after long disconnect
5. ❌ EA explicitly requests config

---

## 4. MT5 Slave EA - CONFIG Reception

### ✅ Fully Implemented

**File**: `mql/MT5/Slave/SankeyCopierSlave.mq5`

**Socket Setup** (lines 31-43, 89-118):
```cpp
//--- Input parameters
input string TradeServerAddress = "tcp://localhost:5556";   // Port 5556
input string ConfigServerAddress = "tcp://localhost:5557";  // Port 5557
input string AccountID = "SLAVE_001";

//--- Global variables
int g_zmq_trade_socket = -1;    // Trade signal reception
int g_zmq_config_socket = -1;   // Configuration reception
string g_current_master = "";   // Current master account
string g_trade_group_id = "";   // Current trade group
```

**CONFIG Socket Init**:
```cpp
// Create SUB socket for config
g_zmq_config_socket = zmq_socket_create(g_zmq_context, ZMQ_SUB);
zmq_socket_connect(g_zmq_config_socket, ConfigServerAddress);

// Subscribe to config for this account
zmq_socket_subscribe(g_zmq_config_socket, AccountID);
```

**Message Processing** (lines 498-536):
```cpp
void ProcessConfigMessage(string json) {
    string new_master = GetJsonValue(json, "master_account");
    string new_group = GetJsonValue(json, "trade_group_id");

    if(new_master != g_current_master) {
        Print("=== Configuration Update ===");
        Print("Master: ", g_current_master, " -> ", new_master);

        g_current_master = new_master;
        g_trade_group_id = new_group;

        // Dynamic resubscription
        zmq_socket_subscribe(g_zmq_trade_socket, g_trade_group_id);
        Print("Subscribed to trade group: ", g_trade_group_id);
    }
}
```

**JSON Parser** (lines 340-374):
```cpp
string GetJsonValue(string json, string key) {
    // Simple key-value extraction
    // Handles quoted strings, numbers, booleans
    // Stops at comma or closing brace
}
```

**Status**: ✅ **PRODUCTION READY** for current minimal ConfigMessage

**Needs Enhancement**: Parse additional fields when ConfigMessage is extended

---

## 5. MT4 Slave EA - CONFIG Reception

### ❌ NOT Implemented

**File**: `mql/MT4/Slave/SankeyCopierSlave.mq4`

**Issues Identified**:

1. **Only 1 Socket** (should be 2):
   ```cpp
   int g_zmq_socket = -1;  // Only trade socket
   ```

2. **Wrong Connection Pattern** (line 67):
   ```cpp
   zmq_socket_bind(g_zmq_socket, ServerAddress);  // WRONG: Should be CONNECT
   ```

3. **No CONFIG Socket**:
   - Missing `g_zmq_config_socket`
   - Missing connection to port 5557
   - Missing subscription logic

4. **No CONFIG Processing**:
   - No `ProcessConfigMessage()`
   - No JSON parsing for config
   - No dynamic subscription

5. **No Topic-Based Subscription**:
   - Subscribes to ALL messages instead of master_account topic

**Status**: ❌ **NOT FUNCTIONAL** for CONFIG channel

**Required Work**:
- Add second socket
- Fix bind → connect
- Port CONFIG logic from MT5 version
- Add JSON parsing

---

## 6. Message Types - Complete Inventory

### Control Messages (Port 5555 - PUSH/PULL)

#### 1. RegisterMessage
```rust
pub struct RegisterMessage {
    pub account_id: String,
    pub ea_type: EaType,        // Master | Slave
    pub platform: Platform,      // MT4 | MT5
    pub account_number: i64,
    pub broker: String,
    pub account_name: String,
    pub server: String,
    pub balance: f64,
    pub equity: f64,
    pub currency: String,
    pub leverage: i32,
    pub timestamp: DateTime<Utc>,
}
```
**Sent by**: Master & Slave EA on initialization
**Purpose**: Register with server

#### 2. UnregisterMessage
```rust
pub struct UnregisterMessage {
    pub account_id: String,
    pub timestamp: DateTime<Utc>,
}
```
**Sent by**: EA on shutdown
**Purpose**: Clean disconnect

#### 3. HeartbeatMessage
```rust
pub struct HeartbeatMessage {
    pub account_id: String,
    pub balance: f64,
    pub equity: f64,
    pub open_positions: Option<i32>,
    pub timestamp: DateTime<Utc>,
}
```
**Sent by**: EA every 30 seconds
**Purpose**: Keep-alive, status update

#### 4. TradeSignal
```rust
pub struct TradeSignal {
    pub action: TradeAction,      // Open | Close | Modify
    pub ticket: i64,
    pub symbol: String,
    pub order_type: OrderType,    // Buy, Sell, BuyLimit, etc.
    pub lots: f64,
    pub open_price: f64,
    pub stop_loss: Option<f64>,
    pub take_profit: Option<f64>,
    pub magic_number: i32,
    pub comment: String,
    pub timestamp: DateTime<Utc>,
    pub source_account: String,
}
```
**Sent by**: Master EA on trade events
**Purpose**: Signal trades to copy

### Trade Distribution (Port 5556 - PUB/SUB)

**TradeSignal** (same as above)
- Published with topic = `master_account`
- Slave EAs subscribe to their configured master's account_id

### Configuration Distribution (Port 5557 - PUB/SUB)

**ConfigMessage** (minimal, current)
- Published with topic = `slave account_id`
- Slave EA subscribes to its own account_id
- Contains: account_id, master_account, trade_group_id, timestamp

---

## 7. Data Flow Diagrams

### Current: API-Triggered CONFIG Distribution

```
┌──────────┐
│  Web UI  │
└────┬─────┘
     │ POST/PUT /api/settings
     ▼
┌─────────────────┐
│  Rust Server    │
│  - Save to DB   │
└────┬────────────┘
     │
     ▼
┌─────────────────────┐
│ send_config_to_ea() │
│ - Build CONFIG      │
└────┬────────────────┘
     │ PUB on port 5557
     │ topic=account_id
     ▼
┌──────────────────┐
│  MT5 Slave EA    │
│  - Receives      │
│  - Parses JSON   │
│  - Resubscribes  │
└──────────────────┘
```

### Missing: Registration-Triggered CONFIG

```
┌──────────────┐
│  Slave EA    │
│  OnInit()    │
└──────┬───────┘
       │ Register (port 5555)
       ▼
┌────────────────────────┐
│  Rust Server           │
│  handle_register()     │
└──────┬─────────────────┘
       │
       ▼
┌────────────────────────┐
│  Database Query        │
│  - Find settings where │
│    slave_account = ID  │
└──────┬─────────────────┘
       │
       ▼
┌────────────────────────┐
│  send_config_to_ea()   │  ← NOT IMPLEMENTED
│  - Send initial config │
└──────┬─────────────────┘
       │ PUB on port 5557
       ▼
┌────────────────┐
│  Slave EA      │
│  Receives      │
└────────────────┘
```

---

## 8. CopySettings Structure (Database Model)

### Complete Structure

**File**: `rust-server/src/models/mod.rs`

```rust
pub struct CopySettings {
    pub id: i32,
    pub enabled: bool,
    pub master_account: String,
    pub slave_account: String,
    pub lot_multiplier: Option<f64>,
    pub reverse_trade: bool,
    pub symbol_mappings: Vec<SymbolMapping>,
    pub filters: TradeFilters,
}

pub struct SymbolMapping {
    pub source_symbol: String,
    pub target_symbol: String,
}

pub struct TradeFilters {
    pub allowed_symbols: Option<Vec<String>>,
    pub blocked_symbols: Option<Vec<String>>,
    pub allowed_magic_numbers: Option<Vec<i32>>,
    pub blocked_magic_numbers: Option<Vec<i32>>,
}
```

### Storage

**SQLite Database**: `copy_settings` table
- Stores all fields
- Queried by Web UI via REST API
- Used by server to transform TradeSignals

**Currently**:
- Server applies lot_multiplier, reverse_trade, filters
- Server transforms symbols
- Slave EA receives pre-transformed signals
- **EA has no visibility into configuration**

---

## 9. What Needs to Be Enhanced

### Priority 1: Extend ConfigMessage ⭐⭐⭐

**Add fields to ConfigMessage**:
```rust
pub struct ConfigMessage {
    // Existing
    pub account_id: String,
    pub master_account: String,
    pub trade_group_id: String,
    pub timestamp: DateTime<Utc>,

    // NEW: Full configuration
    pub enabled: bool,
    pub lot_multiplier: Option<f64>,
    pub reverse_trade: bool,
    pub symbol_mappings: Vec<SymbolMapping>,
    pub filters: TradeFilters,

    // NEW: Version control
    pub config_version: u32,
}
```

**Files to modify**:
- `rust-server/src/models/connection.rs` - Extend struct
- `rust-server/src/api/mod.rs` - Update `send_config_to_ea()`
- `rust-server/src/zeromq/mod.rs` - Update serialization
- `mql/MT5/Slave/SankeyCopierSlave.mq5` - Parse new fields
- `mql/MT4/Slave/SankeyCopierSlave.mq4` - Implement CONFIG support

**Benefit**: EA can filter trades locally, reducing server load

---

### Priority 2: Registration-Triggered CONFIG ⭐⭐⭐

**Add to `handle_register()`**:
```rust
async fn handle_register(&self, msg: RegisterMessage) {
    let account_id = msg.account_id.clone();

    // Existing: Register connection
    self.connection_manager.register_ea(msg).await;

    // NEW: Send config if this is a slave
    if let Ok(settings_list) = self.db.list_copy_settings().await {
        for setting in settings_list {
            if setting.slave_account == account_id && setting.enabled {
                self.send_config_to_ea(&setting).await;
                tracing::info!("Sent initial config to {}", account_id);
            }
        }
    }

    // Existing: Broadcast to WebSocket clients
    let _ = self.broadcast_tx.send(...);
}
```

**Files to modify**:
- `rust-server/src/message_handler.rs` - Add config lookup in handle_register

**Benefit**: EA receives configuration immediately on startup

---

### Priority 3: Fix MT4 Slave EA ⭐⭐

**Required changes**:
1. Add second socket (g_zmq_config_socket)
2. Change `zmq_socket_bind()` → `zmq_socket_connect()`
3. Subscribe to config topic (AccountID)
4. Implement `ProcessConfigMessage()`
5. Port JSON parser from MT5 version
6. Add dynamic trade group subscription

**Files to modify**:
- `mql/MT4/Slave/SankeyCopierSlave.mq4`

**Benefit**: MT4 and MT5 feature parity

---

### Priority 4: Add Config Acknowledgment ⭐

**New message type**:
```rust
pub struct ConfigAckMessage {
    pub account_id: String,
    pub status: String,           // "success" | "error"
    pub config_version: u32,
    pub error_message: Option<String>,
    pub timestamp: DateTime<Utc>,
}
```

**EA sends after receiving CONFIG**:
```cpp
void ProcessConfigMessage(string json) {
    // ... parse and apply config ...

    // Send acknowledgment back to server
    string ack_json = StringFormat(
        "{\"message_type\":\"ConfigAck\",\"account_id\":\"%s\",\"status\":\"success\"}",
        AccountID
    );
    zmq_socket_send(g_zmq_control_socket, ack_json);
}
```

**Server tracks delivery**:
- Log successful config distribution
- Alert if no ACK received within timeout
- Display config sync status in Web UI

**Benefit**: Monitoring and troubleshooting

---

### Priority 5: Add Config Request Mechanism ⭐

**New message type**:
```rust
pub enum MessageType {
    Register(RegisterMessage),
    Unregister(UnregisterMessage),
    Heartbeat(HeartbeatMessage),
    TradeSignal(TradeSignal),
    ConfigRequest(ConfigRequestMessage),  // NEW
}

pub struct ConfigRequestMessage {
    pub account_id: String,
    pub timestamp: DateTime<Utc>,
}
```

**EA can request config**:
```cpp
void RequestConfig() {
    string request_json = StringFormat(
        "{\"message_type\":\"ConfigRequest\",\"account_id\":\"%s\"}",
        AccountID
    );
    zmq_socket_send(g_zmq_control_socket, request_json);
    Print("Requested configuration from server");
}
```

**Server responds**:
```rust
async fn handle_config_request(&self, msg: ConfigRequestMessage) {
    // Look up settings for this account
    // Send CONFIG message via port 5557
}
```

**Benefit**: EA can recover from lost config state

---

## 10. Architecture Diagram - Complete System

```
                    ┌─────────────────────────────────────┐
                    │      RUST SERVER (localhost)        │
                    │                                     │
┌───────────────────┤  Port 5555 (PULL Socket)           │
│                   │  Control & Trade Signal Input      │
│  PUSH messages    └──────────▲──────────────────────────┘
│  from EAs                    │
│                              │
│  ┌────────────┐         ┌────────────┐
│  │ Master EA  │         │ Slave EA   │
│  │            │         │            │
│  │ PUSH→5555  │         │ PUSH→5555  │
│  │ (Register, │         │ (Register, │
│  │ Heartbeat, │         │ Heartbeat) │
│  │ TradeSignal)         │            │
│  └────────────┘         └────────────┘
│
│
├──────────────────────────────────────────────────────────┐
│                                                          │
│  ┌────────────────────────────────────────────────────┐ │
│  │  Port 5556 (PUB Socket)                            │ │
│  │  Trade Signal Distribution                         │ │
│  │  Topic = master_account                            │ │
│  └──────────────────────┬─────────────────────────────┘ │
│                         │                                │
│                         │ PUB/SUB                        │
│                         │ TradeSignal                    │
│                         ▼                                │
│                    ┌────────────┐                        │
│                    │ Slave EA   │                        │
│                    │ SUB←5556   │                        │
│                    │ (topic:    │                        │
│                    │  master_id)│                        │
│                    └────────────┘                        │
│                                                          │
├──────────────────────────────────────────────────────────┤
│                                                          │
│  ┌────────────────────────────────────────────────────┐ │
│  │  Port 5557 (PUB Socket)                            │ │
│  │  Configuration Distribution                        │ │
│  │  Topic = slave account_id                          │ │
│  └──────────────────────┬─────────────────────────────┘ │
│                         │                                │
│                         │ PUB/SUB                        │
│                         │ ConfigMessage                  │
│                         ▼                                │
│                    ┌────────────┐                        │
│                    │ Slave EA   │                        │
│                    │ SUB←5557   │                        │
│                    │ (topic:    │                        │
│                    │  account_id│                        │
│                    └────────────┘                        │
│                                                          │
└──────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────┐
│              WEB UI / REST API (Port 8080)               │
│                                                          │
│  POST /api/settings → Triggers CONFIG send (port 5557)  │
│  PUT /api/settings/:id → Triggers CONFIG send           │
└──────────────────────────────────────────────────────────┘
```

---

## 11. Summary

### ✅ What's Working Well

1. **Solid 3-Port Architecture**
   - Separate channels for control, trades, config
   - PUB/SUB with topic-based routing
   - Clean separation of concerns

2. **CONFIG Channel Infrastructure**
   - Dedicated port (5557)
   - ZmqConfigSender implemented
   - PUB/SUB pattern working

3. **MT5 Slave EA CONFIG Support**
   - Fully functional CONFIG reception
   - JSON parsing
   - Dynamic subscription
   - Production-ready for current ConfigMessage

4. **API-Triggered Updates**
   - Create/update settings → send CONFIG
   - Working as designed

### ❌ Critical Gaps

1. **Incomplete ConfigMessage** (⭐⭐⭐ Priority 1)
   - Only 4 fields sent, 10+ missing
   - No lot_multiplier, reverse_trade, filters
   - EA cannot make intelligent decisions

2. **No Registration-Triggered CONFIG** (⭐⭐⭐ Priority 2)
   - EA connects but receives no config
   - Must wait for admin to create/update settings
   - Poor user experience

3. **MT4 Slave EA Missing CONFIG** (⭐⭐ Priority 3)
   - Only one socket
   - Wrong connection pattern
   - No config processing
   - MT4/MT5 inconsistency

4. **No Config Feedback** (⭐ Priority 4)
   - No acknowledgment from EA
   - No delivery confirmation
   - Difficult to troubleshoot

5. **No Config Request** (⭐ Priority 5)
   - EA cannot request config
   - No recovery mechanism
   - Must restart to get config

---

## 12. Recommended Implementation Order

### Phase 1: Extend ConfigMessage (Week 1)
1. Add fields to ConfigMessage struct
2. Update send_config_to_ea() to populate all fields
3. Update MT5 Slave EA to parse new fields
4. Test with sample data

### Phase 2: Registration-Triggered CONFIG (Week 1-2)
1. Modify handle_register() to lookup settings
2. Send CONFIG on registration
3. Add database query for settings by slave_account
4. Test EA startup flow

### Phase 3: Fix MT4 Slave EA (Week 2)
1. Add second socket
2. Fix connection pattern
3. Port CONFIG logic from MT5
4. Test MT4/MT5 parity

### Phase 4: Add Monitoring (Week 3)
1. Implement ConfigAckMessage
2. Add CONFIG request mechanism
3. Add Web UI indicators for config sync status
4. Add logging and alerts

---

**End of Analysis Document**

This document provides a complete picture of the current ZeroMQ implementation and clear priorities for enhancement.
