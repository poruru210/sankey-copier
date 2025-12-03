# Runtime Status Metrics Runbook

RuntimeStatusUpdater が公開する `/api/runtime-status-metrics` を監視ダッシュボードへ組み込み、異常時にトリアージするための手順をまとめる。

## 1. エンドポイント概要

- URL: `GET /api/runtime-status-metrics`
- 返却 JSON:
  ```json
  {
    "master_evaluations_total": 420,
    "master_evaluations_failed": 0,
    "slave_evaluations_total": 1380,
    "slave_evaluations_failed": 2,
    "slave_bundles_built": 512,
    "last_cluster_size": 2
  }
  ```
- `last_cluster_size` は直近の Slave 評価で参照した Master 台数。マルチ Master の実運用規模チェックに利用できる。

## 2. Grafana 連携

1. `relay-server` の 3000 番ポートを監視ネットワークから疎通可能にする。
2. Grafana の *JSON API* データソースで以下を設定:
   - URL: `http://<relay-host>:3000/api/runtime-status-metrics`
   - Method: `GET`
   - Cache TTL: 10s (Heartbeat 間隔と同程度)
3. パネル例:
   - **Stat**: `slave_evaluations_failed` (threshold: >0 → 赤)
   - **Time series**: `slave_evaluations_total` を微分して 1 秒あたり評価回数を表示
   - **Table**: `last_cluster_size` をマルチ Master の期待値と比較

## 3. アラートポリシー

| シグナル | 条件 | 初動 |
|----------|------|------|
| Slave 評価失敗 | `increase(slave_evaluations_failed[5m]) > 0` | VictoriaLogs で同時刻の `runtime_status_updater` ログを検索。DB/ZeroMQ 切断を確認。 |
| Master 評価失敗連続 | `increase(master_evaluations_failed[5m]) >= 3` | Master Heartbeat が止まっていないか `connections` API で確認。 |
| クラスターサイズ急変 | `last_cluster_size` が 0 または期待値から ±2 以上 | Web UI の Trade Group 構成が変わっていないか確認。手動で Slave の紐付けが外れていないかレビューする。 |

## 4. トリアージ手順

1. **API 健全性**: `curl http://localhost:3000/api/runtime-status-metrics` で 200/JSON を確認。
2. **ログ確認**: `relay-server/logs/*.log` または VictoriaLogs の `target="runtime_status"` を時系列で追跡。
3. **ZeroMQ/DB**: `RuntimeStatusUpdater` は DB/ConnectionManager へアクセスするため、`db_pool` や `connection_manager` の警告が無いか `tracing` ログを確認。
4. **再評価トリガ**: 必要に応じて `RequestConfig` を Slave から手動送信し (`mt-bridge` の RequestConfig ボタン)、RuntimeStatusUpdater が即座に反映されるか観測する。

## 5. 既知の閾値

- `slave_evaluations_total` の 1秒あたり 20 回超は過剰。通常は `(#Slave * Heartbeat/sec)` が上限。
- `master_evaluations_failed` が 1 を超えたら Slack #alert-runtime-status に通知する。
- `last_cluster_size` が 0 のまま 30 秒続いた場合、対象 Slave が Master 未紐付け (`no_master_assigned`) 状態の可能性が高い。

## 6. 将来タスク

- Prometheus Exporter へ同メトリクスを push し、`rate()` を用いたアラートを簡略化する。
- `warning_codes` とメトリクスの相関を取るため、`/api/runtime-status-metrics` のレスポンスへ `recent_warning_counts` を追加する検討を行う。
