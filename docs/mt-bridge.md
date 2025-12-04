# mt-bridge

MT4/MT5のEAとrelay-server間の通信を担うRust製DLL。ZeroMQによる高速メッセージングとMessagePackによる効率的なシリアライズを提供する。

## 責務

1. **ZeroMQ通信**: PUSH/PULL/PUB/SUBソケットの管理
2. **MessagePackシリアライズ**: メッセージの効率的なバイナリ変換
3. **FFIインターフェース**: MQL4/MQL5から呼び出し可能なC API提供
4. **VictoriaLogsクライアント**: HTTP経由のログ送信

## アーキテクチャ

```mermaid
graph LR
    subgraph "MT4/MT5"
        EA[Expert Advisor<br/>MQL4/MQL5]
    end

    subgraph "mt-bridge DLL"
        FFI[FFI Layer]
        ZMQ[ZeroMQ Wrapper]
        MP[MessagePack<br/>Serializer]
        VL[VictoriaLogs<br/>Client]
    end

    subgraph "relay-server"
        RS[ZMQ Sockets]
    end

    subgraph "VictoriaLogs"
        VLS[HTTP Endpoint]
    end

    EA -->|DLL Import| FFI
    FFI --> ZMQ
    FFI --> MP
    FFI --> VL
    ZMQ -->|TCP| RS
    VL -->|HTTP POST| VLS
```

## ディレクトリ構造

```
mt-bridge/
├── src/
│   ├── lib.rs                    # ZMQ FFIラッパー (メインエントリ)
│   ├── ffi.rs                    # ZMQ/MessagePack 統合FFI
│   ├── ffi_helpers.rs            # UTF-16変換・ハンドル管理ヘルパー
│   ├── types.rs                  # メッセージ型定義
│   ├── traits.rs                 # ConfigMessageトレイト
│   ├── victoria_logs.rs          # VictoriaLogsクライアント
│   ├── symbol_filter_tests.rs    # シンボルフィルターテスト
│   └── msgpack/
│       ├── mod.rs                # モジュール定義
│       ├── serialization.rs      # シリアライズ関数
│       └── tests/                # シリアライズテスト
├── build.rs                      # バージョン埋め込み
└── Cargo.toml
```

## ビルド成果物

| ファイル | 説明 |
|---------|------|
| `sankey_copier_zmq.dll` | Windows DLL (32-bit/64-bit) |

DLLにはWindowsリソース情報が埋め込まれる:
- ProductName: SANKEY Copier ZMQ DLL
- バージョン: Gitタグから自動取得

## データモデル

```mermaid
classDiagram
    class HeartbeatMessage {
        +String message_type
        +String account_id
        +String ea_type
        +String platform
        +i64 account_number
        +String broker
        +String server
        +f64 balance
        +f64 equity
        +String currency
        +i64 leverage
        +bool is_trade_allowed
    }

    class SlaveConfigMessage {
        +String account_id
        +String master_account
        +String trade_group_id
        +i32 status
        +LotCalculationMode lot_calculation_mode
        +Option~f64~ lot_multiplier
        +bool reverse_trade
        +Vec~SymbolMapping~ symbol_mappings
        +TradeFilters filters
        +SyncMode sync_mode
        +i32 max_slippage
        +i32 max_retries
        +i32 max_signal_delay_ms
        +u32 config_version
    }

    class MasterConfigMessage {
        +String account_id
        +Option~String~ symbol_prefix
        +Option~String~ symbol_suffix
        +u32 config_version
    }

    class TradeSignalMessage {
        +String action
        +i64 ticket
        +Option~String~ symbol
        +Option~String~ order_type
        +Option~f64~ lots
        +Option~f64~ open_price
        +Option~f64~ stop_loss
        +Option~f64~ take_profit
        +Option~i64~ magic_number
        +String source_account
        +Option~f64~ close_ratio
    }

    class PositionSnapshotMessage {
        +String message_type
        +String source_account
        +Vec~PositionInfo~ positions
        +String timestamp
    }

    class VLogsConfigMessage {
        +bool enabled
        +String endpoint
        +i32 batch_size
        +i32 flush_interval_secs
        +String log_level
    }

    class ConfigMessage {
        <<trait>>
        +account_id() String
        +config_version() u32
        +timestamp() String
        +zmq_topic() String
    }

    ConfigMessage <|.. SlaveConfigMessage
    ConfigMessage <|.. MasterConfigMessage
```

## FFI関数一覧

### ZMQ基本操作 (lib.rs)

