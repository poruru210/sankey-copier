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
        ZMQ[ZeroMQ<br/>:5555/:5556]
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

## 詳細データフロー図（複数Master/Slave構成）

実運用では複数のMasterから複数のSlaveにトレードをコピーする構成が一般的です。

```mermaid
graph TB
    subgraph master_accounts["Master Accounts"]
        M1["Master 1 - IC Markets"]
        M2["Master 2 - OANDA"]
    end

    subgraph master_eas["Master EAs"]
        MEA1["Master EA 1"]
        MEA2["Master EA 2"]
    end

    subgraph dll["mt-bridge DLL"]
        DLL1["DLL Instance 1"]
        DLL2["DLL Instance 2"]
        DLL3["DLL Instance 3"]
        DLL4["DLL Instance 4"]
    end

    subgraph relay["relay-server"]
        direction TB
        PULL["ZMQ PULL 5555"]
        MH["MessageHandler"]
        CE["CopyEngine"]
        CM["ConnectionManager"]
        DB[("SQLite")]
        PUB["ZMQ PUB 5556 (Unified)"]
    end

    subgraph slave_eas["Slave EAs"]
        SEA1["Slave EA 1"]
        SEA2["Slave EA 2"]
        SEA3["Slave EA 3"]
    end

    subgraph slave_accounts["Slave Accounts"]
        S1["Slave 1 - XM"]
        S2["Slave 2 - FXCM"]
        S3["Slave 3 - Exness"]
    end

    M1 --> MEA1
    M2 --> MEA2

    MEA1 --> DLL1
    MEA2 --> DLL2

    DLL1 -->|PUSH| PULL
    DLL2 -->|PUSH| PULL

    PULL --> MH
    MH --> CE
    MH --> CM
    CE --> DB
    CM --> DB

    CE --> PUB
    MH --> PUB

    PUB -->|SUB| DLL3
    PUB -->|SUB| DLL4

    DLL3 --> SEA1
    DLL3 --> SEA2
    DLL4 --> SEA3

    SEA1 --> S1
    SEA2 --> S2
    SEA3 --> S3

    style M1 fill:#e1f5fe
    style M2 fill:#e1f5fe
    style S1 fill:#fff3e0
    style S2 fill:#fff3e0
    style S3 fill:#fff3e0
```

### データフローの説明

| フロー | 説明 |
|--------|------|
| Master → PULL | 各MasterがHeartbeat、TradeSignalをPUSH送信 |
| MessageHandler | メッセージタイプを判定し適切なハンドラーに振り分け |
| CopyEngine | フィルタリング・シンボル変換後、対象Slaveを特定 |
| PUB (Unified) | トピックベースでトレードシグナル(trade_group_id)と設定(account_id)を配信 |

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
    RS->>EA: ZMQ PUB (5556 unified)
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

## 内部モジュール構成図

各コンポーネントの内部構造を示します。

### relay-server 内部構成

```mermaid
graph TB
    subgraph relay["relay-server"]
        subgraph api_layer["API Layer"]
            AXUM["Axum HTTP Server"]
            REST["REST Handlers"]
            WS["WebSocket Handler"]
            MW["Middleware CORS, PNA"]
        end

        subgraph business["Business Logic"]
            MH["MessageHandler"]
            CE["CopyEngine"]
            CM["ConnectionManager"]
            CP["ConfigPublisher"]
        end

        subgraph msg_handlers["Message Handlers"]
            HB["HeartbeatHandler"]
            TS["TradeSignalHandler"]
            CR["ConfigRequestHandler"]
            PS["PositionSnapshotHandler"]
            UR["UnregisterHandler"]
        end

        subgraph data_layer["Data Layer"]
            DB[("SQLite")]
            TG["TradeGroups DAO"]
            TGM["TradeGroupMembers DAO"]
            CD["ConfigDistribution"]
        end

        subgraph zmq_layer["ZeroMQ Layer"]
            PULL["ZMQ PULL 5555"]
            PUB["ZMQ PUB 5556 (Unified)"]
        end

        AXUM --> REST
        AXUM --> WS
        REST --> MW
        REST --> TG
        REST --> TGM
        REST --> CM

        PULL --> MH
        MH --> HB
        MH --> TS
        MH --> CR
        MH --> PS
        MH --> UR

        HB --> CM
        TS --> CE
        CR --> CD
        CE --> PUB
        CP --> PUB

        TG --> DB
        TGM --> DB
        CD --> DB
    end
```

