# relay-server

トレードコピーシステムの中核となるRust製サーバー。MT4/MT5のEAとweb-ui間の仲介役として、設定管理・トレードシグナルの中継・接続管理を行う。

## 責務

1. **EA接続管理**: Master/Slave EAのHeartbeatを受信し、接続状態を追跡
2. **設定配布**: web-uiからの設定変更をEAにリアルタイム配信
3. **トレードシグナル中継**: MasterからのトレードシグナルをフィルタリングしてSlaveに配信
4. **REST API提供**: web-uiへの設定・接続情報APIを提供
5. **WebSocket通知**: リアルタイムイベントをweb-uiにブロードキャスト

## アーキテクチャ

```mermaid
graph TB
    subgraph "MT4/MT5"
        MEA[Master EA]
        SEA[Slave EA]
    end

    subgraph "mt-bridge DLL"
        DLL[sankey_copier_zmq.dll]
    end

    subgraph "relay-server"
        ZMQ_PULL[ZMQ PULL :5555]
        ZMQ_PUB[ZMQ PUB :5556<br/>Unified - Trades & Config]

        MH[MessageHandler]
        CE[CopyEngine]
        CM[ConnectionManager]
        DB[(SQLite)]
        API[REST API :3000]
        WS[WebSocket /ws]
    end

    subgraph "web-ui"
        UI[React App]
    end

    MEA --> DLL
    SEA --> DLL
    DLL -->|PUSH| ZMQ_PULL
    ZMQ_PUB -->|SUB| DLL

    ZMQ_PULL --> MH
    MH --> CE
    MH --> CM
    CE --> ZMQ_PUB
    MH --> ZMQ_PUB
    CM --> DB

    API --> DB
    API --> CM
    UI -->|HTTP| API
    WS -->|Events| UI
```

## ディレクトリ構造

```
relay-server/
├── src/
│   ├── main.rs                    # エントリポイント
│   ├── lib.rs                     # ライブラリ公開インターフェース
│   ├── config.rs                  # TOML設定管理
│   ├── cert.rs                    # TLS証明書管理
│   ├── api/                       # REST APIエンドポイント
│   │   ├── mod.rs                 # ルータ定義
│   │   ├── connections.rs         # EA接続情報API
│   │   ├── trade_groups.rs        # Master設定API
│   │   ├── trade_group_members.rs # Slave設定API
│   │   ├── websocket.rs           # WebSocket
│   │   └── ...
│   ├── models/                    # データモデル
│   │   ├── connection.rs          # EaConnection
│   │   ├── trade_group.rs         # TradeGroup (Master)
│   │   └── trade_group_member.rs  # TradeGroupMember (Slave)
│   ├── db/                        # データベース操作
│   │   ├── trade_groups.rs        # TradeGroup CRUD
│   │   ├── trade_group_members.rs # Member CRUD
│   │   └── config_distribution.rs # 設定配布ロジック
│   ├── zeromq/                    # ZeroMQ通信
│   │   ├── mod.rs                 # ZmqServer, ZmqPublisher
│   │   └── config_publisher.rs    # 設定配信
│   ├── connection_manager/        # EA接続管理
│   │   └── mod.rs                 # ConnectionManager
│   ├── engine/                    # コピーエンジン
│   │   └── mod.rs                 # CopyEngine
│   └── message_handler/           # ZMQメッセージ処理
│       ├── mod.rs                 # MessageHandler
│       ├── heartbeat.rs           # Heartbeat処理
│       ├── trade_signal.rs        # トレードシグナル処理
│       └── config_request.rs      # 設定リクエスト処理
├── config.toml                    # 本番設定
├── config.dev.toml                # 開発環境設定
└── Cargo.toml
```

## データモデル

