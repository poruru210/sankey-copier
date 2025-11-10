# Configuration Distribution Architecture
## Server-Master Design (SQLite as Single Source of Truth)

**Created**: 2025-11-06
**Status**: Design Phase
**Priority**: High - Core Architecture

---

## 1. Design Principle

**"Server is the single source of truth"**

- SQLite database is the master for all configuration
- Web UI always fetches from server (no local caching)
- EA receives configuration from server on startup and changes
- No localStorage, no EA-side configuration files for copy settings

---

## 2. Architecture Overview

```
┌─────────────┐                   ┌──────────────────────┐                   ┌─────────────┐
│   Web UI    │◄─── REST API ────►│   Rust Server        │◄──── ZeroMQ ─────►│  MT4/MT5 EA │
│             │                   │   + SQLite (Master)   │                   │             │
│ - View      │                   │                      │                   │ - Apply     │
│ - Create    │                   │ - Store settings     │                   │   settings  │
│ - Edit      │                   │ - Distribute config  │                   │ - Execute   │
│ - Delete    │                   │ - Validate changes   │                   │   trades    │
└─────────────┘                   └──────────────────────┘                   └─────────────┘
```

---

## 3. Data Flow Scenarios

### 3.1 User Creates New Copy Setting

```
1. [Web UI]
   User fills out form:
   - Master account
   - Slave account
   - Lot multiplier
   - Reverse trade
   - Filters, symbol mappings

2. [Web UI → Rust Server]
   POST /api/settings
   Body: {
     master_account: "12345",
     slave_account: "67890",
     lot_multiplier: 1.5,
     reverse_trade: false,
     filters: { ... },
     symbol_mappings: [ ... ]
   }

3. [Rust Server]
   - Validate input
   - Insert into SQLite
   - Generate unique ID
   - Return success + new setting ID

4. [Rust Server → EA]
   - Identify which slave EA needs this config
   - Send via ZeroMQ CONFIG channel:
     ConfigMessage {
       account_id: "67890",
       enabled: true,
       master_account: "12345",
       lot_multiplier: 1.5,
       reverse_trade: false,
       allowed_symbols: [...],
       blocked_symbols: [...],
       allowed_magic_numbers: [...],
       blocked_magic_numbers: [...],
       symbol_mappings: [...],
     }

5. [EA]
   - Receive CONFIG message
   - Parse and store in global variables/array
   - Log: "Configuration received for account 67890"
   - Start copying trades from master 12345
```

---

### 3.2 User Edits Existing Copy Setting

```
1. [Web UI]
   User clicks gear icon → settings list → clicks setting → modal opens
   User edits parameters (e.g., lot_multiplier: 1.5 → 2.0)

2. [Web UI → Rust Server]
   PUT /api/settings/:id
   Body: {
     lot_multiplier: 2.0,
     ... (all fields)
   }

3. [Rust Server]
   - Validate input
   - Update SQLite record
   - Return success

4. [Rust Server → EA]
   - Send updated CONFIG message via ZeroMQ
   - Same format as create

5. [EA]
   - Receive CONFIG message
   - Update stored configuration
   - Apply new settings immediately to future trades
   - Log: "Configuration updated for account 67890"
```

---

### 3.3 User Deletes Copy Setting

```
1. [Web UI]
   User clicks delete button → confirmation dialog → confirms

2. [Web UI → Rust Server]
   DELETE /api/settings/:id

3. [Rust Server]
   - Fetch setting details (need slave_account ID)
   - Delete from SQLite
   - Return success

4. [Rust Server → EA]
   - Send CONFIG_DISABLE message via ZeroMQ:
     ConfigMessage {
       account_id: "67890",
       enabled: false,
       master_account: null,
       ... (all nulls/defaults)
     }

5. [EA]
   - Receive CONFIG_DISABLE
   - Remove configuration from memory
   - Stop copying trades
   - Log: "Configuration removed for account 67890"
```

---

### 3.4 EA Connects/Reconnects to Server

