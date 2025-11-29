# Trade Copy Dashboard UI Design

## Overview

VictoriaLogs + Grafana を使用したトレードコピーシステムのトレース・性能評価ダッシュボード設計書。

## Architecture

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   Master EA     │     │  Relay Server   │     │    Slave EA     │
│ (MT4/MT5)       │     │   (Rust)        │     │  (MT4/MT5)      │
└────────┬────────┘     └────────┬────────┘     └────────┬────────┘
         │                       │                       │
         │ vlogs_add_entry()     │ tracing::info!()      │ vlogs_add_entry()
         │                       │                       │
         └───────────────────────┼───────────────────────┘
                                 │
                                 ▼
                    ┌────────────────────────┐
                    │     VictoriaLogs       │
                    │   (localhost:9428)     │
                    └────────────┬───────────┘
                                 │
                                 ▼
                    ┌────────────────────────┐
                    │       Grafana          │
                    │   (VictoriaLogs DS)    │
                    └────────────────────────┘
```

---

## Dashboard Layout

### 1. Overview Dashboard (概要ダッシュボード)

メインダッシュボード。システム全体の健全性と主要KPIを一覧表示。

```
┌────────────────────────────────────────────────────────────────────────┐
│                        Trade Copy Dashboard                            │
├────────────────────────────────────────────────────────────────────────┤
│  Time Range: [Last 1h ▼]  [Refresh: 10s ▼]  [Master: All ▼]           │
├──────────────┬──────────────┬──────────────┬──────────────┬────────────┤
│   Stat #1    │   Stat #2    │   Stat #3    │   Stat #4    │  Stat #5   │
│  Total Recv  │ Copy Success │  Copy Failed │ Success Rate │ Avg Delay  │
│    1,234     │    1,180     │     54       │   95.6%      │   45ms     │
├──────────────┴──────────────┴──────────────┴──────────────┴────────────┤
│                                                                        │
│  ┌─────────────────────────────────────────────────────────────────┐  │
│  │             Trade Volume Over Time (Time Series)                 │  │
│  │  ▄▄▄                                                            │  │
│  │ ▄████▄        ▄▄▄▄                    ▄▄▄                        │  │
│  │████████▄   ▄██████▄▄              ▄▄██████                       │  │
│  │──────────────────────────────────────────────────────────────── │  │
│  │ 10:00    10:15    10:30    10:45    11:00    11:15    11:30     │  │
│  └─────────────────────────────────────────────────────────────────┘  │
│                                                                        │
├───────────────────────────────────┬────────────────────────────────────┤
│     Success/Failure by Master     │      Copy Latency Distribution     │
│  ┌─────────────────────────────┐  │  ┌──────────────────────────────┐  │
│  │ Master_001  ████████░░ 89%  │  │  │      ▄▄▄▄                    │  │
│  │ Master_002  █████████░ 95%  │  │  │    ▄██████▄                  │  │
│  │ Master_003  ███████░░░ 75%  │  │  │  ▄██████████▄                │  │
│  │ Master_004  ██████████ 100% │  │  │▄██████████████▄              │  │
│  └─────────────────────────────┘  │  │ 0   50   100  150  200ms     │  │
│                                   │  └──────────────────────────────┘  │
├───────────────────────────────────┴────────────────────────────────────┤
│                        Error Rate by Type                              │
│  ┌──────────────────────────────────────────────────────────────────┐ │
│  │  [Transform Error: 35%] [Send Failed: 28%] [Member Not Found: 22%]│ │
│  │  [TradeGroup Not Found: 10%] [Other: 5%]                          │ │
│  └──────────────────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────────────────┘
```

---

### 2. Trade Flow Dashboard (トレードフロー追跡)

個々のトレードのエンドツーエンドの追跡を可視化。

```
┌────────────────────────────────────────────────────────────────────────┐
│                        Trade Flow Tracking                             │
├────────────────────────────────────────────────────────────────────────┤
│  Ticket: [________]  Symbol: [All ▼]  Action: [All ▼]  Status: [All ▼] │
├────────────────────────────────────────────────────────────────────────┤
│                                                                        │
│  Trade Flow Timeline                                                   │
│  ┌──────────────────────────────────────────────────────────────────┐ │
│  │                                                                    │ │
│  │ Master_001 ──○────────────────────────────────────────────────▶  │ │
│  │            10:30:00.100                                           │ │
│  │                 │                                                  │ │
│  │                 ▼                                                  │ │
│  │ Relay      ─────○──────────────────────────────────────────────▶  │ │
│  │            10:30:00.145 (Δ+45ms)                                   │ │
│  │                 │                                                  │ │
│  │            ┌────┴────┐                                             │ │
│  │            ▼         ▼                                             │ │
│  │ Slave_001  ────○─────────────────────────────────────────────▶   │ │
│  │            10:30:00.180 (Δ+35ms)                                   │ │
│  │                                                                    │ │
│  │ Slave_002  ────○─────────────────────────────────────────────▶   │ │
│  │            10:30:00.195 (Δ+50ms)                                   │ │
│  │                                                                    │ │
│  │ Slave_003  ────✕─────────────────────────────────────────────▶   │ │
│  │            10:30:00.175 (FILTERED)                                 │ │
│  │                                                                    │ │
│  └──────────────────────────────────────────────────────────────────┘ │
│                                                                        │
├────────────────────────────────────────────────────────────────────────┤
│  Trade Details Table                                                   │
│  ┌──────────────────────────────────────────────────────────────────┐ │
│  │ Time       │ Source    │ Event            │ Symbol│ Lots │ Status│ │
│  ├────────────┼───────────┼──────────────────┼───────┼──────┼───────┤ │
│  │ 10:30:00.1 │ Master_001│ Processing trade │EURUSD │ 1.00 │ ●     │ │
│  │ 10:30:00.14│ relay-srv │ Copying trade    │EURUSD │ 0.50 │ ●     │ │
│  │ 10:30:00.18│ Slave_001 │ Trade received   │EURUSD │ 0.50 │ ●     │ │
│  │ 10:30:00.19│ Slave_002 │ Trade received   │EURUSD │ 0.25 │ ●     │ │
│  │ 10:30:00.17│ relay-srv │ Trade filtered   │EURUSD │ -    │ ○     │ │
│  └──────────────────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────────────────┘

