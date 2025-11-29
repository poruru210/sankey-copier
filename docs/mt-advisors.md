# MT-Advisors

MetaTrader上で動作するExpert Advisor。Master EAがトレードを検出し、Slave EAがコピー実行する。

## Responsibilities

### Master EA
- **トレード検出**: ポジション/オーダーの開始・決済・変更を検出
- **シグナル送信**: TradeSignalをRelay-Serverへ送信
- **ポジション同期**: SyncRequestに応じてPositionSnapshotを送信
- **Heartbeat**: 30秒ごとに状態を報告

### Slave EA
- **シグナル受信**: TradeSignalを受信して実行
- **ロット計算**: 倍率またはマージン比率で計算
- **方向反転**: 設定に応じてBuy↔Sell反転
- **同期処理**: 既存ポジションの同期実行
- **チケットマッピング**: Master→Slaveチケット対応管理

## Architecture

```mermaid
graph TB
    subgraph "Master EA"
        MD[Trade Detection<br/>OnTradeTransaction/OnTick]
        MS[Signal Sender]
        MH[Heartbeat]
        MC[Config Handler]
    end

    subgraph "Include Files"
        CMN[Common.mqh<br/>DLL imports]
        MSG[Messages.mqh<br/>Serialization]
        MSIG[MasterSignals.mqh]
    end

    subgraph "Slave EA"
        SR[Signal Receiver]
        TE[Trade Executor]
        LT[Lot Transformer]
        TR[Trade Reversal]
        TM[Ticket Mapping]
        SC[Config Handler]
    end

    subgraph "Include Files "
        TRD[Trade.mqh<br/>Config parsing]
        SLV[SlaveTrade.mqh<br/>Execution]
        MAP[Mapping.mqh]
    end

    subgraph "MT-Bridge DLL"
        DLL[sankey_copier_zmq.dll]
    end

    MD --> MS
    MS --> MSIG
    MSIG --> MSG
    MSG --> CMN
    CMN --> DLL

    SR --> SC
    SC --> TRD
    SR --> TE
    TE --> SLV
    TE --> LT
    TE --> TR
    TE --> TM
    TM --> MAP
    SLV --> CMN
```

## Project Structure

```
mt-advisors/
├── MT5/
│   ├── SankeyCopierMaster.mq5     # Master EA (MT5)
│   └── SankeyCopierSlave.mq5      # Slave EA (MT5)
├── MT4/
│   ├── SankeyCopierMaster.mq4     # Master EA (MT4)
│   └── SankeyCopierSlave.mq4      # Slave EA (MT4)
└── Include/
    └── SankeyCopier/
        ├── Common.mqh             # DLL imports, constants
        ├── Zmq.mqh                # ZMQ initialization
        ├── Messages.mqh           # Message serialization
        ├── MessageParsing.mqh     # Message parsing utilities
        ├── MasterSignals.mqh      # Master signal functions
        ├── Trade.mqh              # Config parsing, lot transform
        ├── SlaveTrade.mqh         # Trade execution
        ├── SlaveTypes.mqh         # Slave type definitions
        ├── Mapping.mqh            # Ticket mapping
        ├── GridPanel.mqh          # UI panel
        └── Logging.mqh            # VictoriaLogs integration
```

## Master EA

### Input Parameters

```mql5
input string RelayServerAddress = "tcp://localhost:5555";   // PUSH endpoint
input string ConfigSourceAddress = "tcp://localhost:5557";  // SUB endpoint
input int    ScanInterval = 100;                            // ms
input bool   ShowConfigPanel = true;
input int    PanelWidth = 280;
```

### Trade Detection

```mermaid
sequenceDiagram
    participant Trader
    participant MT as MetaTrader
    participant Master as Master EA
    participant DLL as MT-Bridge
    participant RS as Relay-Server

    Trader->>MT: Open Position
    MT->>Master: OnTradeTransaction (MT5)<br/>or OnTick scan (MT4)

    Master->>Master: Detect new position

    Master->>DLL: serialize_trade_signal("Open", ...)
    Master->>DLL: zmq_socket_send_binary()
    DLL->>RS: TradeSignal

    Note over Master: Track position for<br/>close/modify detection
```

#### MT5: Event-Driven (OnTradeTransaction)

