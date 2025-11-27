# MQH/MQL リファクタリング計画書

## 概要

MT4/MT5 EA間の重複コードを削減し、保守性を向上させるためのリファクタリング計画。

## 現状分析

### コード統計
| カテゴリ | ファイル数 | 総LOC |
|----------|-----------|-------|
| EA Files (MQ5/MQ4) | 4 | 3,510 |
| Include Files (MQH) | 8 | 3,774 |
| **合計** | **12** | **7,284** |

### 主要課題
1. **SlaveTrade重複**: MT5/MT4 Slave EAで約400 LOCが重複
2. **メッセージパース重複**: 6箇所で同一の19行パターン（計114 LOC）
3. **タイマー処理重複**: Heartbeat/Config要求で約220 LOC重複
4. **GridPanel肥大化**: 1,705 LOCの単一ファイル

---

## 実装フェーズ

### Phase 1: CRITICAL（SlaveTrade.mqh）

**目的**: 取引実行ロジックをMT4/MT5で統合

**新規ファイル**: `Include/SankeyCopier/SlaveTrade.mqh`

**抽出対象関数**:
| 関数名 | MT5 Slave | MT4 Slave | 説明 |
|--------|-----------|-----------|------|
| ExecuteOpenTrade | 827-897行 | 834-917行 | 成行注文実行 |
| ExecuteCloseTrade | 902-931行 | 922-954行 | ポジション決済 |
| ExecuteModifyTrade | 936-947行 | 959-987行 | SL/TP変更 |
| ExecutePendingOrder | 952-1000行 | 1006-1064行 | 指値/逆指値注文 |
| ExecuteCancelPendingOrder | 1005-1020行 | 1069-1084行 | 待機注文キャンセル |
| SyncWithLimitOrder | 697-757行 | 657-711行 | 指値で同期 |
| SyncWithMarketOrder | 764-822行 | 718-771行 | 成行で同期 |
| CheckPendingOrderFills | N/A | 777-829行 | 待機注文約定検知(MT4のみ) |

**アーキテクチャ**:
```mql4
// Platform-agnostic interface
void ExecuteOpenTrade(...) {
   #ifdef IS_MT5
      _ExecuteOpenTrade_MT5(...);  // CTrade使用
   #else
      _ExecuteOpenTrade_MT4(...);  // OrderSend使用
   #endif
}
```

**期待効果**: 各Slave EAから130+ LOC削減

---

### Phase 2: HIGH（MessageParsing.mqh, TimerHandling.mqh）

#### MessageParsing.mqh

**目的**: ZMQ PUB/SUBメッセージのトピック/ペイロード分離を統合

**新規ファイル**: `Include/SankeyCopier/MessageParsing.mqh`

**抽出対象**:
```mql4
bool ExtractZmqTopicAndPayload(uchar &buffer[], int buffer_size,
                               string &topic, uchar &payload[]);
```

**使用箇所（6箇所）**:
- MT5 Master OnTimer: 220-248行
- MT5 Slave OnTimer: 256-284行
- MT4 Master OnTimer: 188-221行
- MT4 Slave OnTimer: 252-280行
- MT5 Slave OnTick: 366-401行
- MT4 Slave OnTick: 358-403行

**期待効果**: 114 LOC削減

#### TimerHandling.mqh

**目的**: Heartbeat・設定要求・パネル更新ロジックを統合

**新規ファイル**: `Include/SankeyCopier/TimerHandling.mqh`

**抽出対象関数**:
| 関数名 | 削減LOC | 説明 |
|--------|---------|------|
| HandleHeartbeatTimer | 120 | Heartbeat送信＋取引状態検知 |
| HandleConfigRequestTimer | 40 | 初回設定リクエスト |
| UpdateSlavePanel Status | 60 | パネル状態更新（Slave専用） |

**期待効果**: 220 LOC削減

---

### Phase 3: MEDIUM（InitBase.mqh, CleanupBase.mqh）

#### InitBase.mqh

**目的**: EA初期化パターンを統合

**抽出対象**:
- InitializeMasterEA(): Master EA共通初期化
- InitializeSlaveEA(): Slave EA共通初期化

#### CleanupBase.mqh

**目的**: EA終了処理を統合

**抽出対象**:
- CleanupEA(): 共通クリーンアップ処理

---

## 期待効果まとめ

| 指標 | 現状 | 目標 | 削減率 |
|------|------|------|--------|
| EA合計LOC | ~2,400 | ~1,600 | 33% |
| 重複コード | ~600 LOC | ~100 LOC | 83% |
| #ifdefブロック | 91+ | ~20 | 78% |

---

## 進捗管理

- [x] Phase 1: SlaveTrade.mqh ✅ 作成完了 (590 LOC)
- [x] Phase 2: MessageParsing.mqh ✅ 作成完了 (130 LOC)
- [x] Phase 2: TimerHandling.mqh ✅ 作成完了 (210 LOC)
- [x] Phase 3: InitBase.mqh ✅ 作成完了 (170 LOC)
- [x] Phase 3: CleanupBase.mqh ✅ 作成完了 (90 LOC)
- [x] Slave EA更新 ✅ 完了
  - MT5 Slave: 1,050行 → 718行（-332行、32%削減）
  - MT4 Slave: 1,100行 → 679行（-421行、38%削減）

## 実績まとめ

| 指標 | 変更前 | 変更後 | 削減率 |
|------|--------|--------|--------|
| MT5 Slave LOC | ~1,050 | 718 | 32% |
| MT4 Slave LOC | ~1,100 | 679 | 38% |
| Slave EA合計 | ~2,150 | 1,397 | 35% |
| 重複コード | ~600 LOC | ~0 LOC | 100% |

**リファクタリング完了日**: 2025-11-27

## 作成されたファイル

| ファイル | パス | LOC | 説明 |
|----------|------|-----|------|
| SlaveTrade.mqh | Include/SankeyCopier/ | ~590 | 取引実行抽象化（MT4/MT5統合） |
| MessageParsing.mqh | Include/SankeyCopier/ | ~130 | ZMQメッセージパース |
| TimerHandling.mqh | Include/SankeyCopier/ | ~210 | タイマー処理統合 |
| InitBase.mqh | Include/SankeyCopier/ | ~170 | EA初期化ヘルパー |
| CleanupBase.mqh | Include/SankeyCopier/ | ~90 | EA終了処理ヘルパー |

**合計**: ~1,190 LOC の新規共通コード

---

## 注意事項

1. **後方互換性**: 既存のEA動作を変更しない
2. **段階的移行**: 一度に全てを変更せず、フェーズ毎にテスト
3. **プラットフォーム差異**: MT4/MT5の差異は内部実装で吸収