Legend: ● Success  ○ Filtered  ✕ Error
```

---

### 3. Performance Dashboard (性能評価)

システムのパフォーマンスメトリクスを詳細表示。

```
┌────────────────────────────────────────────────────────────────────────┐
│                      Performance Metrics                               │
├────────────────────────────────────────────────────────────────────────┤
│  Time Range: [Last 6h ▼]  Resolution: [1m ▼]                          │
├────────────────────────────────────────────────────────────────────────┤
│                                                                        │
│  End-to-End Latency (Master → Slave)                                   │
│  ┌──────────────────────────────────────────────────────────────────┐ │
│  │  P99: 250ms  |  P95: 150ms  |  P50: 45ms  |  Avg: 52ms           │ │
│  │                                                                    │ │
│  │     P99 ═══════════════════════════════════════════════           │ │
│  │     P95 ═══════════════════════════                               │ │
│  │     P50 ═══════════════                                           │ │
│  │     Avg ═══════════════════                                       │ │
│  │                                                                    │ │
│  │    ┌─────────────────────────────────────────────────────────┐   │ │
│  │    │ ▄▄  ▄▄▄   ▄                  ▄▄▄                         │   │ │
│  │    │███▄████▄▄██▄▄           ▄▄▄████████▄                     │   │ │
│  │    │──────────────────────────────────────────────────────── │   │ │
│  │    │ 06:00   08:00   10:00   12:00   14:00   16:00            │   │ │
│  │    └─────────────────────────────────────────────────────────┘   │ │
│  └──────────────────────────────────────────────────────────────────┘ │
│                                                                        │
├───────────────────────────────────┬────────────────────────────────────┤
│     Throughput (Trades/min)       │      Error Rate Over Time          │
│  ┌─────────────────────────────┐  │  ┌──────────────────────────────┐  │
│  │        ▄▄                   │  │  │                    ▄▄        │  │
│  │      ▄████▄     ▄▄▄         │  │  │                  ▄████       │  │
│  │    ▄████████▄▄█████▄        │  │  │  ▄▄   ▄▄      ▄██████       │  │
│  │  ▄██████████████████▄▄      │  │  │ ███▄▄███▄▄▄▄████████▄       │  │
│  │──────────────────────────── │  │  │──────────────────────────── │  │
│  │  06:00  08:00  10:00  12:00 │  │  │  06:00  08:00  10:00  12:00 │  │
│  │  Avg: 45/min  Peak: 120/min │  │  │  Avg: 2.1%  Peak: 8.5%      │  │
│  └─────────────────────────────┘  │  └──────────────────────────────┘  │
│                                   │                                    │
├───────────────────────────────────┴────────────────────────────────────┤
│  Latency by Symbol                                                     │
│  ┌──────────────────────────────────────────────────────────────────┐ │
│  │ Symbol   │  Count  │  Avg(ms) │  P50(ms) │  P95(ms) │  P99(ms)  │ │
│  ├──────────┼─────────┼──────────┼──────────┼──────────┼───────────┤ │
│  │ EURUSD   │  1,234  │   42     │   38     │   125    │   210     │ │
│  │ GBPUSD   │    856  │   45     │   40     │   135    │   225     │ │
│  │ USDJPY   │    523  │   48     │   43     │   140    │   235     │ │
│  │ XAUUSD   │    412  │   55     │   50     │   160    │   280     │ │
│  └──────────────────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────────────────┘
```

---

### 4. Error Analysis Dashboard (エラー分析)

エラーの詳細分析と根本原因特定を支援。

```
┌────────────────────────────────────────────────────────────────────────┐
│                        Error Analysis                                  │
├────────────────────────────────────────────────────────────────────────┤
│  Time Range: [Last 24h ▼]  Level: [ERROR ▼]  Source: [All ▼]          │
├────────────────────────────────────────────────────────────────────────┤
│                                                                        │
│  Error Distribution                                                    │
│  ┌───────────────────────────────────────────────────────────────────┐│
│  │                                                                   ││
│  │    ┌────────────────────────────┐                                 ││
│  │    │   Transform Error (35%)    │████████████████████             ││
│  │    └────────────────────────────┘                                 ││
│  │    ┌───────────────────────┐                                      ││
│  │    │  Send Failed (28%)    │████████████████                      ││
│  │    └───────────────────────┘                                      ││
│  │    ┌──────────────────┐                                           ││
│  │    │ Member N/F (22%) │█████████████                              ││
│  │    └──────────────────┘                                           ││
│  │    ┌────────────┐                                                 ││
│  │    │ Group (10%)│███████                                          ││
│  │    └────────────┘                                                 ││
│  │    ┌──────┐                                                       ││
│  │    │Other │████                                                   ││
│  │    └──────┘                                                       ││
│  └───────────────────────────────────────────────────────────────────┘│
│                                                                        │
├───────────────────────────────────┬────────────────────────────────────┤
│    Errors by Master Account       │      Error Timeline                │
│  ┌─────────────────────────────┐  │  ┌──────────────────────────────┐  │
│  │ Master_003  ██████████ 42   │  │  │                  ▲            │  │
│  │ Master_001  ████████   28   │  │  │        ▲         █ ▲          │  │
│  │ Master_005  ██████     18   │  │  │    ▲   █    ▲    █ █  ▲       │  │
│  │ Master_002  ████       12   │  │  │ ▲  █   █ ▲  █    █ █  █       │  │
│  │ Master_004  ██          5   │  │  │ █  █   █ █  █    █ █  █       │  │
│  └─────────────────────────────┘  │  │────────────────────────────── │  │
│                                   │  │ 00:00  06:00  12:00  18:00    │  │
│                                   │  └──────────────────────────────┘  │
├───────────────────────────────────┴────────────────────────────────────┤
│  Recent Errors (Log Table)                                             │
│  ┌──────────────────────────────────────────────────────────────────┐ │
│  │ Time       │ Level │ Source     │ Message                        │ │
│  ├────────────┼───────┼────────────┼────────────────────────────────┤ │
│  │ 11:30:15   │ ERROR │ relay-srv  │ Failed to transform signal:    │ │
│  │            │       │            │ Symbol mapping not found for   │ │
│  │            │       │            │ XAUUSD.raw                     │ │
│  ├────────────┼───────┼────────────┼────────────────────────────────┤ │
│  │ 11:28:42   │ ERROR │ relay-srv  │ Failed to send signal to trade │ │
│  │            │       │            │ group: Connection refused      │ │
│  ├────────────┼───────┼────────────┼────────────────────────────────┤ │
│  │ 11:25:10   │ ERROR │ relay-srv  │ Failed to get members for      │ │
│  │            │       │            │ master Master_003: DB timeout  │ │
│  └──────────────────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────────────────┘
```

---

### 5. Account Health Dashboard (アカウント健全性)

マスター/スレーブアカウントの状態監視。

```
┌────────────────────────────────────────────────────────────────────────┐
│                        Account Health                                  │
├────────────────────────────────────────────────────────────────────────┤
│  Last Updated: 11:35:00  Auto-refresh: [10s ▼]                        │
├────────────────────────────────────────────────────────────────────────┤
│                                                                        │
│  Master Accounts                                                       │
│  ┌──────────────────────────────────────────────────────────────────┐ │
│  │ Account    │ Status │ Last Signal │ Trades/h │ Success │ Slaves │ │
│  ├────────────┼────────┼─────────────┼──────────┼─────────┼────────┤ │
│  │ Master_001 │  ● ON  │ 2m ago      │    45    │  98.2%  │   5    │ │
│  │ Master_002 │  ● ON  │ 5m ago      │    32    │  95.5%  │   3    │ │
│  │ Master_003 │  ○ OFF │ 2h ago      │     0    │   -     │   4    │ │
│  │ Master_004 │  ● ON  │ 30s ago     │    78    │  99.1%  │   8    │ │
│  │ Master_005 │  ⚠ ERR │ 15m ago     │    12    │  72.3%  │   2    │ │
│  └──────────────────────────────────────────────────────────────────┘ │
│                                                                        │
│  Slave Accounts by Master                                              │
│  ┌──────────────────────────────────────────────────────────────────┐ │
│  │ Master: [Master_001 ▼]                                            │ │
│  │                                                                    │ │
│  │ Slave      │ Status │ Last Copy   │ Multiplier │ Success │ Filter│ │
│  ├────────────┼────────┼─────────────┼────────────┼─────────┼───────┤ │
│  │ Slave_001  │  ● ON  │ 2m ago      │   0.5x     │  99.1%  │ None  │ │
│  │ Slave_002  │  ● ON  │ 2m ago      │   0.25x    │  97.8%  │ None  │ │
│  │ Slave_003  │  ○ OFF │ 1h ago      │   1.0x     │   -     │ Symbol│ │
│  │ Slave_004  │  ● ON  │ 2m ago      │   0.1x     │  98.5%  │ None  │ │
│  │ Slave_005  │  ⚠ ERR │ 30m ago     │   0.5x     │  65.2%  │ None  │ │
│  └──────────────────────────────────────────────────────────────────┘ │
│                                                                        │
├────────────────────────────────────────────────────────────────────────┤
│  Connection Status Map                                                 │
│  ┌──────────────────────────────────────────────────────────────────┐ │
│  │                                                                    │ │
│  │  Master_001 ──●── relay-server ──●── Slave_001                    │ │
│  │              │                   ├──●── Slave_002                  │ │
│  │              │                   ├──●── Slave_003                  │ │
│  │              │                   └──●── Slave_004                  │ │
│  │                                                                    │ │
│  │  Master_002 ──●── relay-server ──●── Slave_005                    │ │
│  │                                  └──✕── Slave_006 (disconnected)  │ │
│  │                                                                    │ │
│  └──────────────────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────────────────┘