```mermaid
classDiagram
    class EaConnection {
        +String account_id
        +EaType ea_type
        +Platform platform
        +i64 account_number
        +String broker
        +String server
        +f64 balance
        +f64 equity
        +ConnectionStatus status
        +bool is_trade_allowed
        +DateTime last_heartbeat
    }

    class TradeGroup {
        +String id
        +MasterSettings master_settings
        +String created_at
        +String updated_at
    }

    class MasterSettings {
        +Option~String~ symbol_prefix
        +Option~String~ symbol_suffix
        +u32 config_version
    }

    class TradeGroupMember {
        +i32 id
        +String trade_group_id
        +String slave_account
        +SlaveSettings slave_settings
        +i32 status
        +String created_at
        +String updated_at
    }

    class SlaveSettings {
        +LotCalculationMode lot_calculation_mode
        +Option~f64~ lot_multiplier
        +Option~String~ symbol_prefix
        +Option~String~ symbol_suffix
        +Vec~SymbolMapping~ symbol_mappings
        +TradeFilters filters
        +bool reverse_trade
        +SyncMode sync_mode
        +i32 max_retries
        +i32 max_signal_delay_ms
        +u32 config_version
    }

    class TradeSignal {
        +TradeAction action
        +i64 ticket
        +Option~String~ symbol
        +Option~OrderType~ order_type
        +Option~f64~ lots
        +Option~f64~ open_price
        +Option~f64~ stop_loss
        +Option~f64~ take_profit
        +String source_account
    }

    TradeGroup "1" --> "1" MasterSettings
    TradeGroup "1" --> "*" TradeGroupMember
    TradeGroupMember "1" --> "1" SlaveSettings

    note for EaConnection "status: 0=DISABLED, 1=ENABLED, 2=CONNECTED"
```

## REST APIエンドポイント

| メソッド | パス | 説明 |
|---------|------|------|
| GET | `/api/connections` | 全EA接続情報取得 |
| GET | `/api/connections/:id` | 特定EA接続情報取得 |
| GET | `/api/trade-groups` | 全TradeGroup一覧 |
| GET | `/api/trade-groups/:id` | TradeGroup詳細取得 |
| PUT | `/api/trade-groups/:id` | Master設定更新 |
| DELETE | `/api/trade-groups/:id` | TradeGroup削除 |
| GET | `/api/trade-groups/:id/members` | Slave一覧取得 |
| POST | `/api/trade-groups/:id/members` | Slave追加 |
| PUT | `/api/trade-groups/:id/members/:slave_id` | Slave設定更新 |
| DELETE | `/api/trade-groups/:id/members/:slave_id` | Slave削除 |
| POST | `/api/trade-groups/:id/members/:slave_id/toggle` | Slave有効/無効切替 |
| GET | `/api/logs` | サーバーログ取得 |
| GET | `/api/mt-installations` | MTインストール検出 |

## WebSocketイベント

| イベント | 説明 |
|---------|------|
| `trade_received:{account}:{symbol}:{lots}` | トレード受信 |
| `trade_copied:{account}:{symbol}:{lots}:{member}` | トレード複製完了 |
| `ea_disconnected:{account}` | EA切断 |
| `trade_group_updated:{json}` | TradeGroup更新 |
| `member_added:{json}` | Member追加 |
| `member_updated:{json}` | Member更新 |
| `member_deleted:{id}` | Member削除 |

## ZeroMQ通信

### ポート構成

2ポートアーキテクチャ: Receiver (PULL) と Publisher (統合PUB) のみ使用。

| ポート | タイプ | 用途 |
|-------|-------|------|
| 5555 | PULL | EA→サーバー (Heartbeat, TradeSignal等) |
| 5556 | PUB | サーバー→EA (TradeSignal + Config 統合配信) |

### メッセージフォーマット

すべてのメッセージはMessagePack形式でシリアライズ。

```
PUB/SUB トピック形式: "{topic} {MessagePack payload}"
例: "IC_Markets_12345 <binary data>"
```

## 処理フロー

### Heartbeat処理

