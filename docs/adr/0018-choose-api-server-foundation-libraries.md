# 0018: API サーバー基盤ライブラリの選定

- 日付: 2026-09-09
- 意思決定者: プロジェクト
- 関連 ADR: 0010, 0011, 0017

## コンテキスト

ADR 0011 は非同期 runtime に Tokio、HTTP framework に axum を使用すると決定しました。最初の API server 実装にあたり、HTTP 制限と運用 log を担う主要 library、および各 library の責務を確定する必要があります。

## 用語

| 用語 | 定義 |
| --- | --- |
| middleware | handler の前後で request または response に共通処理を適用する部品。 |
| span | 一連の処理に属する構造化 event を関連付ける単位。 |
| feature flag | crate に含める機能を compile 時に選択する Cargo の仕組み。 |

## 決定

- 非同期 runtime は `tokio`、HTTP server と routing は `axum` を使用します。
- middleware の合成、同時実行数制限、超過 request の load shedding は `tower`、HTTP request body size と response 完了までの timeout は `tower-http` を使用します。手書き middleware は既存 library で要件を満たせない場合だけ追加します。
- 構造化 event と span は `tracing`、標準出力への JSON 出力と環境変数による level filter は `tracing-subscriber` を使用します。application code は `tracing` の macro に依存し、出力形式の設定を entry point に限定します。
- すべての library は workspace で version を統一し、default feature を無効にして使用する feature flag だけを有効化します。version の選択と更新は ADR 0017 の安定化期間および security 確認規則に従います。

## 検討

### D-1: HTTP server と非同期 runtime

#### 判断基準

- ADR 0010 の生成 server interface と整合すること
- 非同期 I/O、graceful shutdown、明示的な state 注入を扱えること
- project 内で runtime を一つに統一できること

#### 選択肢

- **Tokio と axum:** 既存 ADR および生成対象と整合し、Tower middleware を利用できる。
- **Tokio と Actix Web:** 非同期 HTTP を扱えるが、生成予定の axum server interface と一致しない。
- **async-std と別の HTTP framework:** runtime の選択が既存 ADR と分裂する。

#### 採用

**Tokio と axum**を採用します。

- 判断基準「生成境界」: ADR 0010 で選んだ `rust-axum` の生成物を直接接続できます。
- 判断基準「runtime 統一」: signal、socket、timer を Tokio 上に統一できます。
- 判断基準「拡張性」: axum の `Router` を Tower service として middleware に接続できます。

### D-2: HTTP 制限 middleware

#### 判断基準

- concurrency、body size、response body を含む timeout を宣言的に合成できること
- axum と同じ service abstraction を使用すること
- security 境界を個々の handler に重複させないこと

#### 選択肢

- **tower と tower-http:** crate は増えるが、axum と同じ Tower abstraction で汎用制限と HTTP 固有制限を分担できる。
- **axum の extractor と手書き middleware:** dependency は抑えられるが、body を読まない handler や streaming response を一律に制限しにくい。
- **reverse proxy の制限のみ:** application は単純になるが、local と Cloud Run の application process で同じ境界を保証できない。

#### 採用

**tower と tower-http**を採用します。

- 判断基準「適用範囲」: handler の実装に依存せず router 全体へ制限を適用できます。
- 判断基準「整合性」: axum が採用する Tower service/layer と同じ合成方法を使用できます。
- 判断基準「責務」: concurrency と待機列を作らない load shedding は汎用 `tower`、HTTP body と timeout は `tower-http` に分けます。

### D-3: 構造化 log

#### 判断基準

- Cloud Run が収集できる標準出力へ JSON を出力できること
- async request の event を span で関連付けられること
- application code と出力先・形式を分離できること

#### 選択肢

- **tracing と tracing-subscriber:** 設定は必要だが、構造化 field、span、出力 layer を分離できる。
- **log と env_logger:** 単純だが、async 処理を span で関連付ける機能が限定される。
- **JSON の直接生成:** 出力は制御できるが、level filter、span、field の規約を独自実装することになる。

#### 採用

**tracing と tracing-subscriber**を採用します。

- 判断基準「構造化」: message に埋め込まず、検索可能な field として値を記録できます。
- 判断基準「非同期処理」: request 処理を span に関連付ける拡張経路があります。
- 判断基準「分離」: event の生成と JSON subscriber の構成を別の責務にできます。

## 影響

### 良い影響

- server、middleware、log の責務と依存先が明確になります。
- resource 制限と構造化 log を route ごとに再実装せず適用できます。
- OpenAPI から生成する axum server interface と同じ基盤を使用できます。

### 悪い影響・トレードオフ

- Tokio、axum、Tower ecosystem、tracing ecosystem の API と release cycle に依存します。
- feature flag と互換 version の組み合わせを dependency 更新時に検証する必要があります。
- request span や correlation ID は route と生成 server interface の導入時に別途設計する必要があります。