Legend: ● Connected/Active  ○ Disabled  ⚠ Error  ✕ Disconnected
```

---

## Panel Specifications

### Stat Panels (統計パネル)

| Panel | Metric | Query Pattern | Threshold |
|-------|--------|---------------|-----------|
| Total Received | Count of signals received | `_msg:"Processing trade signal"` | - |
| Copy Success | Count of successful copies | `_msg:"Copying trade to"` | - |
| Copy Failed | Count of failed copies | `level:ERROR` | Red >10 |
| Success Rate | Success / Total | Calculated | Red <90%, Yellow <95% |
| Avg Delay | Average latency | Requires span instrumentation | Red >200ms |

### Time Series Panels (時系列パネル)

| Panel | Description | Query |
|-------|-------------|-------|
| Trade Volume | Trades over time | `_msg:"Processing trade signal" \| stats count() by _time` |
| Error Rate | Errors over time | `level:ERROR \| stats count() by _time` |
| Latency Distribution | P50/P95/P99 latency | Requires span/duration fields |

### Table Panels (テーブルパネル)

| Panel | Columns | Query |
|-------|---------|-------|
| Trade Details | Time, Source, Event, Symbol, Lots, Status | Filter by ticket/time |
| Recent Errors | Time, Level, Source, Message | `level:ERROR \| sort by _time desc` |
| Account Status | Account, Status, Last Signal, Success Rate | Aggregation query |

---

## VictoriaLogs Queries

### Basic Queries

```logsql
# All trade signals received
_msg:"Processing trade signal"

