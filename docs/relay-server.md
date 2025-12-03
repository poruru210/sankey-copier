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
        +i32 master_runtime_status
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
        +i32 runtime_status
        +bool enabled_flag
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
| GET | `/api/runtime-status-metrics` | RuntimeStatusUpdater が計測した評価回数/失敗率/クラスターサイズのスナップショット |

### Status Engine によるフィールドの分担

| フィールド | 役割 | 更新主体 |
|------------|------|-----------|
| `enabled_flag` | ユーザー意図 (UI トグル状態)。Master/Slave とも `POST /toggle` API でのみ変更。 | Web UI / API クライアント |
| `runtime_status` | Status Engine が算出する実効ステータス (`0=Manual OFF / 1=Standby / 2=Streaming`)。 | Relay Server (Status Engine) |
| `master_runtime_status` | Master 単位の実効ステータス。Master ノード可視化や ZMQ Config builder に利用。 | Relay Server |
| `warning_codes` | Master/Slave ごとの警告配列。`enum WarningCode` で型付けされ、Config/REST/WebSocket が同じ内容を返す。 | Relay Server |

- `enabled_flag` を書き換えた後、Status Engine は最新の接続状況や EA からの Heartbeat を参照して `runtime_status`/`master_runtime_status` を再計算し、DBへ書き戻す。<br>
- Web UI や EA は `runtime_status` 系フィールドを観測値として扱い、意図 (`enabled_flag`) と実行 (`runtime_status`) のギャップを UI/ログで可視化する。<br>
- ZeroMQ の `allow_new_orders` は `runtime_status == 2` のときのみ `true` となり、EA サイドのコピー可否フラグと同期する。
- `warning_codes` は Nord バーの色決定や CS ログの根拠として使われ、Master 側の `MasterOffline` などクラスタ情報が含まれる。

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
例: "config/IC_Markets_12345 <binary data>"
```

### トピックルーティング

2ポートアーキテクチャでは、統合PUBソケット(5556)から全メッセージを配信し、トピックでフィルタリングします。

#### トピック形式と用途

| トピック形式 | 用途 | 配信先 | 実装箇所 |
|------------|------|--------|---------|
| `config/{account_id}` | Master/Slave設定配布 | 特定EA<br>※account_id = master_accountまたはslave_account | `trade_groups.rs` L311, `trade_group_members.rs` L628 |
| `trade/{master_account}/{slave_account}` | トレードシグナル配信 | 特定Slave | `config_publisher.rs` L163 |
| `config/global` | VictoriaLogs設定等 | 全EA | `config_publisher.rs` L138 |

**例**:
- Master設定: `config/IC_Markets_123456` (account_id = master_account)
- Slave設定: `config/XM_789012` (account_id = slave_account)
- トレードシグナル: `trade/IC_Markets_123456/XM_789012`
- グローバル設定: `config/global`

**注**: 
- `account_id`は`{BrokerName}_{AccountNumber}`形式（例: `IC_Markets_123456`）
- Master/Slaveとも同じ`config/{account_id}`形式を使用（EA側で自身のaccount_idでフィルタリング）

#### 動的トピック生成（EA側）

EA側ではFFI関数を使用してトピックを動的に生成します (`mt-bridge/src/ffi.rs`):

```mql5
// Master/Slave設定トピック生成
ushort topic_buffer[256];
int len = build_config_topic(AccountID, topic_buffer, 256);
// 結果: "config/{AccountID}"
// AccountID形式: "IC_Markets_123456" (BrokerName_AccountNumber)

// トレードシグナルトピック生成 (Slave側)
int len = build_trade_topic(MasterAccountID, SlaveAccountID, topic_buffer, 256);
// 結果: "trade/{MasterAccountID}/{SlaveAccountID}"
// 例: "trade/IC_Markets_123456/XM_789012"

// グローバル設定トピック取得
int len = get_global_config_topic(topic_buffer, 256);
// 結果: "config/global"
```

#### ConfigMessageトレイト

Relay Server側では`ConfigMessage`トレイトでトピック生成を統一 (`sankey_copier_zmq` crate):

```rust
pub trait ConfigMessage: serde::Serialize {
    fn zmq_topic(&self) -> String;
}

impl ConfigMessage for MasterConfigMessage {
    fn zmq_topic(&self) -> String {
        format!("config/{}", self.account_id)
    }
}