### mt-bridge 内部構成

```mermaid
graph TB
    subgraph mt_bridge["mt-bridge DLL"]
        subgraph ffi_layer["FFI Layer"]
            ZMQ_FFI["ZMQ FFI"]
            MP_FFI["MessagePack FFI"]
            VL_FFI["VictoriaLogs FFI"]
        end

        subgraph zmq_mod["ZeroMQ Module"]
            CTX["Context Manager"]
            SOCK["Socket Manager"]
            SEND["Send Functions"]
            RECV["Receive Functions"]
        end

        subgraph msgpack_mod["MessagePack Module"]
            SER["Serializers"]
            PAR["Parsers"]
            ACC["Field Accessors"]
            BUF["Buffer Management"]
        end

        subgraph vlogs_mod["VictoriaLogs Module"]
            CFG["Config"]
            BUFFER["Log Buffer"]
            HTTP["HTTP Client"]
            FLUSH["Flush Thread"]
        end

        subgraph helpers["Helpers"]
            UTF16["UTF-16 Converter"]
            HANDLE["Handle Manager"]
        end

        ZMQ_FFI --> CTX
        ZMQ_FFI --> SOCK
        SOCK --> SEND
        SOCK --> RECV

        MP_FFI --> SER
        MP_FFI --> PAR
        PAR --> ACC
        SER --> BUF

        VL_FFI --> CFG
        VL_FFI --> BUFFER
        BUFFER --> FLUSH
        FLUSH --> HTTP

        SER --> UTF16
        PAR --> UTF16
        CTX --> HANDLE
        SOCK --> HANDLE
    end
```

### mt-advisors 内部構成

```mermaid
graph TB
    subgraph master_ea["Master EA"]
        subgraph m_events["Event Handlers"]
            M_INIT["OnInit"]
            M_TICK["OnTick"]
            M_TIMER["OnTimer"]
            M_DEINIT["OnDeinit"]
        end

        subgraph m_detection["Order Detection"]
            SCAN["ScanExistingOrders"]
            NEW["CheckForNewOrders"]
            MOD["CheckForModifiedOrders"]
            CLOSE["CheckForClosedOrders"]
        end

        subgraph m_signals["Signal Sending"]
            OPEN_SIG["SendOpenSignal"]
            CLOSE_SIG["SendCloseSignal"]
            MOD_SIG["SendModifySignal"]
            SNAP["SendPositionSnapshot"]
        end

        subgraph m_comm["Communication"]
            M_ZMQ["ZMQ PUSH Socket"]
            M_HB["Heartbeat Sender"]
        end

        M_INIT --> SCAN
        M_TICK --> NEW
        M_TICK --> MOD
        M_TICK --> CLOSE
        M_TIMER --> M_HB

        NEW --> OPEN_SIG
        CLOSE --> CLOSE_SIG
        MOD --> MOD_SIG

        OPEN_SIG --> M_ZMQ
        CLOSE_SIG --> M_ZMQ
        MOD_SIG --> M_ZMQ
        M_HB --> M_ZMQ
    end

    subgraph slave_ea["Slave EA"]
        subgraph s_events["Event Handlers"]
            S_INIT["OnInit"]
            S_TICK["OnTick"]
            S_TIMER["OnTimer"]
            S_DEINIT["OnDeinit"]
        end

        subgraph s_processing["Signal Processing"]
            PROC["ProcessTradeSignals"]
            PARSE["ParseTradeSignal"]
            FILTER["ShouldProcessTrade"]
        end

        subgraph s_execution["Trade Execution"]
            EXEC_O["ExecuteOpenTrade"]
            EXEC_C["ExecuteCloseTrade"]
            EXEC_M["ExecuteModifyTrade"]
            LOT["TransformLotSize"]
        end

        subgraph s_mapping["Mapping"]
            ADD_MAP["AddTicketMapping"]
            GET_MAP["GetSlaveTicket"]
            RECOVER["RecoverMappings"]
        end

        subgraph s_comm["Communication"]
            S_SUB["ZMQ SUB 5556 (Unified)"]
            S_PUSH["ZMQ PUSH 5555"]
        end

        S_TIMER --> PROC
        S_TICK --> PROC
        PROC --> S_SUB
        S_SUB --> PARSE
        PARSE --> FILTER

        FILTER --> EXEC_O
        FILTER --> EXEC_C
        FILTER --> EXEC_M

        EXEC_O --> LOT
        EXEC_O --> ADD_MAP
        EXEC_C --> GET_MAP

        S_INIT --> RECOVER
        S_INIT --> S_SUB
    end
```