```mql5
void OnTradeTransaction(
    const MqlTradeTransaction& trans,
    const MqlTradeRequest& request,
    const MqlTradeResult& result)
{
    switch(trans.type) {
        case TRADE_TRANSACTION_DEAL_ADD:
            // Position opened
            SendPositionOpenSignal(ticket, symbol, type, lots, ...);
            break;

        case TRADE_TRANSACTION_HISTORY_ADD:
            // Position closed
            SendPositionCloseSignal(ticket, close_ratio);
            break;

        case TRADE_TRANSACTION_ORDER_ADD:
            // Pending order created
            SendOrderOpenSignal(...);
            break;

        case TRADE_TRANSACTION_ORDER_DELETE:
            // Pending order deleted
            SendOrderCloseSignal(...);
            break;

        case TRADE_TRANSACTION_ORDER_UPDATE:
            // Order modified
            SendOrderModifySignal(...);
            break;
    }
}
```

**Code Reference**: `SankeyCopierMaster.mq5:445-534`

#### MT4: Polling (OnTick)

```mql5
void OnTick() {
    CheckForNewOrders();      // New positions/orders
    CheckForModifiedOrders(); // SL/TP changes
    CheckForPartialCloses();  // Volume changes
    CheckForClosedOrders();   // Closed positions
}
```

**Code Reference**: `SankeyCopierMaster.mq4:381-544`

### Partial Close Detection

```mermaid
flowchart TD
    A[OnTick/OnTradeTransaction] --> B{Position volume changed?}
    B -->|No| C[Skip]
    B -->|Yes| D[Calculate close_ratio]
    D --> E["close_ratio = (tracked_lots - current_lots) / tracked_lots"]
    E --> F[SendCloseSignal with close_ratio]
    F --> G[Update tracked volume]
```

**Code Reference**: `SankeyCopierMaster.mq5:611-637`

### Heartbeat

30秒ごとにOnTimerから送信。

```mql5
// HeartbeatMessage contents:
- account_id
- balance, equity
- open_positions count
- ea_type: "Master"
- platform: "MT4" or "MT5"
- account_number, broker, server
- is_trade_allowed
- symbol_prefix, symbol_suffix
- version (BUILD_INFO)
```

**Code Reference**: `SankeyCopierMaster.mq5:175-306`, `Messages.mqh:131-187`

### Position Snapshot

SyncRequest受信時、またはMaster起動時に送信。

```mermaid
sequenceDiagram
    participant Slave as Slave EA
    participant RS as Relay-Server
    participant Master as Master EA

    Slave->>RS: SyncRequest (status==CONNECTED, sync_mode!=SKIP)
    RS->>Master: SyncRequest via SUB :5557
    Master->>Master: Collect all open positions
    Master->>Master: Clean symbol names
    Master->>RS: PositionSnapshot via PUSH :5555
    RS->>Slave: PositionSnapshot via SUB :5557
```

**Code Reference**: `MasterSignals.mqh:124-228`

## Slave EA

### Input Parameters

```mql5
input string RelayServerAddress = "tcp://localhost:5555";       // PUSH endpoint
input string TradeSignalSourceAddress = "tcp://localhost:5556"; // Trade SUB
input string ConfigSourceAddress = "tcp://localhost:5557";      // Config SUB
input bool   ShowConfigPanel = true;
input int    PanelWidth = 280;
```

### Configuration Structure

```mermaid
classDiagram
    class CopyConfig {
        +master_account: string
        +trade_group_id: string
        +status: int
        +lot_calculation_mode: int
        +lot_multiplier: double
        +reverse_trade: bool
        +config_version: int
        +symbol_prefix: string
        +symbol_suffix: string
        +symbol_mappings[]: SymbolMapping
        +filters: TradeFilters
        +source_lot_min: double
        +source_lot_max: double
        +master_equity: double
        +sync_mode: int
        +limit_order_expiry_min: int
        +market_sync_max_pips: double
        +max_slippage: int
        +copy_pending_orders: bool
        +max_retries: int
        +max_signal_delay_ms: int
        +use_pending_order_for_delayed: bool
        +allow_new_orders: bool
    }

    class SymbolMapping {
        +source_symbol: string
        +target_symbol: string
    }

    class TradeFilters {
        +allowed_symbols[]: string
        +blocked_symbols[]: string
        +allowed_magic_numbers[]: int
        +blocked_magic_numbers[]: int
    }

    CopyConfig "1" *-- "*" SymbolMapping
    CopyConfig "1" *-- "1" TradeFilters
```