impl ConfigMessage for SlaveConfigMessage {
    fn zmq_topic(&self) -> String {
        format!("config/{}", self.account_id)
    }
}
```


## Config Builder

- `relay-server/src/config_builder.rs` が `MasterConfigMessage`/`SlaveConfigMessage` の生成を一手に引き受け、Heartbeat・REST API・config_request・unregister のすべての経路で同じステータス計算ロジックを再利用しています。
- `status_engine.rs` から返される `MasterStatusResult`/`SlaveStatusResult` をビルダーが受け取り、Slave 設定メッセージの `allow_new_orders` は `runtime_status == CONNECTED` のときだけ `true` になるようにしています。
- API やハンドラはビルダーのステータス結果をデータベースの `runtime_status` に書き戻し、Web UI は `enabled_flag`（ユーザー意図）と `runtime_status`（サーバ側判定）の 2 層情報で表示しつつ、EA には `allow_new_orders` をそのまま渡せるようになりました。

### RuntimeStatusUpdater サービス

- `relay-server/src/runtime_status_updater.rs` に `RuntimeStatusUpdater` が追加され、Heartbeat / Timeout / Intent API / RequestConfig / Unregister のすべてがこのサービスを経由して `trade_group_members.runtime_status` を再計算します。
- `master_cluster_snapshot` と `slave_connection_snapshot` をまとめて取得し、Config Builder で使われる `SlaveConfigBundle` を生成すると同時に DB 更新・ZMQ 配信・`warning_codes` 算出まで一貫して扱います。
- Master/Slave 双方の評価結果は `RuntimeStatusMetrics` に記録され、`GET /api/runtime-status-metrics` エンドポイントも同じスナップショットを返します。Grafana や VictoriaLogs から呼び出すことで、Heartbeat スパイクや評価失敗を即座に検知できます。
- `warning_codes` は Master/Slave 用の enum に整理され、`config` メッセージ / REST 応答 / WebSocket で同じ配列が共有されます。Nord バーは `warning_codes` が空でない場合に黄色へフォールバックし、EA も原因をログできるようになりました。


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

## ステータス判定ロジック

### ステータス値の定義

| 値 | 名称 | 対象 | 説明 |
|----|------|------|------|
| -1 | NO_CONFIG | Master/Slave | 設定未受信またはリセット状態 |
| 0 | DISABLED | Master/Slave | 無効化（Web UI OFF または EA自動売買OFF） |
| 1 | ENABLED | Slave専用 | Slave有効だがMasterが未接続 |
| 2 | CONNECTED | Master/Slave | 完全に有効（トレード実行可能） |

### Masterのステータス判定

**判定要素**:
1. **Web UI Switch**: Web UI上でON/OFFが切り替えられているか
2. **EA自動売買許可**: MT4/MT5のEA側で自動売買が許可されているか (`is_trade_allowed`)

**判定ルール**:

| Web UI Switch | EA自動売買 | ステータス | 説明 |
|:-------------:|:----------:|:----------:|------|
| ✅ ON | ✅ ON | `CONNECTED (2)` | トレードシグナル送信可能 |
| ❌ OFF | - | `DISABLED (0)` | Web UIでOFF |
| ✅ ON | ❌ OFF | `DISABLED (0)` | EA自動売買がOFF |

**実装の所在**: `relay-server/src/models/status_engine.rs`

`MasterIntent` (Web UI のトグル状態) と `ConnectionSnapshot` (Heartbeat が持つ接続状態 / `is_trade_allowed`) を `evaluate_master_status(intent, snapshot)` に渡す。Status Engine が `MasterStatusResult { status }` を返し、その値が `master_runtime_status` として DB/WebSocket に書き込まれる。

### Slaveのステータス判定

**判定要素**:
1. **Slave自体の条件**:
   - Web UI Switch (ON/OFF)
   - EA自動売買許可 (`is_trade_allowed`)
2. **接続Masterの状態**:
   - 接続している各Masterが `CONNECTED` か `DISABLED` か

**判定ルール**:

| Slave自体の条件 | 接続Masterの状態 | Slaveのステータス | 説明 |
|----------------|------------------|:----------------:|------|
| Switch❌ または 自動売買❌ | - | `DISABLED (0)` | Slave自体が無効 |
| Switch✅ かつ 自動売買✅ | **少なくとも1つのMasterが DISABLED** | `ENABLED (1)` | Slave準備完了だがMaster未接続 |
| Switch✅ かつ 自動売買✅ | **すべてのMasterが CONNECTED** | `CONNECTED (2)` | コピー取引実行可能 |

**実装の所在**: `relay-server/src/models/status_engine.rs`

`SlaveIntent` と Slave の `ConnectionSnapshot` に加え、関連する Master の `runtime_status` を `MasterClusterSnapshot::new(vec![...])` に詰めて `evaluate_slave_status(intent, slave_conn, cluster)` を呼び出す。Cluster 内のすべてが `STATUS_CONNECTED` なら Slave も `CONNECTED`、それ以外は `ENABLED`。Status Engine は `allow_new_orders` を同時に算出し、Config Builder (`config_builder.rs`) 経由で EA に届ける。

### N:N接続のサポート

このシステムはMasterとSlaveのN:N接続を完全にサポートします。

**例**: Slave Aが Master1, Master2, Master3 に接続している場合

| Master1 | Master2 | Master3 | Slave Aのステータス |
|:-------:|:-------:|:-------:|:------------------:|
| CONNECTED | CONNECTED | CONNECTED | `CONNECTED (2)` |
| CONNECTED | CONNECTED | DISABLED | `ENABLED (1)` |
| DISABLED | DISABLED | DISABLED | `ENABLED (1)` |

**ルール**: **すべてのMasterが `CONNECTED` の場合のみ Slaveは `CONNECTED (2)`、それ以外は `ENABLED (1)`**

### STATUS_NO_CONFIG (-1) の用途

1. **EA起動直後の初期状態**: 設定メッセージ未受信
2. **Member削除時のリセット通知**: EAに設定削除を通知
3. **設定エラー時のフォールバック**: 異常状態からのリカバリ

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

## 動的ポート割り当て

ZeroMQポートを `0` に設定すると、OSが利用可能なポートを自動的に割り当てます。
これにより、ポート競合を回避し、複数インスタンスの同時実行が可能になります。

### 動作フロー

```mermaid
sequenceDiagram
    participant Config as config.toml
    participant Runtime as runtime.toml
    participant Server as relay-server
    participant EA as MT EA

    Note over Config: receiver_port = 0<br/>sender_port = 0

    Server->>Config: 設定読み込み
    Server->>Server: ポート0検出 → 動的割り当て

    alt runtime.toml存在
        Server->>Runtime: 既存ポート読み込み
        Server->>Server: 既存ポート再利用を試行
        alt ポート使用可能
            Server->>Server: 既存ポートでバインド
        else ポート競合
            Server->>Server: 新規ポート割り当て
            Server->>Runtime: 新ポート保存
        end
    else runtime.toml不在
        Server->>Server: 新規ポート割り当て
        Server->>Runtime: ポート保存
    end

    Server->>EA: EAインストール時にINI生成
    Note over EA: sankey_copier.ini<br/>ReceiverPort=xxxxx<br/>PublisherPort=xxxxx
