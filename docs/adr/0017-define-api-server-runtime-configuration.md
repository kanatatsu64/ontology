# 0017: API サーバーの実行時設定と依存関係更新方針

- 日付: 2026-09-09
- 意思決定者: プロジェクト
- 関連 ADR: 0011, 0012, 0016

## コンテキスト

route を追加する前に、Cloud Run で安全に起動・停止できる API サーバーの枠組みが必要です。ADR 0011 で必須とした resource 上限の値、外部設定の境界、依存関係を導入する際の鮮度と脆弱性確認を具体化します。

## 用語

| 用語 | 定義 |
| --- | --- |
| 安定化期間 | 新しい release の問題が判明する機会を設けるため、採用まで待つ期間。 |
| request timeout | request の処理開始から response 完了まで許容する時間。 |
| body size | 一つの HTTP request body で受け付ける最大 byte 数。 |
| concurrency | 一つの process が同時に処理する request 数。 |

## 決定

- API サーバーは `BIND_ADDRESS` と `PORT` で待受先を設定し、既定値を Cloud Run と互換性のある `0.0.0.0:8080` とします。
- request timeout は `REQUEST_TIMEOUT_SECONDS=30`（範囲 1–3,600）、concurrency は `MAX_CONCURRENT_REQUESTS=256`（範囲 1–10,000）、body size は `MAX_REQUEST_BODY_BYTES=1048576`（範囲 1–16,777,216）、graceful shutdown の期限は `SHUTDOWN_TIMEOUT_SECONDS=30`（範囲 1–300）を既定値とします。単位は timeout が秒、body size が byte です。すべて環境変数で上書きでき、範囲外または構文上不正な値では起動を失敗させます。同時実行枠が埋まっている場合は request を process 内で待機させず、`503 Service Unavailable` を返します。
- process は `SIGTERM` または割り込みを受けると新規受付を止めます。graceful shutdown の期限後は残った connection を終了します。
- log は標準出力へ JSON で出力し、level filter は `RUST_LOG`、未設定または不正な場合は `info` とします。設定 error に設定値を含めません。
- Rust dependency は default feature を無効にして必要な feature だけを有効化し、workspace で直接 dependency の完全 version を固定します。採用候補は release から 14 日以上経過した非 yanked version のうち最新のものとし、追加・更新時に RustSec advisory と license を確認します。dependency を解決できる環境で `Cargo.lock` を生成して commit し、CI で `cargo audit` を実行します。
- 2026-09-09 時点の採用 version は `axum 0.8.6`、`tokio 1.47.1`、`tower 0.5.2`、`tower-http 0.6.6`、`tracing 0.1.41`、`tracing-subscriber 0.3.20` とします。採用の基準日は 2026-08-26 とし、それより後の release は使用しません。

## 検討

### D-1: 設定の入力と既定値

#### 判断基準

- 同じ image を local と Cloud Run で使用できること
- 設定漏れでも resource が無制限にならないこと
- 過大な設定値で framework の許容範囲超過や極端な resource 予約を起こさないこと
- 不正な明示値を見逃さないこと

#### 選択肢

- **環境変数と保守的な既定値:** 起動が容易で上限も常に有効だが、環境ごとの tuning が必要になる。
- **全項目を必須の環境変数にする:** 設定は明示的だが、値が同じでも全環境に重複して定義する。
- **設定 file:** 複雑な設定を表現できるが、image 外から file を配布する仕組みが増える。

#### 採用

**環境変数と保守的な既定値**を採用します。

- 判断基準「image の共通化」: Cloud Run が供給する `PORT` を直接利用でき、local でも追加 file なしで起動できます。
- 判断基準「resource 上限」: 未設定時にも有限の timeout、concurrency、body size、終了期限を適用し、過大値は middleware を構築する前に拒否します。
- 判断基準「誤設定の検出」: 明示された不正値は既定値へ戻さず起動 error にします。

### D-2: dependency の version 選択

#### 判断基準

- security 修正を長期間取り込まない状態を避けること
- release 直後に発見される regression の影響を避けること
- build と review を再現可能にすること

#### 選択肢

- **14 日の安定化期間と完全 version 固定:** 定期更新が必要だが、鮮度と初期 regression 回避を両立できる。
- **常に最新 version:** security 修正は早いが、公開直後の version を production に入れる可能性がある。
- **互換 version 範囲のみ指定:** patch を容易に得られるが、lock file 更新時の入力が明示されにくい。

#### 採用

**14 日の安定化期間と完全 version 固定**を採用します。

- 判断基準「鮮度」: 基準日以前で最新の候補を定期的に確認し、古い version の放置を避けます。
- 判断基準「安定性」: 公開後 14 日は採用せず、緊急の security 修正だけ review で例外を記録します。
- 判断基準「再現性」: manifest で直接 dependency を固定し、dependency 解決時に生成する lock file で推移 dependency も固定します。

## 影響

### 良い影響

- route の実装前から、過負荷、巨大 request、長時間 request、終了処理に有限の境界があります。
- local と Cloud Run で同じ設定 interface を利用できます。
- dependency 更新時の鮮度、安定性、security の確認方法が明確になります。

### 悪い影響・トレードオフ

- 実際の workload に応じて既定値を計測し、環境変数を調整する必要があります。
- dependency release と採用の間に最低 14 日の遅延が生じます。
- security 修正の緊急度が安定化期間より高い場合は、例外の review と記録が必要です。