# Successful copies
_msg:"Copying trade to"

# All errors
level:ERROR

# Errors by type
level:ERROR AND _msg:"Failed to transform"
level:ERROR AND _msg:"Failed to send signal"
level:ERROR AND _msg:"Failed to get members"
level:ERROR AND _msg:"Failed to get TradeGroup"

# Filtered trades
_msg:"Trade filtered out"

# By source (Master/Slave/Relay)
source:"relay-server"
source:~"ea:master:.*"
source:~"ea:slave:.*"
```

### Aggregation Queries

```logsql
# Trade count by master account
_msg:"Processing trade signal" | stats count() by source_account

# Error count by type
level:ERROR | stats count() by _msg

# Trades per minute
_msg:"Processing trade signal" | stats count() by _time:1m

# Error rate over time
* | stats
  count() if (level:ERROR) as errors,
  count() as total
  by _time:5m
| math errors / total * 100 as error_rate
```

### Performance Queries

```logsql
# Symbol-specific queries
_msg:"Copying trade" AND symbol:"EURUSD"

# High-frequency master detection
_msg:"Processing trade signal" | stats count() as cnt by source_account | filter cnt > 100

# Slave copy distribution
_msg:"Copying trade to" | extract "Copying trade to <slave_account>:" | stats count() by slave_account
```

---

## Recommended Logging Improvements

現在のログ構造を改善し、より効果的なダッシュボードを実現するための提案：

### 1. Structured Fields の追加

```rust
// Before (current)
tracing::info!("Processing trade signal: {:?}", signal);