```mermaid
sequenceDiagram
    participant EA as Master/Slave EA
    participant DLL as mt-bridge DLL
    participant RS as relay-server
    participant CM as ConnectionManager
    participant DB as SQLite

    EA->>DLL: Heartbeat data
    DLL->>RS: ZMQ PUSH (MessagePack)
    RS->>RS: parse HeartbeatMessage
    RS->>CM: update_heartbeat()

    alt 新規EA
        CM->>CM: auto-register
        CM->>DB: create TradeGroup (if Master)
    else 既存EA
        CM->>CM: update last_heartbeat
    end

    RS->>EA: send VLogsConfig (ZMQ PUB 5556)
```

### トレードシグナル処理

```mermaid
sequenceDiagram
    participant MEA as Master EA
    participant RS as relay-server
    participant CE as CopyEngine
    participant DB as SQLite
    participant SEA as Slave EA

    MEA->>RS: TradeSignal (Open/Close/Modify)
    RS->>RS: parse TradeSignalMessage
    RS->>DB: get TradeGroupMembers

    loop 各Slave
        RS->>CE: should_copy_trade(signal, member)

        alt フィルター通過
            CE->>CE: transform_signal()
            Note right of CE: シンボル変換<br/>prefix/suffix適用
            RS->>SEA: ZMQ PUB (5556)
        else フィルター除外
            Note right of CE: スキップ
        end
    end
```

### 設定更新フロー

```mermaid
sequenceDiagram
    participant UI as web-ui
    participant API as REST API
    participant DB as SQLite
    participant ZMQ as ZmqConfigPublisher
    participant EA as Slave EA

    UI->>API: PUT /api/trade-groups/{id}/members/{slave}
    API->>DB: update slave_settings
    API->>DB: increment config_version
    API->>ZMQ: send_config_to_slave()

    ZMQ->>ZMQ: build SlaveConfigMessage
    Note right of ZMQ: effective_status計算<br/>Master接続状態確認
    ZMQ->>EA: ZMQ PUB (5556 unified)

    EA->>EA: 設定適用
```

## Slaveステータス

| 値 | 名称 | 説明 |
|----|------|------|
| 0 | DISABLED | ユーザーが無効化 |
| 1 | ENABLED | 有効だがMasterオフライン/売買不許可 |
| 2 | CONNECTED | 完全に有効（トレードコピー実行可能） |
| 4 | REMOVED | 削除済み |

`effective_status`の計算ロジック:
- ユーザー設定が無効 → 0 (DISABLED)
- Masterがオフライン → 1 (ENABLED)
- Masterの`is_trade_allowed`がfalse → 1 (ENABLED)
- 上記以外 → 2 (CONNECTED)

## CopyEngine フィルタリング

`should_copy_trade()`で以下を検証:
1. Slaveが`CONNECTED`状態か
2. `copy_pending_orders`設定（指値注文の場合）
3. `source_lot_min` / `source_lot_max`
4. `allowed_symbols` / `blocked_symbols`
5. `allowed_magic_numbers` / `blocked_magic_numbers`

`transform_signal()`で以下を変換:
1. Masterの`symbol_prefix`/`symbol_suffix`を削除
2. `symbol_mappings`を適用
3. Slaveの`symbol_prefix`/`symbol_suffix`を追加

## 設定ファイル

```toml
[server]
host = "0.0.0.0"
port = 3000

[database]
url = "sqlite://sankey_copier.db?mode=rwc"

# 2-port architecture: receiver (PULL) and sender (unified PUB)
[zeromq]
receiver_port = 5555
sender_port = 5556
timeout_seconds = 30

[cors]
disable = false
additional_origins = []

[logging]
enabled = true
directory = "logs"
rotation = "daily"

[tls]
cert_path = "certs/server.pem"
key_path = "certs/server-key.pem"
```

環境別設定の優先順:
1. `config.toml` (ベース)
2. `config.{CONFIG_ENV}.toml` (環境別)
3. `config.local.toml` (ローカル上書き)

## 関連コンポーネント

- [mt-bridge](./mt-bridge.md): EA↔サーバー通信用DLL
- [mt-advisors](./mt-advisors.md): MT4/MT5用EA
- [web-ui](./web-ui.md): 設定・監視用Webインターフェース