### web-ui 内部構成

```mermaid
graph TB
    subgraph web_ui["web-ui"]
        subgraph pages["Pages (App Router)"]
            CONN["connections"]
            TG["trade-groups"]
            TGD["trade-groups/:id"]
            INST["installations"]
            SITES["sites"]
            SET["settings"]
        end

        subgraph components["Components"]
            FLOW["ConnectionsViewReactFlow"]
            NODE["AccountNode"]
            EDGE["SettingsEdge"]
            CREATE["CreateConnectionDialog"]
            EDIT["EditConnectionDrawer"]
            MASTER["MasterSettingsDrawer"]
            SLAVE["SlaveSettingsForm"]
        end

        subgraph hooks["Hooks"]
            USC["useSankeyCopier"]
            UFD["useFlowData"]
            UTG["useTradeGroups"]
            UMC["useMasterConfig"]
            USV["useSettingsValidation"]
        end

        subgraph state["State - Jotai"]
            SITE_A["sitesAtom"]
            CONN_A["connectionsAtom"]
            SET_A["settingsAtom"]
            UI_A["UI Atoms"]
        end

        subgraph api["API Layer"]
            CLIENT["ApiClient"]
            WS_C["WebSocket Client"]
        end

        CONN --> FLOW
        FLOW --> NODE
        FLOW --> EDGE
        FLOW --> CREATE
        FLOW --> EDIT

        NODE --> MASTER
        MASTER --> SLAVE

        FLOW --> USC
        USC --> UFD
        USC --> UTG
        USC --> UMC

        USC --> SITE_A
        USC --> CONN_A
        USC --> SET_A

        USC --> CLIENT
        USC --> WS_C
    end
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

2ポートアーキテクチャ: Receiver (PULL) と Publisher (統合PUB) のみ使用。

| ポート | 用途 |
|--------|------|
| 3000 | relay-server REST API (HTTPS) |
| 5555 | ZeroMQ PULL (EA→サーバー) |
| 5556 | ZeroMQ PUB (トレードシグナル + 設定配布 統合) |
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

## デプロイメント図

### 実行環境構成

```mermaid
graph TB
    subgraph user_pc["User PC - Windows"]
        subgraph mt_instances["MetaTrader Instances"]
            MT1["MT4 or MT5 - Master"]
            MT2["MT4 or MT5 - Slave 1"]
            MT3["MT4 or MT5 - Slave 2"]
        end

        subgraph eas_dll["EAs and DLL"]
            EA1["Master EA"]
            EA2["Slave EA"]
            EA3["Slave EA"]
            DLL["sankey_copier_zmq.dll"]
        end

        subgraph relay_process["relay-server Process"]
            RS["relay-server.exe"]
            DB[("sankey_copier.db")]
            CERTS["certs"]
            LOGS["logs"]
        end

        subgraph browser["Browser"]
            WEB["web-ui localhost 3000"]
        end

        MT1 --> EA1
        MT2 --> EA2
        MT3 --> EA3
        EA1 --> DLL
        EA2 --> DLL
        EA3 --> DLL

        DLL <-->|TCP 5555-5556| RS
        RS --> DB
        RS --> CERTS
        RS --> LOGS

        WEB <-->|HTTPS| RS
    end

    subgraph vlogs["VictoriaLogs - Optional"]
        VL["VictoriaLogs 9428"]
    end

    RS -->|HTTP POST| VL
    DLL -->|HTTP POST| VL