| 関数 | 説明 | 戻り値 |
|------|------|--------|
| `zmq_context_create()` | コンテキスト作成 | handle (≥0) / -1 |
| `zmq_context_destroy(handle)` | コンテキスト破棄 | 0 / -1 |
| `zmq_socket_create(ctx, type)` | ソケット作成 | handle / -1 |
| `zmq_socket_destroy(handle)` | ソケット破棄 | 0 / -1 |
| `zmq_socket_bind(handle, endpoint)` | バインド | 0 / -1 |
| `zmq_socket_connect(handle, endpoint)` | 接続 | 0 / -1 |
| `zmq_socket_send(handle, msg)` | テキスト送信 | 0 / -1 |
| `zmq_socket_send_binary(handle, data, len)` | バイナリ送信 | 0 / -1 |
| `zmq_socket_receive(handle, buf, size)` | 受信 | bytes / 0 / -1 |
| `zmq_socket_subscribe_all(handle)` | 全トピック購読 | 0 / -1 |
| `zmq_socket_subscribe(handle, topic)` | トピック購読 | 0 / -1 |

### トピックヘルパー (constants.rs / ffi.rs)

| 関数 | 説明 | 戻り値 |
|------|------|--------|
| `build_sync_topic_ffi(master, slave)` | sync/トピック生成 | UTF-16文字列 |
| `get_sync_topic_prefix()` | sync/プレフィックス取得 | UTF-16文字列 |

**トピック形式**:
- Config: `config/{account_id}`
- Trade: `trade/{master_id}/{slave_id}`
- Sync: `sync/{master_id}/{slave_id}`

注: 同一の PUB ソケット (unified PUB) で複数のトピックを配信する方式を採っています。トピックは論理的なルーティング文字列です。

ソケットタイプ:
- `ZMQ_PUB` = 1
- `ZMQ_SUB` = 2
- `ZMQ_PULL` = 7
- `ZMQ_PUSH` = 8

### MessagePackシリアライズ (ffi.rs)

| 関数 | 説明 |
|------|------|
| `serialize_heartbeat(...)` | Heartbeatをシリアライズ |
| `serialize_request_config(...)` | ConfigRequestをシリアライズ |
| `serialize_trade_signal(...)` | TradeSignalをシリアライズ |
| `serialize_unregister(...)` | Unregisterをシリアライズ |
| `get_serialized_buffer()` | シリアライズ結果のポインタ取得 |
| `copy_serialized_buffer(dest, len)` | シリアライズ結果をコピー |

### MessagePackパース (ffi.rs)

| 関数群 | 説明 |
|--------|------|
| `parse_slave_config(data, len)` → `slave_config_free(ptr)` | Slave設定パース |
| `parse_master_config(data, len)` → `master_config_free(ptr)` | Master設定パース |
| `parse_trade_signal(data, len)` → `trade_signal_free(ptr)` | トレードシグナルパース |
| `parse_position_snapshot(data, len)` → `position_snapshot_free(ptr)` | ポジションスナップショットパース |
| `parse_vlogs_config(data, len)` → `vlogs_config_free(ptr)` | VLogs設定パース |

### フィールドアクセサ (ffi.rs)

```c
// Slave Config
slave_config_get_string(ptr, field) → *const u16
slave_config_get_double(ptr, field) → f64
slave_config_get_int(ptr, field) → i32
slave_config_get_bool(ptr, field) → bool

// Symbol Mappings
slave_config_get_symbol_mappings_count(ptr) → i32
slave_config_get_mapping_source(ptr, idx) → *const u16
slave_config_get_mapping_target(ptr, idx) → *const u16

// Filters
slave_config_get_allowed_magic_count(ptr) → i32
slave_config_get_allowed_magic_at(ptr, idx) → i32

// Trade Signal
trade_signal_get_string(ptr, field) → *const u16
trade_signal_get_double(ptr, field) → f64
trade_signal_get_int(ptr, field) → i32

// Position Snapshot
position_snapshot_get_positions_count(ptr) → i32
position_snapshot_get_position_string(ptr, idx, field) → *const u16
position_snapshot_get_position_double(ptr, idx, field) → f64
```

### VictoriaLogs (victoria_logs.rs)

| 関数 | 説明 |
|------|------|
| `vlogs_configure(endpoint, source)` | ログ送信先設定 |
| `vlogs_disable()` | ログ送信無効化 |
| `vlogs_add_entry(level, category, msg, ctx)` | ログエントリ追加 |
| `vlogs_flush()` | バッファをフラッシュ |
| `vlogs_buffer_size()` | バッファサイズ取得 |

## 通信フロー

### EA → relay-server (送信)

```mermaid
sequenceDiagram
    participant EA as MQL EA
    participant FFI as FFI Layer
    participant SER as Serializer
    participant ZMQ as ZMQ Socket
    participant RS as relay-server

    EA->>FFI: serialize_heartbeat(...)
    FFI->>SER: build HeartbeatMessage
    SER->>SER: rmp_serde::to_vec()
    SER-->>FFI: buffer_len

    EA->>FFI: get_serialized_buffer()
    FFI-->>EA: *const u8

    EA->>FFI: zmq_socket_send_binary(handle, ptr, len)
    FFI->>ZMQ: sock.send(bytes)
    ZMQ->>RS: TCP packet
```