```

### runtime.toml

動的に割り当てられたポートは `runtime.toml` に永続化されます。
次回起動時に同じポートを再利用し、EAとの設定整合性を維持します。

```toml
# Auto-generated - DO NOT EDIT
# Dynamic port configuration for ZeroMQ

[zeromq]
receiver_port = 15555
sender_port = 15556
generated_at = "2024-01-15T10:30:00Z"
```

### 設定例

**固定ポート（デフォルト）**:
```toml
[zeromq]
receiver_port = 5555
sender_port = 5556
```

**動的ポート（自動割り当て）**:
```toml
[zeromq]
receiver_port = 0
sender_port = 0
```

### Web-UIでのポート表示

Settings画面のZeroMQ設定セクションで、現在使用中のポートと動的割り当て状態を確認できます。
- `is_dynamic: true` - 動的に割り当てられたポート
- `generated_at` - ポート生成日時（動的の場合のみ）

### EAとの連携

MTインストール時に `sankey_copier.ini` が生成され、EAはこのファイルからポート設定を読み込みます。
ポート変更時はEAの再インストールが必要です（Web-UI Installationsページから実行）。

## Phase4: MT EA のステータス同期 (P9)

- Relay Server の Status Engine は `runtime_status`/`allow_new_orders` を `ConfigBuilder` 経由で統一しているため、MT4/MT5 側はこの値をそのまま表示・実行条件として扱うべきです。
- 詳細な作業手順・テスト計画は `plan/p9-mt-advisor-plan.md` を参照してください。現フェーズでは `allow_new_orders` が `CONNECTED` 時のみ `true` となり、`status` は Web UI に合わせた状態表示用として維持されています。
- MT EA では `ProcessConfigMessage` で受信した `CopyConfig.status`/`allow_new_orders` を `GridPanel` に反映しつつ、`Open` シグナルは `allow_new_orders` を起点に、`Close`/`Modify` は常に許可する動作でサーバ側判定と整合させます。

## 関連コンポーネント

- [mt-bridge](./mt-bridge.md): EA↔サーバー通信用DLL
- [mt-advisors](./mt-advisors.md): MT4/MT5用EA
- [web-ui](./web-ui.md): 設定・監視用Webインターフェース
