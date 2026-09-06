# 0011: Rust API サーバーの非同期ランタイムと HTTP フレームワーク

- 日付: 2026-09-06
- 意思決定者: プロジェクト
- 関連 ADR: 0004, 0010

## コンテキスト

生成された Rust の HTTP 境界を実行する runtime と framework を選びます。

## 用語

| 用語 | 定義 |
| --- | --- |
| 非同期 runtime | 待ち時間のある複数の処理を効率よく進行させる実行基盤。 |
| HTTP framework | request の受付、routing、response の生成を支援する仕組み。 |
| handler | 一つの API 操作を受け持つ処理。 |
| blocking 処理 | 完了まで実行 thread を占有する処理。 |
| graceful shutdown | 受付を止め、処理中の request に猶予を与えて終了すること。 |

## 決定

Tokio と axum を使用します。runtime は entry point で一度だけ生成し、handler は可変 global state を持ちません。timeout、同時実行数、body size、graceful shutdown の期限を必須設定とし、blocking 処理を async executor 上で直接実行しません。

## 検討

### D-1: runtime と framework

#### 判断基準

- 選定した server generator の対象であること
- 非同期 HTTP と DB I/O を扱えること
- request state を明示的に注入できること

#### 選択肢

- **Tokio + axum:** generator と整合し、tower 資産を使えるが、両 API に依存する。
- **Tokio + Actix Web:** 成熟しているが、選定 generator の handler model と異なる。
- **同期 runtime:** 単純だが、待機の多い service に不向き。

#### 採用

**Tokio + axum**を選びます。runtime は entry point で一度作り、handler は可変 global state を持ちません。

### D-2: process の終了と上限

#### 判断基準

- deploy 時に処理中 request を無制限に切断しないこと
- resource 枯渇を設定で防げること

#### 選択肢

- **graceful shutdown と必須 limit:** 設定は増えるが、終了と負荷時の挙動を制御できる。
- **既定動作のみ:** 簡単だが、環境ごとに挙動が不明確になる。

#### 採用

**graceful shutdown と必須 limit**を選びます。timeout、concurrency、body size を設定し、受付停止後に期限付きで処理を完了します。

## 影響

application は Tokio の async model と axum/tower の抽象に依存します。blocking 処理を executor 上で直接実行できません。