**Code Reference**: `SlaveTypes.mqh:36-63`

### Trade Signal Processing

```mermaid
flowchart TD
    A[Receive TradeSignal] --> B[Parse with parse_trade_signal]
    B --> C{Find matching CopyConfig<br/>for source_account}
    C -->|Not found| D[Skip]
    C -->|Found| E{status == CONNECTED?}
    E -->|No| F[Skip]
    E -->|Yes| G{allow_new_orders?}
    G -->|No| H[Skip]
    G -->|Yes| I{action type?}
    I -->|Open| J[ExecuteOpenTrade]
    I -->|Close| K[ExecuteCloseTrade]
    I -->|Modify| L[ExecuteModifyTrade]
```

**Code Reference**: `SankeyCopierSlave.mq5:482-581`

### Lot Calculation

```mermaid
flowchart TD
    A[Source Lots] --> B{lot_calculation_mode?}
    B -->|multiplier| C["new_lots = lots × lot_multiplier"]
    B -->|margin_ratio| D["ratio = slave_equity / master_equity"]
    D --> E["new_lots = lots × ratio"]
    C --> F[NormalizeLotSize]
    E --> F
    F --> G{Within symbol constraints?}
    G -->|Yes| H[Return normalized lots]
    G -->|Adjust| I[Clamp to min/max]
    I --> H
```

**Code Reference**: `Trade.mqh:529-559`

#### Multiplier Mode

```
Input: lots=1.0, lot_multiplier=2.5
Output: 1.0 × 2.5 = 2.5 lots
```

#### Margin Ratio Mode

```
Input: lots=1.0, slave_equity=50000, master_equity=25000
Ratio: 50000 / 25000 = 2.0
Output: 1.0 × 2.0 = 2.0 lots
```

### Trade Reversal

```mermaid
flowchart LR
    A[Order Type] --> B{reverse_trade?}
    B -->|No| C[Keep original]
    B -->|Yes| D[Reverse]
    D --> E["Buy → Sell<br/>Sell → Buy<br/>BuyLimit → SellLimit<br/>SellLimit → BuyLimit<br/>BuyStop → SellStop<br/>SellStop → BuyStop"]
```

**Code Reference**: `Trade.mqh:574-586`

### Trade Execution

```mermaid
sequenceDiagram
    participant Signal as TradeSignal
    participant Exec as ExecuteOpenTrade
    participant Trans as Transform
    participant MT as MetaTrader

    Signal->>Exec: action="Open"

    Exec->>Exec: Check signal delay
    alt delay > max_signal_delay_ms
        alt use_pending_order_for_delayed
            Exec->>MT: Place Pending Order
        else
            Exec->>Exec: Skip signal
        end
    else delay OK
        Exec->>Trans: TransformLotSize()
        Trans-->>Exec: transformed lots

        Exec->>Trans: ReverseOrderType()
        Trans-->>Exec: final order type

        loop max_retries
            Exec->>MT: OrderSend / trade.Buy()
            alt Success
                Exec->>Exec: AddTicketMapping()
                Exec-->>Signal: Done
            else Failure
                Exec->>Exec: Retry
            end
        end
    end
```

**Code Reference**: `SlaveTrade.mqh:101-173` (MT5), `SlaveTrade.mqh:475-557` (MT4)

### Partial Close Handling

```mql5
void ExecuteCloseTrade(long master_ticket, double close_ratio) {
    long slave_ticket = GetSlaveTicketFromMapping(master_ticket);

    if (close_ratio > 0 && close_ratio < 1.0) {
        // Partial close
        double current_lots = PositionGetDouble(POSITION_VOLUME);
        double close_lots = current_lots * close_ratio;
        trade.PositionClosePartial(slave_ticket, close_lots);
        // Keep mapping (position still open)
    } else {
        // Full close
        trade.PositionClose(slave_ticket);
        RemoveTicketMapping(master_ticket);
    }
}
```

**Code Reference**: `SlaveTrade.mqh:179-242`