```
1. [EA]
   - Establish ZeroMQ connection
   - Send HEARTBEAT message with account_id

2. [Rust Server]
   - Receive HEARTBEAT
   - Update connection status in memory
   - Look up SQLite for settings where:
     * master_account = this EA's account_id (if Master EA)
     * slave_account = this EA's account_id (if Slave EA)

3. [Rust Server → EA]
   - Send all relevant CONFIG messages
   - For slave EA: Send its copy settings
   - For master EA: Send list of slaves (optional, for UI feedback)

4. [EA]
   - Receive CONFIG messages
   - Store configurations
   - Start operating based on settings
   - Log: "Synchronized N configurations from server"
```

---

### 3.5 User Views Connection Dashboard

```
1. [Web UI]
   Page load or refresh

2. [Web UI → Rust Server]
   GET /api/connections (list all EAs)
   GET /api/settings (list all copy settings)

3. [Rust Server]
   - Query SQLite
   - Query connection manager (live status)
   - Combine data
   - Return JSON

4. [Web UI]
   - Render dashboard with:
     * Master cards (with status)
     * Slave cards (with status)
     * Connection lines (based on settings)
   - Filter based on sidebar selection
```

---

## 4. ZeroMQ Message Protocol Extension

### Current Messages (Already Implemented)

**HEARTBEAT** (EA → Server):
```json
{
  "type": "HEARTBEAT",
  "account_id": "12345",
  "ea_type": "Master",
  "platform": "MT4",
  "account_number": 12345,
  "broker": "OANDA",
  "server": "OANDAdemo01",
  "balance": 10000.0,
  "equity": 10050.0,
  "currency": "USD",
  "leverage": 100
}
```

**TRADE_SIGNAL** (Master EA → Server):
```json
{
  "type": "TRADE_SIGNAL",
  "account_id": "12345",
  "symbol": "EURUSD",
  "order_type": "Buy",
  "lots": 0.1,
  "price": 1.0850,
  "stop_loss": 1.0800,
  "take_profit": 1.0950,
  "magic_number": 123456,
  "comment": "Strategy A"
}
```

### New Messages (To Implement)

**CONFIG** (Server → Slave EA):
```json
{
  "type": "CONFIG",
  "account_id": "67890",
  "enabled": true,
  "master_account": "12345",
  "lot_multiplier": 1.5,
  "reverse_trade": false,
  "allowed_symbols": ["EURUSD", "GBPUSD"],
  "blocked_symbols": ["USDJPY"],
  "allowed_magic_numbers": [123456],
  "blocked_magic_numbers": [999999],
  "symbol_mappings": [
    {"source": "EURUSD", "target": "EURUSD.m"},
    {"source": "GBPUSD", "target": "GBPUSD.m"}
  ]
}
```

**CONFIG_DISABLE** (Server → Slave EA):
```json
{
  "type": "CONFIG",
  "account_id": "67890",
  "enabled": false,
  "master_account": null
}
```

**CONFIG_ACK** (Slave EA → Server, optional):
```json
{
  "type": "CONFIG_ACK",
  "account_id": "67890",
  "status": "success",
  "message": "Configuration applied"
}
```

---

## 5. Implementation Components

### 5.1 Rust Server Changes

#### File: `rust-server/src/models.rs`

Add new message type:

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FullConfigMessage {
    pub r#type: String,  // "CONFIG"
    pub account_id: String,
    pub enabled: bool,
    pub master_account: Option<String>,
    pub lot_multiplier: Option<f64>,
    pub reverse_trade: bool,
    pub allowed_symbols: Option<Vec<String>>,
    pub blocked_symbols: Option<Vec<String>>,
    pub allowed_magic_numbers: Option<Vec<i32>>,
    pub blocked_magic_numbers: Option<Vec<i32>>,
    pub symbol_mappings: Vec<SymbolMapping>,
}

impl From<CopySettings> for FullConfigMessage {
    fn from(settings: CopySettings) -> Self {
        Self {
            r#type: "CONFIG".to_string(),
            account_id: settings.slave_account,
            enabled: settings.enabled,
            master_account: Some(settings.master_account),
            lot_multiplier: settings.lot_multiplier,
            reverse_trade: settings.reverse_trade,
            allowed_symbols: settings.filters.allowed_symbols,
            blocked_symbols: settings.filters.blocked_symbols,
            allowed_magic_numbers: settings.filters.allowed_magic_numbers,
            blocked_magic_numbers: settings.filters.blocked_magic_numbers,
            symbol_mappings: settings.symbol_mappings,
        }
    }
}
```

#### File: `rust-server/src/zeromq/mod.rs`

Extend `ZmqConfigSender`:

```rust
pub struct ZmqConfigSender {
    socket: zmq::Socket,
}