```

### ポート構成詳細

2ポートアーキテクチャ: トレードシグナルと設定配布を統合PUBポートで配信。

```mermaid
graph LR
    subgraph relay_ports["relay-server"]
        P3000["3000 - HTTPS REST, WebSocket"]
        P5555["5555 - ZMQ PULL"]
        P5556["5556 - ZMQ PUB (Unified)"]
    end

    subgraph external["External"]
        P9428["9428 - VictoriaLogs"]
    end

    subgraph clients["Clients"]
        BROWSER["Browser"]
        EA_M["Master EA"]
        EA_S["Slave EA"]
    end

    BROWSER <-->|HTTPS| P3000
    EA_M -->|PUSH| P5555
    EA_S -->|PUSH| P5555
    P5556 -->|SUB| EA_M
    P5556 -->|SUB| EA_S
```

### ファイル配置

```
C:\Users\{User}\
├── AppData\Roaming\MetaQuotes\Terminal\{ID}\
│   └── MQL5\
│       ├── Experts\
│       │   ├── SankeyCopierMaster.ex5
│       │   └── SankeyCopierSlave.ex5
│       ├── Libraries\
│       │   └── sankey_copier_zmq.dll
│       └── Include\
│           └── SankeyCopier\
│               ├── Common.mqh
│               ├── Zmq.mqh
│               └── ...
│
└── SANKEY-Copier\  (または任意のディレクトリ)
    ├── relay-server.exe
    ├── config.toml
    ├── sankey_copier.db
    ├── certs\
    │   ├── server.pem
    │   └── server-key.pem
    └── logs\
        └── sankey-copier-server.YYYY-MM-DD.log
```

### 通信プロトコル

| 通信路 | プロトコル | 暗号化 | 用途 |
|--------|-----------|--------|------|
| Browser ↔ relay-server | HTTPS (REST/WS) | TLS 1.3 | 設定管理・監視 |
| EA ↔ relay-server | ZeroMQ/TCP | なし (localhost) | トレードシグナル |
| relay-server → VictoriaLogs | HTTP POST | なし/TLS | ログ送信 |

### プロセス起動順序

```mermaid
sequenceDiagram
    participant User
    participant RS as relay-server
    participant MT as MetaTrader
    participant EA as EA (Master/Slave)
    participant WEB as Browser

    User->>RS: 1. relay-server.exe 起動
    RS->>RS: DB初期化、ZMQソケット作成

    User->>MT: 2. MetaTrader 起動
    User->>EA: 3. EAをチャートにアタッチ
    EA->>RS: 4. Heartbeat送信開始
    RS->>RS: EA自動登録

    User->>WEB: 5. https://localhost:3000 アクセス
    WEB->>RS: 接続・設定取得
    WEB->>User: ダッシュボード表示
```

## ドキュメント一覧

- [relay-server](./relay-server.md) - 中継サーバーの詳細
- [mt-bridge](./mt-bridge.md) - 通信DLLの詳細
- [mt-advisors](./mt-advisors.md) - EAの詳細
- [web-ui](./web-ui.md) - WebUIの詳細