### Open Sync (Position Synchronization)

```mermaid
flowchart TD
    A[Slave connects with status=CONNECTED] --> B{sync_mode?}
    B -->|skip| C[Skip sync]
    B -->|limit_order| D[SyncWithLimitOrder]
    B -->|market_order| E[SyncWithMarketOrder]

    D --> F[For each Master position]
    F --> G{Already mapped?}
    G -->|Yes| H[Skip]
    G -->|No| I[TransformLotSize]
    I --> J[ReverseOrderType if needed]
    J --> K[Place Limit Order at Master's open_price]
    K --> L[Set expiry: limit_order_expiry_min]

    E --> M[For each Master position]
    M --> N{Already mapped?}
    N -->|Yes| O[Skip]
    N -->|No| P{Price deviation <= max_pips?}
    P -->|No| Q[Skip]
    P -->|Yes| R[Execute Market Order]
```

**Code Reference**: `SankeyCopierSlave.mq5:587-719`

### Ticket Mapping

Master/Slaveチケットの対応を管理し、再起動後も復元可能。

```mermaid
classDiagram
    class TicketMapping {
        +master_ticket: long
        +slave_ticket: long
    }

    class PendingTicketMapping {
        +master_ticket: long
        +pending_ticket: long
    }

    class MappingManager {
        +g_order_map[]: TicketMapping
        +g_pending_order_map[]: PendingTicketMapping
        +AddTicketMapping()
        +GetSlaveTicketFromMapping()
        +RemoveTicketMapping()
        +RecoverMappingsFromPositions()
    }

    MappingManager --> TicketMapping
    MappingManager --> PendingTicketMapping
```

#### Comment Format

```
M{master_ticket}  # Market order, e.g., "M1234567890"
P{master_ticket}  # Pending order, e.g., "P1234567890"
```

最大21文字（MT5の31文字制限内）

**Code Reference**: `Mapping.mqh:208-221`

#### Recovery from Restart

```mql5
void RecoverMappingsFromPositions() {
    // Scan all open positions
    for (int i = 0; i < PositionsTotal(); i++) {
        string comment = PositionGetString(POSITION_COMMENT);

        // Parse "M{ticket}" or "P{ticket}"
        if (StringSubstr(comment, 0, 1) == "M") {
            long master_ticket = StringToInteger(StringSubstr(comment, 1));
            AddTicketMapping(master_ticket, PositionGetInteger(POSITION_TICKET));
        }
    }

    // Scan pending orders too
    for (int i = 0; i < OrdersTotal(); i++) {
        string comment = OrderGetString(ORDER_COMMENT);
        // ...similar parsing
    }
}
```

**Code Reference**: `Mapping.mqh:286-404`

## ZMQ Communication

### Socket Configuration

| EA | Socket Type | Port | Purpose |
|----|-------------|------|---------|
| Master | PUSH | 5555 | Heartbeat, TradeSignal, PositionSnapshot |
| Master | SUB | 5557 | MasterConfig, SyncRequest |
| Slave | PUSH | 5555 | Heartbeat, RequestConfig, SyncRequest |
| Slave | SUB | 5556 | TradeSignal (topic: trade_group_id) |
| Slave | SUB | 5557 | SlaveConfig (topic: account_id) |

### Message Flow

```mermaid
sequenceDiagram
    participant MEA as Master EA
    participant RS as Relay-Server
    participant SEA as Slave EA

    Note over MEA,SEA: Startup

    MEA->>RS: Heartbeat (PUSH :5555)
    MEA->>RS: RequestConfig (PUSH :5555)
    RS->>MEA: MasterConfigMessage (PUB :5557)

    SEA->>RS: Heartbeat (PUSH :5555)
    SEA->>RS: RequestConfig (PUSH :5555)
    RS->>SEA: SlaveConfigMessage (PUB :5557)

    Note over MEA,SEA: Sync (if sync_mode != skip)

    SEA->>RS: SyncRequest (PUSH :5555)
    RS->>MEA: SyncRequest (PUB :5557)
    MEA->>RS: PositionSnapshot (PUSH :5555)
    RS->>SEA: PositionSnapshot (PUB :5557)

    Note over MEA,SEA: Trade Copying

    MEA->>RS: TradeSignal (PUSH :5555)
    RS->>SEA: TradeSignal (PUB :5556)
    SEA->>SEA: Execute trade
```