// After (recommended)
tracing::info!(
    ticket = signal.ticket,
    symbol = signal.symbol.as_deref().unwrap_or(""),
    action = ?signal.action,
    lots = signal.lots.unwrap_or(0.0),
    master_account = %signal.source_account,
    "trade_signal_received"
);
```

### 2. Span Instrumentation for Latency

```rust
// Add spans for measuring processing time
#[tracing::instrument(
    name = "process_trade_copy",
    fields(
        ticket = %signal.ticket,
        master = %signal.source_account,
        slave = %member.slave_account,
    )
)]
async fn process_trade_copy(&self, signal: &TradeSignal, member: &TradeGroupMember) {
    // ... processing logic
}
```

### 3. Event Types (推奨ログイベント)

| Event Name | Fields | Purpose |
|------------|--------|---------|
| `trade_signal_received` | ticket, symbol, action, lots, master_account | マスターからのシグナル受信 |
| `trade_copy_started` | ticket, master_account, slave_account | コピー処理開始 |
| `trade_copy_completed` | ticket, slave_account, transformed_symbol, transformed_lots, duration_ms | コピー完了 |
| `trade_copy_filtered` | ticket, slave_account, filter_reason | フィルタリング |
| `trade_copy_failed` | ticket, slave_account, error | コピー失敗 |

### 4. LogEntry Extension

```rust
#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    #[serde(rename = "_time")]
    pub time: String,
    #[serde(rename = "_msg")]
    pub msg: String,
    pub level: String,
    pub source: String,
    pub target: String,

    // New structured fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_type: Option<String>,        // "trade_signal_received", etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ticket: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,            // "Open", "Close", "Modify"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lots: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub master_account: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slave_account: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,          // Processing duration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_type: Option<String>,

    // Existing optional fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
}
```

---

## Grafana Data Source Configuration

### VictoriaLogs Data Source Setup

```yaml
# grafana/provisioning/datasources/victorialogs.yaml
apiVersion: 1

