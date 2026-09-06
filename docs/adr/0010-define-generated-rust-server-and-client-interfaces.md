# 0010: OpenAPI から生成する Rust のサーバー境界とクライアント

- 日付: 2026-09-06
- 意思決定者: プロジェクト
- 関連 ADR: 0008, 0009

## コンテキスト

OpenAPI と実装の逸脱を compile 時に検出できる Rust 境界が必要です。

## 用語

| 用語 | 定義 |
| --- | --- |
| サーバー境界 | HTTP request を受け取り、業務実装を呼び出して response を返す接点。 |
| client | API を呼び出す側が利用する型と操作の集合。 |
| handler trait | 各 API 操作を業務実装へ要求する Rust の interface。 |
| routing | request の path と method を対応する handler へ振り分けること。 |
| compile 時検出 | 実行前の build で型や interface の不一致をエラーにすること。 |

## 決定

- version と template を固定した OpenAPI Generator の `rust-axum` から model、routing、handler trait を、Rust generator から client を生成して commit します。
- 生成層は HTTP/JSON、入力検証、型変換だけを担い、手書きの業務実装が生成 trait を実装します。
- 生成 file は直接編集せず、generator 更新は生成差分として review します。

## 検討

### D-1: 生成方式

#### 判断基準

- OpenAPI を唯一の API 契約に保てること
- server と client の両方を生成できること
- axum の handler 境界を生成できること

#### 選択肢

- **OpenAPI Generator:** 両側を生成できるが、版と生成差分の管理が必要になる。
- **code first:** Rust 統合は強いが、OpenAPI が入力でなくなる。
- **独自 generator:** 制御しやすいが、保守範囲が広い。

#### 採用

**OpenAPI Generator**を選び、`rust-axum` の model、routing、handler trait と Rust client を生成します。

### D-2: 生成 code と手書き code の境界

#### 判断基準

- 再生成で手書き実装を失わないこと
- HTTP と業務処理を分離できること

#### 選択肢

- **trait 境界:** adapter は増えるが、生成側から業務実装を分離できる。
- **生成 file を直接編集:** 短期は容易だが、再生成できなくなる。

#### 採用

**trait 境界**を選びます。生成層は HTTP/JSON、入力検証、型変換だけを担い、手書き実装が trait を実装します。

## 影響

generator 更新は明示的な生成差分としてレビューします。必要な OpenAPI 3.1 fixture を生成できない場合は generator 選定を新しい ADR で変更します。
