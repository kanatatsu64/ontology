# 0008: 生成 API の記述形式と契約の境界

- 日付: 2026-09-06
- 意思決定者: プロジェクト
- 関連 ADR: 0002, 0003

## コンテキスト

API 仕様はレビュー可能で、ブラウザーから利用でき、Rust のサーバー境界とクライアントを生成できる必要があります。nullable、必須入力、十進数、日付も欠落なく表現します。

## 決定

- HTTP/JSON API を OpenAPI 3.1 の決定的な JSON 文書として生成し、直接編集しません。
- `text` は nullable string、`number` は nullable な decimal string、`date` は nullable な `format: date` として生成します。必須 Property は create schema の `required`、任意 Property は `default` で表します。
- `/v1`、`application/json`、UUID、安定した `operationId`、共通 problem detail、HTTP status、Bearer security scheme を仕様へ生成します。
- validator と nullable/default/decimal/date/required の fixture で、OpenAPI と生成 code を検査します。

## 検討

### D-1: API の記述形式

#### 判断基準

- ブラウザーを含む周辺アプリから利用できること
- サーバーとクライアントを生成できること
- HTTP operation と JSON の型を一つの成果物で表せること

#### 選択肢

- **OpenAPI 3.1:** HTTP と JSON Schema を扱えるが、tool ごとに対応差がある。
- **OpenAPI 3.0:** tool は多いが、null の表現が JSON Schema と異なる。
- **GraphQL Schema:** graph 取得に適するが、HTTP status や operation を別途規約化する必要がある。
- **Protocol Buffers/gRPC:** 厳密だが、browser には変換層が必要になる。

#### 採用

**OpenAPI 3.1**を選びます。HTTP 契約と、全 Property 型が含む `null` を JSON Schema に沿って一つの仕様に表現できます。

### D-2: OpenAPI 成果物の形式

#### 判断基準

- 同じ入力から byte-for-byte で再生成できること
- parser ごとの型解釈差を避けること
- 人が PR の差分を確認できること

#### 選択肢

- **JSON:** 表現は冗長だが、scalar の型が明示的になる。
- **YAML:** 読みやすいが、ontology YAML と原典を混同しやすく暗黙変換もある。

#### 採用

**JSON**を選びます。key の順序と format を generator で固定します。OpenAPI は生成物であり、直接編集しません。

### D-3: Property の wire format

#### 判断基準

- nullability と required を別々に表現できること
- 金額の十進精度を client 言語に依存せず維持できること
- 標準的な日付表現を使えること

#### 選択肢

- **十進数を JSON number にする:** 自然だが、client が binary float に変換し得る。
- **十進数を string にする:** 変換は必要だが、表記した精度を維持できる。

#### 採用

**十進数を string にする**案を選び、次の schema を生成します。

- `text`: `{"type":["string","null"]}`
- `number`: `{"type":["string","null"],"format":"decimal"}`
- `date`: `{"type":["string","null"],"format":"date"}`
- 必須 Property: create request の `required` へ追加
- 任意 Property: `default` を設定し、`required` へ追加しない

### D-4: API 全体の規約

#### 判断基準

- operation と生成コードの対応が安定すること
- 互換性を major version 単位で管理できること
- エラーと認証要求を機械処理できること

#### 選択肢

- **OpenAPI に共通規約を生成する:** 仕様が増えるが、client にも伝播する。
- **実装だけの規約にする:** 仕様は短いが、生成 client が認識できない。

#### 採用

**OpenAPI に共通規約を生成する**案を選びます。`/v1`、`application/json`、UUID、安定した `operationId`、共通 problem detail schema、HTTP status、Bearer security scheme を明記します。認証方式自体は別 ADR で扱います。

### D-5: tool の対応差

#### 判断基準

- 仕様が妥当であることを自動検査できること
- generator 更新で型や検証が黙って変わらないこと

#### 選択肢

- **fixture と validator で制限する:** 利用機能は狭まるが、対応差を早期検出できる。
- **全 JSON Schema 機能を許可する:** 表現力は高いが、生成不能な仕様になり得る。

#### 採用

**fixture と validator で制限する**案を選びます。nullable、default、decimal、date、required の fixture を用意し、標準 validator と生成コードの compile で検査します。

## 影響

OpenAPI 差分で API 互換性をレビューできます。複雑な graph query と bulk API は別の検討事項とします。