datasources:
  - name: VictoriaLogs
    type: victorialogs-datasource
    access: proxy
    url: http://victoria-logs:9428
    isDefault: true
    jsonData:
      maxLines: 1000
```

### Dashboard Provisioning

```yaml
# grafana/provisioning/dashboards/default.yaml
apiVersion: 1

providers:
  - name: 'Trade Copy Dashboards'
    orgId: 1
    folder: 'Trade Copy'
    type: file
    disableDeletion: false
    editable: true
    options:
      path: /var/lib/grafana/dashboards
```

---

## Alert Rules (アラート設定)

### Critical Alerts

| Alert | Condition | Severity |
|-------|-----------|----------|
| High Error Rate | Error rate > 10% for 5m | Critical |
| Copy Failures | > 50 failures in 5m | Critical |
| No Signals | No signals received for 15m | Warning |
| High Latency | P95 > 500ms for 5m | Warning |

### Alert Configuration Example

```yaml
# Grafana Alert Rule
groups:
  - name: TradeCopyAlerts
    rules:
      - alert: HighErrorRate
        expr: |
          (
            count(level:ERROR) / count(*)
          ) > 0.1
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "High error rate in trade copy system"
          description: "Error rate is {{ $value | printf \"%.2f\" }}%"
```

---

## Docker Compose Update

```yaml
# docker-compose.yml additions
services:
  grafana:
    image: grafana/grafana:latest
    container_name: sankey-grafana
    ports:
      - "3000:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
      - GF_INSTALL_PLUGINS=victoriametrics-victorialogs-datasource
    volumes:
      - grafana-data:/var/lib/grafana
      - ./grafana/provisioning:/etc/grafana/provisioning
      - ./grafana/dashboards:/var/lib/grafana/dashboards
    depends_on:
      - victoria-logs

volumes:
  grafana-data:
```

---

## Implementation Priority

### Phase 1: Basic Monitoring (Week 1)
1. Grafana + VictoriaLogs datasource setup
2. Overview dashboard with basic stats
3. Error log table

### Phase 2: Trade Tracking (Week 2)
1. Trade flow dashboard
2. Account health dashboard
3. Basic alerting

### Phase 3: Performance Metrics (Week 3)
1. Logging improvements (structured fields)
2. Latency tracking with spans
3. Performance dashboard

### Phase 4: Advanced Features (Week 4)
1. Custom visualizations
2. Advanced alerting rules
3. Dashboard refinement based on usage