impl ZmqConfigSender {
    pub async fn send_full_config(&self, settings: &CopySettings) -> Result<()> {
        let config_msg: FullConfigMessage = settings.clone().into();
        let json = serde_json::to_string(&config_msg)?;
        self.socket.send(&json, 0)?;
        tracing::info!("Sent full config to EA: {}", settings.slave_account);
        Ok(())
    }

    pub async fn send_config_disable(&self, account_id: &str) -> Result<()> {
        let config_msg = FullConfigMessage {
            r#type: "CONFIG".to_string(),
            account_id: account_id.to_string(),
            enabled: false,
            master_account: None,
            lot_multiplier: None,
            reverse_trade: false,
            allowed_symbols: None,
            blocked_symbols: None,
            allowed_magic_numbers: None,
            blocked_magic_numbers: None,
            symbol_mappings: vec![],
        };
        let json = serde_json::to_string(&config_msg)?;
        self.socket.send(&json, 0)?;
        tracing::info!("Sent config disable to EA: {}", account_id);
        Ok(())
    }
}
```

#### File: `rust-server/src/api/mod.rs`

Update API handlers to send config:

```rust
async fn create_settings(
    State(state): State<AppState>,
    Json(req): Json<CreateSettingsRequest>,
) -> Result<Json<ApiResponse<CopySettings>>, Response> {
    match state.db.create_copy_settings(&req).await {
        Ok(settings) => {
            refresh_settings_cache(&state).await;

            // Send config to EA
            if let Err(e) = state.config_sender.send_full_config(&settings).await {
                tracing::error!("Failed to send config to EA: {}", e);
            }

            Ok(Json(ApiResponse::success(settings)))
        }
        Err(e) => {
            tracing::error!("Failed to create settings: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

async fn update_settings(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<CreateSettingsRequest>,
) -> Result<Json<ApiResponse<CopySettings>>, Response> {
    match state.db.update_copy_settings(id, &req).await {
        Ok(settings) => {
            refresh_settings_cache(&state).await;

            // Send updated config to EA
            if let Err(e) = state.config_sender.send_full_config(&settings).await {
                tracing::error!("Failed to send config to EA: {}", e);
            }

            Ok(Json(ApiResponse::success(settings)))
        }
        Err(e) => {
            tracing::error!("Failed to update settings: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

async fn delete_settings(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<()>>, Response> {
    // First get the account_id before deleting
    let account_id = match state.db.get_copy_settings(id).await {
        Ok(settings) => settings.slave_account,
        Err(e) => {
            tracing::error!("Failed to get settings before delete: {}", e);
            return Ok(Json(ApiResponse::error(e.to_string())));
        }
    };

    match state.db.delete_copy_settings(id).await {
        Ok(_) => {
            refresh_settings_cache(&state).await;

            // Send disable config to EA
            if let Err(e) = state.config_sender.send_config_disable(&account_id).await {
                tracing::error!("Failed to send config disable to EA: {}", e);
            }

            Ok(Json(ApiResponse::success(())))
        }
        Err(e) => {
            tracing::error!("Failed to delete settings: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}
```

#### File: `rust-server/src/connection_manager.rs`

Add method to sync configs on EA connection:

```rust
impl ConnectionManager {
    pub async fn sync_configs_for_ea(
        &self,
        account_id: &str,
        db: &Database,
        config_sender: &ZmqConfigSender,
    ) -> Result<()> {
        // Get all settings where this account is the slave
        let settings = db.get_settings_for_slave(account_id).await?;

        tracing::info!("Syncing {} configurations for EA {}", settings.len(), account_id);

        for setting in settings {
            if let Err(e) = config_sender.send_full_config(&setting).await {
                tracing::error!("Failed to sync config for {}: {}", account_id, e);
            }
        }

        Ok(())
    }
}
```

Update heartbeat handler:

```rust
async fn handle_heartbeat(&self, msg: HeartbeatMessage) {
    // ... existing code ...

    // Sync configurations for this EA
    if let Err(e) = self.sync_configs_for_ea(
        &msg.account_id,
        &self.db,
        &self.config_sender
    ).await {
        tracing::error!("Failed to sync configs on heartbeat: {}", e);
    }
}
```

---

### 5.2 Database Changes

#### File: `rust-server/src/db/mod.rs`

Add query method:

```rust
impl Database {
    pub async fn get_settings_for_slave(&self, slave_account: &str) -> Result<Vec<CopySettings>> {
        let conn = self.pool.get().await?;

        let rows = conn.call(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT * FROM copy_settings WHERE slave_account = ? AND enabled = 1"
            )?;

            let settings = stmt.query_map([slave_account], |row| {
                // ... map row to CopySettings ...
            })?
            .collect::<Result<Vec<_>, _>>()?;

            Ok(settings)
        }).await?;

        Ok(rows)
    }
}
```

---

### 5.3 EA Side Implementation (MQL4/MQL5)

#### File: `mql/MT4/Experts/SankeyCopier_Slave.mq4`

Add configuration storage:

```mql4
// Global configuration
bool g_ConfigEnabled = false;
string g_MasterAccount = "";
double g_LotMultiplier = 1.0;
bool g_ReverseTrade = false;

// Symbol filters
string g_AllowedSymbols[];
string g_BlockedSymbols[];
int g_AllowedMagicNumbers[];
int g_BlockedMagicNumbers[];

// Symbol mappings
struct SymbolMapping {
    string source;
    string target;
};
SymbolMapping g_SymbolMappings[];

// Handle incoming CONFIG message
void OnConfigMessage(string json) {
    // Parse JSON (use custom parser or library)
    // For example using simple string parsing:

    if (StringFind(json, "\"type\":\"CONFIG\"") < 0) return;

    // Extract enabled flag
    g_ConfigEnabled = ParseBool(json, "enabled");

    if (!g_ConfigEnabled) {
        Print("Configuration disabled, stopping copy");
        g_MasterAccount = "";
        return;
    }

    // Extract master account
    g_MasterAccount = ParseString(json, "master_account");

    // Extract lot multiplier
    g_LotMultiplier = ParseDouble(json, "lot_multiplier");
    if (g_LotMultiplier <= 0) g_LotMultiplier = 1.0;

    // Extract reverse trade
    g_ReverseTrade = ParseBool(json, "reverse_trade");

    // Extract arrays (more complex parsing)
    ParseStringArray(json, "allowed_symbols", g_AllowedSymbols);
    ParseStringArray(json, "blocked_symbols", g_BlockedSymbols);
    ParseIntArray(json, "allowed_magic_numbers", g_AllowedMagicNumbers);
    ParseIntArray(json, "blocked_magic_numbers", g_BlockedMagicNumbers);

    // Extract symbol mappings
    ParseSymbolMappings(json, g_SymbolMappings);

    Print("Configuration received:");
    Print("  Master: ", g_MasterAccount);
    Print("  Lot Multiplier: ", g_LotMultiplier);
    Print("  Reverse Trade: ", g_ReverseTrade);
    Print("  Allowed Symbols: ", ArraySize(g_AllowedSymbols));
}

// Apply filters before copying trade
bool ShouldCopyTrade(string symbol, int magicNumber) {
    if (!g_ConfigEnabled) return false;

    // Check allowed symbols
    if (ArraySize(g_AllowedSymbols) > 0) {
        bool found = false;
        for (int i = 0; i < ArraySize(g_AllowedSymbols); i++) {
            if (g_AllowedSymbols[i] == symbol) {
                found = true;
                break;
            }
        }
        if (!found) return false;
    }

    // Check blocked symbols
    for (int i = 0; i < ArraySize(g_BlockedSymbols); i++) {
        if (g_BlockedSymbols[i] == symbol) return false;
    }

    // Check allowed magic numbers
    if (ArraySize(g_AllowedMagicNumbers) > 0) {
        bool found = false;
        for (int i = 0; i < ArraySize(g_AllowedMagicNumbers); i++) {
            if (g_AllowedMagicNumbers[i] == magicNumber) {
                found = true;
                break;
            }
        }
        if (!found) return false;
    }

    // Check blocked magic numbers
    for (int i = 0; i < ArraySize(g_BlockedMagicNumbers); i++) {
        if (g_BlockedMagicNumbers[i] == magicNumber) return false;
    }

    return true;
}

// Apply transformations to trade signal
void TransformTradeSignal(TradeSignal &signal) {
    // Apply symbol mapping
    for (int i = 0; i < ArraySize(g_SymbolMappings); i++) {
        if (g_SymbolMappings[i].source == signal.symbol) {
            signal.symbol = g_SymbolMappings[i].target;
            break;
        }
    }

    // Apply lot multiplier
    signal.lots = signal.lots * g_LotMultiplier;
    signal.lots = NormalizeDouble(signal.lots, 2);

    // Apply reverse trade
    if (g_ReverseTrade) {
        signal.orderType = ReverseOrderType(signal.orderType);
    }
}
```

---

### 5.4 Web UI Changes

No major changes needed for Web UI. It continues to:
- Fetch data from REST API on page load
- Send create/update/delete requests to REST API
- Display data based on server responses

**Optional enhancement**: Add loading indicators and optimistic updates for better UX.

---

## 6. Error Handling & Edge Cases

### 6.1 EA Offline When Settings Changed

**Scenario**: User changes settings, but EA is offline.

**Solution**:
- Settings saved to SQLite immediately
- When EA reconnects, `sync_configs_for_ea` sends all settings
- EA applies settings and starts operating

**No data loss**.

---

### 6.2 ZeroMQ Message Delivery Failure

**Scenario**: Server sends CONFIG message, but EA doesn't receive (network issue).

**Solution**:
- EA sends periodic HEARTBEAT (every 10 seconds)
- Server detects heartbeat and checks if config is in sync
- If not, resend CONFIG message
- Alternative: EA can request config explicitly via REQUEST_CONFIG message

---

### 6.3 Multiple Slaves for Same Master

**Scenario**: Master account has 5 slave accounts.

**Solution**:
- Each slave receives its own CONFIG message
- Server loops through all settings for that master
- Each slave operates independently

---

### 6.4 Configuration Validation

**Server-side validation** (before saving to SQLite):
- `lot_multiplier` > 0
- `master_account` and `slave_account` are different
- `master_account` exists in connections
- `slave_account` exists in connections
- No duplicate (master, slave) pair

**EA-side validation** (after receiving CONFIG):
- Check if master account is online
- Validate symbol exists on broker
- Validate lot size within broker limits

---

### 6.5 Concurrent Updates

**Scenario**: Two browser windows open, both editing same setting.

**Solution**:
- SQLite handles concurrency (last write wins)
- Web UI shows toast notification on successful save
- Optional: Add optimistic locking (version field)

---

## 7. Security Considerations

### 7.1 Authentication

**Current**: No authentication (local deployment assumed)

**Future**:
- Add JWT authentication for REST API
- EA authentication via API key or shared secret
- Role-based access control (admin vs viewer)

---

### 7.2 Input Validation

**Strict validation on server**:
- Sanitize all string inputs
- Validate numeric ranges
- Prevent SQL injection (using parameterized queries)
- Prevent JSON injection in ZeroMQ messages

---

### 7.3 Rate Limiting

**Prevent abuse**:
- Limit CONFIG messages to 1 per second per EA
- Rate limit REST API endpoints
- Throttle rapid setting changes

---

## 8. Performance Optimization

### 8.1 Database Indexing

Add indexes for common queries:

```sql
CREATE INDEX idx_slave_account ON copy_settings(slave_account);
CREATE INDEX idx_master_account ON copy_settings(master_account);
CREATE INDEX idx_enabled ON copy_settings(enabled);
```

---

### 8.2 Connection Manager Caching

Cache active connections in memory:
- Avoid SQLite queries on every trade signal
- Update cache on heartbeat
- Expire stale connections after 30 seconds

---

### 8.3 Bulk Config Sync

On server startup:
- Load all settings into memory cache
- Send configs to all connected EAs in parallel
- Use async/await to avoid blocking

---

## 9. Testing Strategy

### 9.1 Unit Tests

**Rust Server**:
- `send_full_config` sends correct JSON
- `sync_configs_for_ea` queries correct settings
- API handlers validate input correctly

**EA**:
- Config parsing works with sample JSON
- Filters correctly block/allow trades
- Transformations apply correctly

---

### 9.2 Integration Tests

**End-to-End**:
1. Start Rust server
2. Start mock EA (Python script simulating ZeroMQ)
3. Create setting via REST API
4. Verify mock EA receives CONFIG message
5. Update setting
6. Verify mock EA receives updated CONFIG
7. Delete setting
8. Verify mock EA receives CONFIG_DISABLE

---

### 9.3 Load Tests

**Scenario**: 50 master accounts, 200 slave accounts, 500 copy settings

**Metrics**:
- REST API response time < 100ms
- CONFIG message delivery < 50ms
- Database query time < 10ms

---

## 10. Deployment & Migration

### 10.1 Migration Path

**Phase 1**: Add CONFIG message support (backward compatible)
- Rust server sends both old ConfigMessage and new FullConfigMessage
- Old EAs ignore FullConfigMessage
- New EAs use FullConfigMessage

**Phase 2**: Update all EAs to support FullConfigMessage

**Phase 3**: Remove old ConfigMessage support

---

### 10.2 Rollback Plan

If issues arise:
- Revert Rust server to previous version
- SQLite data remains unchanged
- EAs continue operating (may not have latest config)

---

## 11. Monitoring & Logging

### 11.1 Server Logs

Log events:
- CONFIG message sent (with account_id and timestamp)
- CONFIG_DISABLE sent
- Config sync on EA connection
- Failed ZeroMQ sends

Example:
```
[INFO] Sent full config to EA: 67890 (master: 12345)
[ERROR] Failed to send config to EA: 67890 (error: Connection refused)
```

---

### 11.2 EA Logs

Log events:
- CONFIG received
- Configuration applied
- Filters blocking trade
- Transformations applied

Example:
```
[INFO] Configuration received: Master=12345, LotMult=1.5
[DEBUG] Trade blocked: Symbol USDJPY in blocked list
[DEBUG] Transformed: EURUSD -> EURUSD.m, Lot=0.1 -> 0.15
```

---

## 12. Future Enhancements

### Phase 2
1. **Config versioning**: Track changes to settings over time
2. **Audit log**: Who changed what and when
3. **Config templates**: Save common configurations for reuse
4. **Batch operations**: Enable/disable multiple settings at once

### Phase 3
1. **Live config reload**: EA applies config without restart
2. **A/B testing**: Test two configurations side-by-side
3. **Performance analytics**: Track profitability per configuration
4. **Smart defaults**: AI-suggested optimal multipliers

---

## 13. Success Criteria

**Must have**:
- ✅ SQLite is single source of truth
- ✅ Web UI creates/edits/deletes settings successfully
- ✅ EA receives CONFIG on startup and changes
- ✅ All filters and transformations work correctly
- ✅ No data loss when EA offline
- ✅ Settings persist across server restarts

**Nice to have**:
- ⭕ Config sync within 1 second
- ⭕ Zero downtime during config changes
- ⭕ Web UI shows config delivery status

---

## 14. Questions for Review

Before implementation:

1. **JSON Parsing in MQL**: MQL4/MQL5 don't have native JSON parsing. Options:
   - Option A: Write custom JSON parser in MQL
   - Option B: Use simple key-value format instead of JSON
   - Option C: Use external DLL for JSON parsing
   - **Recommendation**: Option A (custom parser) for simplicity

2. **Config Persistence in EA**: Should EA save config to local file as backup?
   - Pro: Can operate if server is down
   - Con: May cause inconsistency
   - **Recommendation**: No local persistence, always trust server

3. **Config Acknowledge**: Should EA send CONFIG_ACK back to server?
   - Pro: Server knows config was received
   - Con: Additional complexity
   - **Recommendation**: Yes, for monitoring purposes

4. **Heartbeat Frequency**: Current is 10 seconds. Is this optimal?
   - Faster: Better sync, more network traffic
   - Slower: Less traffic, delayed sync
   - **Recommendation**: Keep 10 seconds, make configurable

---

**End of Design Document**

**Ready for review and approval.**