### relay-server → EA (受信)

```mermaid
sequenceDiagram
    participant RS as relay-server
    participant ZMQ as ZMQ Socket
    participant FFI as FFI Layer
    participant PAR as Parser
    participant EA as MQL EA

    RS->>ZMQ: PUB message
    EA->>FFI: zmq_socket_receive(handle, buf, size)
    FFI->>ZMQ: sock.recv_bytes(DONTWAIT)

    alt メッセージあり
        ZMQ-->>FFI: bytes
        FFI-->>EA: len > 0
        EA->>FFI: parse_slave_config(data, len)
        FFI->>PAR: rmp_serde::from_slice()
        PAR-->>FFI: SlaveConfigMessage
        FFI-->>EA: *mut SlaveConfigMessage

        EA->>FFI: slave_config_get_string(ptr, "status")
        FFI-->>EA: UTF-16 string

        EA->>FFI: slave_config_free(ptr)
    else メッセージなし
        ZMQ-->>FFI: EAGAIN
        FFI-->>EA: 0
    end
```

## UTF-16文字列処理

MQL5はUTF-16文字列を使用するため、Rust側で変換処理を行う。

```mermaid
graph LR
    subgraph "MQL側"
        MQL[string / wchar_t*]
    end

    subgraph "FFI境界"
        U16[*const u16<br/>null終端]
    end

    subgraph "Rust側"
        STR[String / &str]
    end

    MQL -->|引数渡し| U16
    U16 -->|utf16_to_string| STR
    STR -->|string_to_utf16_buffer| U16
    U16 -->|戻り値| MQL
```

戻り値用のバッファはラウンドロビン方式で4スロットを使用:
```rust
static STRING_BUFFER_1: [u16; 512]
static STRING_BUFFER_2: [u16; 512]
static STRING_BUFFER_3: [u16; 512]
static STRING_BUFFER_4: [u16; 512]
static BUFFER_INDEX: usize  // 0→1→2→3→0...
```

## VictoriaLogsクライアント

```mermaid
sequenceDiagram
    participant EA as MQL EA
    participant FFI as FFI Layer
    participant BUF as Buffer
    participant THR as Background Thread
    participant VL as VictoriaLogs

    EA->>FFI: vlogs_configure(endpoint, source)
    FFI->>THR: spawn flush thread

    loop ログ追加
        EA->>FFI: vlogs_add_entry(level, cat, msg, ctx)
        FFI->>BUF: push LogEntry
    end

    EA->>FFI: vlogs_flush()
    FFI->>THR: send FlushMessage

    THR->>BUF: drain entries
    THR->>VL: HTTP POST /insert/jsonline
    Note right of VL: Content-Type:<br/>application/x-ndjson
```

ログエントリ形式 (JSON Lines):
```json
{"_msg":"Trade opened","level":"INFO","category":"Trade","source":"ea:master:IC_Markets_123","ts":"2024-01-01T00:00:00Z"}
```

## エラーハンドリング

| 状況 | 戻り値 |
|------|--------|
| 正常終了 | 0 または有効なハンドル |
| nullポインタ | -1 または null |
| インデックス範囲外 | -1 または null |
| ZMQエラー | -1 |
| MessagePackパースエラー | null |
| UTF-16変換エラー | null |

非ブロッキング受信:
- メッセージあり: 受信バイト数 (> 0)
- メッセージなし: 0 (EAGAIN)
- エラー: -1

## MQLからの使用例

```mql5
#import "sankey_copier_zmq.dll"
    int zmq_context_create();
    int zmq_socket_create(int ctx, int type);
    int zmq_socket_connect(int sock, string endpoint);
    int zmq_socket_send_binary(int sock, uchar &data[], int len);
    int serialize_heartbeat(string account_id, double balance, ...);
    uchar get_serialized_buffer();
#import

int g_context;
int g_socket;

int OnInit() {
    g_context = zmq_context_create();
    g_socket = zmq_socket_create(g_context, 8);  // PUSH
    zmq_socket_connect(g_socket, "tcp://localhost:5555");
    return INIT_SUCCEEDED;
}

void SendHeartbeat() {
    int len = serialize_heartbeat(
        account_id, balance, equity, positions,
        timestamp, version, ea_type, platform,
        account_number, broker, account_name,
        server, currency, leverage, is_trade_allowed,
        symbol_prefix, symbol_suffix, symbol_map
    );

    if (len > 0) {
        uchar buffer[];
        ArrayResize(buffer, len);
        copy_serialized_buffer(buffer, len);
        zmq_socket_send_binary(g_socket, buffer, len);
    }
}
```

## 関連コンポーネント

- [relay-server](./relay-server.md): 通信先サーバー
- [mt-advisors](./mt-advisors.md): このDLLを使用するEA
- [web-ui](./web-ui.md): 設定・監視UI
