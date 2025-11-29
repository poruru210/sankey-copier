# SANKEY Copier ドキュメント

MT4/MT5間でトレードをコピーするシステムの技術ドキュメント。

## システム概要

SANKEY Copierは、MetaTrader 4/5のアカウント間でトレードをリアルタイムにコピーするシステムです。

```mermaid
graph TB
    subgraph "MetaTrader (Master)"
        MT_M[MT4/MT5<br/>Master Account]
        MEA[Master EA]
    end

    subgraph "mt-bridge"
        DLL[sankey_copier_zmq.dll]
    end

    subgraph "relay-server"
        ZMQ[ZeroMQ<br/>:5555/:5556/:5557]
        API[REST API<br/>:3000]
        WS[WebSocket<br/>/ws]
        DB[(SQLite)]
    end

    subgraph "web-ui"
        UI[React App<br/>:8080]
    end

    subgraph "MetaTrader (Slave)"
        SEA[Slave EA]
        MT_S[MT4/MT5<br/>Slave Account]
    end

    MT_M --> MEA
    MEA --> DLL
    DLL -->|PUSH| ZMQ
    ZMQ -->|PUB| DLL
    DLL --> SEA
    SEA --> MT_S

    API --> DB
    WS --> UI
    UI --> API
```

## コンポーネント

| コンポーネント | 説明 | 技術 |
|---------------|------|------|
| [relay-server](./relay-server.md) | 中継サーバー | Rust, Axum, ZeroMQ, SQLite |
| [mt-bridge](./mt-bridge.md) | EA-サーバー通信DLL | Rust, ZeroMQ, MessagePack |
| [mt-advisors](./mt-advisors.md) | MT4/MT5用EA | MQL4/MQL5 |
| [web-ui](./web-ui.md) | 設定・監視UI | Next.js, React, TypeScript |

## 通信フロー

### トレードコピーの流れ

```mermaid
sequenceDiagram
    participant Master as Master EA
    participant DLL as mt-bridge
    participant RS as relay-server
    participant Slave as Slave EA
    participant MT as Slave MT

    Note over Master: トレード実行検出

    Master->>DLL: serialize_trade_signal()
    DLL->>RS: ZMQ PUSH (5555)

    RS->>RS: フィルタリング
    RS->>RS: シンボル変換

    RS->>DLL: ZMQ PUB (5556)
    DLL->>Slave: parse_trade_signal()

    Slave->>Slave: ロット計算
    Slave->>MT: OrderSend()
```

### 設定更新の流れ

```mermaid
sequenceDiagram
    participant UI as web-ui
    participant API as REST API
    participant DB as SQLite
    participant RS as relay-server
    participant EA as Slave EA

    UI->>API: PUT /api/trade-groups/{id}/members/{slave}
    API->>DB: UPDATE slave_settings
    API->>RS: send_config_to_slave()
    RS->>EA: ZMQ PUB (5557)
    EA->>EA: 設定適用
```

## コンポーネント関連図

```mermaid
graph LR
    subgraph "ユーザー"
        User[ユーザー]
    end

    subgraph "クライアント層"
        WebUI[web-ui]
        MT4[MT4/MT5]
    end

    subgraph "通信層"
        DLL[mt-bridge DLL]
    end

    subgraph "サーバー層"
        RS[relay-server]
    end

    User -->|ブラウザ| WebUI
    User -->|チャート| MT4

    WebUI -->|REST API| RS
    WebUI <-->|WebSocket| RS

    MT4 -->|DLL Import| DLL
    DLL <-->|ZeroMQ| RS
```

## 設定処理の分担

### relay-serverが処理

- 設定のDB保存・永続化
- 設定のEAへの配布
- シンボル変換（prefix/suffix/mapping）
- ステータス管理

### EA側が処理

- ロット計算 (`lot_multiplier`, `margin_ratio`)
- トレード実行
- リトライ制御
- スリッページ制御
- 売買方向反転

```mermaid
flowchart LR
    subgraph "web-ui"
        Form[設定フォーム]
    end

    subgraph "relay-server"
        DB[(設定保存)]
        Transform[シンボル変換]
    end

    subgraph "EA (Slave)"
        LotCalc[ロット計算]
        Execute[トレード実行]
    end

    Form -->|設定更新| DB
    DB -->|配布| Transform
    Transform -->|シグナル| LotCalc
    LotCalc --> Execute
```

## ステータス

| 値 | 名称 | 説明 |
|----|------|------|
| 0 | DISABLED | ユーザーが無効化 |
| 1 | ENABLED | 有効だがMasterオフライン |
| 2 | CONNECTED | 完全に有効 |
| 4 | REMOVED | 削除済み |

## ポート構成

| ポート | 用途 |
|--------|------|
| 3000 | relay-server REST API (HTTPS) |
| 5555 | ZeroMQ PULL (EA→サーバー) |
| 5556 | ZeroMQ PUB (トレードシグナル) |
| 5557 | ZeroMQ PUB (設定配布) |
| 8080 | web-ui (開発時) |

## 主要な型定義

### TradeSignal

```rust
struct TradeSignal {
    action: TradeAction,      // Open, Close, Modify
    ticket: i64,
    symbol: Option<String>,
    order_type: Option<OrderType>,
    lots: Option<f64>,
    open_price: Option<f64>,
    stop_loss: Option<f64>,
    take_profit: Option<f64>,
    magic_number: Option<i32>,
    source_account: String,
    close_ratio: Option<f64>,  // 部分決済用
}
```

### SlaveSettings

```rust
struct SlaveSettings {
    lot_calculation_mode: LotCalculationMode,
    lot_multiplier: Option<f64>,
    symbol_prefix: Option<String>,
    symbol_suffix: Option<String>,
    symbol_mappings: Vec<SymbolMapping>,
    filters: TradeFilters,
    reverse_trade: bool,
    sync_mode: SyncMode,
    max_slippage: Option<i32>,
    max_retries: i32,
    max_signal_delay_ms: i32,
    config_version: u32,
}
```

## ドキュメント一覧

- [relay-server](./relay-server.md) - 中継サーバーの詳細
- [mt-bridge](./mt-bridge.md) - 通信DLLの詳細
- [mt-advisors](./mt-advisors.md) - EAの詳細
- [web-ui](./web-ui.md) - WebUIの詳細
