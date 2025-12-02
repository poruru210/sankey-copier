# P10 Phase4: Web UI 更新計画

## 背景
- Phase2/3 で Relay Server の Status Engine が `enabled_flag`（ユーザー意図）と `runtime_status`（実効ステータス）を分離し、Phase4 P9 で MT EA は `runtime_status`/`allow_new_orders` を唯一のソースとして取り込む形に変更されました。Web UI はこの新しい二層ステータスを正しく表示・操作できるように追従する必要があります。
- 現在の UI は `status` フィールドを直接参照しており、トグル操作と実行状態が混在したため、`runtime_status` を優先的に用いたバッジ表示や `enabled_flag` に基づくトグル処理に移行しないと P9 で整備したサーバ設計と整合しません。

## 目的
1. API 応答に含まれる `enabled_flag` / `runtime_status` / `master_runtime_status` を Web UI 側に確実に伝搬する。
2. Connections ダッシュボード（Flow/Account nodes）と設定パネルのステータスバッジを `runtime_status` をキーに色分けし、`enabled_flag` はトグル操作だけに利用する。
3. Web UI のテスト・ドキュメントを更新して、新ステータス仕様を明文化し REST/WS と整合する。

## 対象箇所
- `web-ui/hooks/useFlowData.ts` / `hooks/connections/*`: Flow ノードのデータ変換で `settings.status` を `runtime_status` へ切り替える。
- `web-ui/components/flow-nodes/*`（例えば `AccountNode.tsx` や badge 描画）: バッジ/アイコンの色・文言を `runtime_status`/`enabled_flag` に応じて表示。
- `web-ui/hooks/useMembers.ts` / `utils/tradeGroupAdapter.ts`: API から返る `TradeGroupMember` を `enabled_flag`/`runtime_status` を注入して `settings` を組み立てる。
- `web-ui/components/settings-panel` / `connections/AccountCard`: トグル操作（マスター/スレーブ）を `enabled_flag` を基準に API 呼び出しを行い、`runtime_status` は WebSocket 通知で更新する。
- `web-ui/__tests__` + Playwright モック: 新表示の例と、トグル → `enabled_flag` 更新 → `runtime_status` 更新の流れを確認するテスト。

## 実装ステップ
1. `web-ui/utils/tradeGroupAdapter.ts` で `member.runtime_status` を `settings.runtime_status` として採用し、`enabled_flag` を `setting.enabled_flag` にコピー。`status` フィールドは将来的に削除される方向で `runtime_status` のミラーとして維持するのみ.
2. `useMembers.ts` などの hook では `runtime_status` がない場合に `status` をフォールバックするが、基本は `runtime_status` を優先。`enabled_flag` を元に `isEnabled` を決定し、Flow ノードの `AccountInfo` に `isActive` = `runtime_status === 2` の論理を入れる。`onToggle` は `enabled_flag` のみに対して API 舞う `toggle` を呼ぶ。
3. `AccountNode` / `FlowBadges` などのコンポーネントを更新し、「CONNECTED」 (runtime_status=2)/「WAIT」 (runtime_status=1)/「DISABLED」 (runtime_status=0) のバッジと色を表示。`allow_new_orders` を直接扱う必要はなく、`runtime_status` が 2 のときのみ copy readiness を示す UI を追加する。
4. トグルボタンには `enabled_flag` が false なら「手動OFF/Manual Off」などの状態ラベルを付け、`runtime_status` が 1 のときは「Master接続待ち」等を表示。 `Content` や `Flow` の tooltip もこれら新語を使う。
5. `web-ui/__tests__/mocks/testData.ts` や Playwright モック API を `enabled_flag`/`runtime_status` を返すよう更新し、`__tests__/components` に `runtime_status` 依存の snapshot や動作チェックを追加。
6. docs（`web-ui.md` + `docs/relay-server.md` の該当セクション）に Web UI がどのフィールドを表示・操作するか追記。

## テスト/検証
1. Unit/Hook Tests: `useFlowData` / `useMembers` で `runtime_status`/`enabled_flag` の変化に応じてノードデータが変化するケースを追加。
2. Playwright E2E: トグル操作 → API 結果（mocked `enabled_flag`） → WebSocket `runtime_status` 更新の流れをシミュレートし、バッジの文言が更新されることを確認。
3. API Contracts: `trade_group_members` レスポンスと `trade_groups` の `master_runtime_status` を docs・mock に明記し、`runtime_status`/`enabled_flag` を標準フィールドとして定義。

## 補足
- `allow_new_orders` は EA にのみ渡されるため Web UI では直接使わず、`runtime_status === CONNECTED` のときに `Copy Ready` 表示をオンにするのみ。その結果 Web UI/Relay/EA の整合性が保持されます。
- P11（Docs & Tests更新）と並行して進める必要があるため、テスト/ドキュメントは P10 と P11 で共同して扱います。