## Configuration Sources

### EA-Side Only (Input Parameters)

設定変更にはEA再起動が必要。

| Setting | Master | Slave | Description |
|---------|--------|-------|-------------|
| RelayServerAddress | ✓ | ✓ | PUSH endpoint |
| ConfigSourceAddress | ✓ | ✓ | Config SUB endpoint |
| TradeSignalSourceAddress | - | ✓ | Trade SUB endpoint |
| ScanInterval | ✓ | - | Position scan interval (ms) |
| ShowConfigPanel | ✓ | ✓ | UI panel display |

### From Relay-Server (Web-UI経由)

リアルタイムで更新される。

| Setting | Master | Slave | Description |
|---------|--------|-------|-------------|
| symbol_prefix | ✓ | - | 削除するプレフィックス |
| symbol_suffix | ✓ | - | 削除するサフィックス |
| lot_calculation_mode | - | ✓ | 計算モード |
| lot_multiplier | - | ✓ | ロット倍率 |
| reverse_trade | - | ✓ | 方向反転 |
| sync_mode | - | ✓ | 同期モード |
| limit_order_expiry_min | - | ✓ | リミット有効期限 |
| market_sync_max_pips | - | ✓ | マーケット同期許容差 |
| max_slippage | - | ✓ | スリッページ許容値 |
| max_retries | - | ✓ | リトライ回数 |
| max_signal_delay_ms | - | ✓ | シグナル遅延許容値 |
| use_pending_order_for_delayed | - | ✓ | 遅延時ペンディング使用 |
| copy_pending_orders | - | ✓ | ペンディングコピー |

## Platform Differences (MT4 vs MT5)

| Feature | MT4 | MT5 |
|---------|-----|-----|
| Trade Detection | OnTick polling | OnTradeTransaction event |
| Position Access | OrderSelect() | PositionSelectByTicket() |
| Order Types | OP_BUY, OP_SELL | ORDER_TYPE_BUY, etc. |
| Trade Execution | OrderSend() | CTrade class |
| Pending Fill Detection | OnTick polling | OnTradeTransaction |
| Ticket Type | int | long |

## Status Values

### Member Status

| Value | Name | Description |
|-------|------|-------------|
| 0 | DISABLED | ユーザーが無効化 |
| 1 | ENABLED | 有効だがMasterオフライン |
| 2 | CONNECTED | 有効かつMasterオンライン |
| 4 | REMOVED | 削除済み |

### Sync Mode

| Value | Name | Description |
|-------|------|-------------|
| 0 | SKIP | 同期しない |
| 1 | LIMIT_ORDER | Masterの価格でリミット注文 |
| 2 | MARKET_ORDER | 価格許容範囲内でマーケット注文 |

### Lot Calculation Mode

| Value | Name | Description |
|-------|------|-------------|
| 0 | MULTIPLIER | 固定倍率 |
| 1 | MARGIN_RATIO | エクイティ比率 |

## Key Files Reference

| File | Lines | Purpose |
|------|-------|---------|
| `Common.mqh` | 50-148 | DLL imports |
| `Messages.mqh` | 131-187 | Heartbeat serialization |
| `MasterSignals.mqh` | 22-228 | Signal sending functions |
| `Trade.mqh` | 109-352 | Config parsing |
| `Trade.mqh` | 529-586 | Lot transform, reversal |
| `SlaveTrade.mqh` | 101-312 | MT5 trade execution |
| `SlaveTrade.mqh` | 475-857 | MT4 trade execution |
| `Mapping.mqh` | 286-404 | Ticket recovery |
| `SankeyCopierMaster.mq5` | 445-534 | OnTradeTransaction |
| `SankeyCopierSlave.mq5` | 482-581 | ProcessTradeSignal |
| `SankeyCopierSlave.mq5` | 587-719 | ProcessPositionSnapshot |

## Defaults

```mql5
#define DEFAULT_SLIPPAGE              30      // points
#define DEFAULT_MAX_RETRIES           3       // attempts
#define DEFAULT_MAX_SIGNAL_DELAY_MS   5000    // 5 seconds
```

設定がRelay-Serverから届くまでこれらの値が使用される。
