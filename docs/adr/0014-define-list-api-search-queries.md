# 0014: list API の検索クエリ

- 日付: 2026-09-06
- 意思決定者: プロジェクト
- 関連 ADR: 0003, 0008, 0009

## コンテキスト

Entity と Link の list API で、ID、Link の端点、Entity の Property を条件に対象を絞り込む必要があります。条件は AND と OR を組み合わせられ、Property の型に対して妥当な比較だけを受け付ける必要があります。

## 用語

| 用語 | 定義 |
| --- | --- |
| filter / 検索式 | list の結果に含める Entity または Link を定める条件。 |
| 述語 | 検索対象、演算子、比較値から成る検索式の最小単位。 |
| operand / 比較値 | 述語で検索対象と比較する scalar、範囲値、またはその集合。 |
| scalar / 単一値 | 範囲を持たない一つの `text`、`number`、`datetime` 値。 |
| 範囲値 | 下端、上端、および各端点を含むかどうかを持つ連続した値。 |
| calendar 範囲 | 特定の time zone における月または日を表す `datetime` の範囲値。 |

## 決定

- 既存の Entity/Link list の `GET` operation に、任意の `filter` query parameter を追加します。`filter` の値は percent-encode した UTF-8 JSON とし、省略時は全件を対象にします。
- filter は `and`、`or`、または一つの述語を持つ式です。`and` と `or` は一つ以上の式を配列で受け取り、相互にネストできます。一つの object に複数の演算子を指定することはできません。
- Entity は `id` の `in` 述語と、Property 述語を使用できます。Property 述語は endpoint の Entity 型に定義された Property だけを対象とし、使用可能な演算子と値の形式を Property 型から生成します。
- Link は `id`、`from`、`to` の `in` 述語を使用できます。それぞれ一つ以上の UUID を受け取り、同じ述語内の値は OR として評価します。`from` と `to` は Link 型に定義された向きを維持します。
- `text` は `eq`、`ne`、`in`、`contains`、`starts_with`、`ends_with`、`number` と `datetime` は `eq`、`ne`、`in`、`lt`、`lte`、`gt`、`gte` を使用できます。すべての Property 型で `is_null` と `is_not_null` を使用できます。
- `number` と `datetime` の比較 operand は scalar または範囲値とします。`in` は集合所属のままとし、配列の各要素に scalar または範囲値を指定できます。範囲値は端点ごとに `open` または `closed` を明示します。
- `datetime` の範囲値には明示的な両端のほか、IANA time zone と `month` または `day` を指定する calendar 範囲を用意します。calendar 範囲は現地時刻の期間開始を含み、次の期間開始を含まない半開区間とします。
- filter は cursor の検索条件に含め、同じ cursor と異なる filter の組み合わせを `422` にします。不正な構造、未知の Property、型に合わない値、許可されない演算子、制限超過も `422` とします。
- ネストは root を含めて 8 階層、述語は合計 100 個、各 `in` の要素は 100 個までとします。空の `and`、`or`、`in` は許可しません。

## 検討

### D-1: filter の転送形式

#### 判断基準

- 既存の list operation と pagination を維持できること
- AND/OR の木構造を曖昧なく表現できること
- OpenAPI で入力 schema を生成できること

#### 選択肢

- **query parameter の JSON:** URL 長の制約はあるが、既存の GET list と一つの operation にできる。
- **個別の query parameter:** 単純な条件は読みやすいが、ネストの表現に独自規則が必要になる。
- **request body を持つ search operation:** 大きな条件を送れるが、list と別の operation と pagination 契約が必要になる。

#### 採用

**query parameter の JSON**を選びます。`filter` parameter は JSON schema を OpenAPI の `content: application/json` で記述し、JSON 全体を query parameter の値として percent-encode します。

### D-2: 論理式

#### 判断基準

- AND と OR を任意に組み合わせられること
- 評価順序に依存しないこと
- 不完全または曖昧な条件を拒否できること

#### 選択肢

- **再帰的な式 object:** 冗長だが、構造と演算子が明示的になる。
- **文字列の query language:** 短いが、parser、escaping、grammar の保守が必要になる。
- **平坦な条件配列:** 単純だが、AND と OR のネストを表現できない。

#### 採用

**再帰的な式 object**を選びます。論理式は次のいずれか一つだけを持ちます。

```json
{"and": [<expression>, <expression>]}
{"or": [<expression>, <expression>]}
```

配列要素は左から評価する必要がなく、結果は順序に依存しません。空配列は許可しません。

### D-3: Link の述語

#### 判断基準

- 複数の Link ID を一度に指定できること
- from と to の候補をそれぞれ複数指定できること
- Link の方向を検索時にも維持すること

#### 選択肢

- **field ごとの `in` 述語:** 複合条件には論理式が必要だが、複数候補の意味が明確になる。
- **単一値の `eq` だけ:** schema は小さいが、複数候補を OR 式へ展開する必要がある。
- **from/to の組の配列:** 端点の組には適するが、片側だけの検索と ID 検索が別形式になる。

#### 採用

**field ごとの `in` 述語**を選びます。

```json
{"id": {"in": ["<link-uuid-1>", "<link-uuid-2>"]}}
{"from": {"in": ["<entity-uuid-1>", "<entity-uuid-2>"]}}
{"to": {"in": ["<entity-uuid-3>", "<entity-uuid-4>"]}}
```

同じ配列内は OR です。複数 field を組み合わせる場合は `and` または `or` を明示します。`from` と `to` を逆転して照合しません。

### D-4: Entity の ID と Property の述語

#### 判断基準

- ID と Property の両方で検索できること
- Property 型に無効な比較を schema と runtime で拒否できること
- null と値の比較を区別できること

#### 選択肢

- **型別の述語 schema:** 生成 schema は増えるが、利用可能な演算子と値を静的に示せる。
- **全型共通の演算子:** schema は単純だが、無効な比較を runtime でしか拒否できない。
- **文字列へ変換して比較:** 実装は共通化できるが、数値と日時の順序を正しく扱えない。

#### 採用

**型別の述語 schema**を選びます。ID は Link と同じ `in` 形式、Property は `property`、演算子、値を持つ形式にします。

```json
{"id": {"in": ["<entity-uuid-1>", "<entity-uuid-2>"]}}
{"property": "memo", "contains": "交通費"}
{"property": "amount", "gte": "1000.00"}
{"property": "spent_at", "lt": "2026-10-01T00:00:00Z"}
{"property": "spent_at", "eq": {"range": {"unit": "month", "value": "2026-07", "time_zone": "Asia/Tokyo"}}}
{"property": "memo", "is_null": true}
```

演算子と operand は次のとおりです。

| Property 型 | 演算子 | operand |
| --- | --- | --- |
| `text` | `eq`, `ne`, `contains`, `starts_with`, `ends_with` | `null` ではない string 一つ |
| `text` | `in` | `null` を含まない string の非空配列 |
| `number` | `eq`, `ne`, `lt`, `lte`, `gt`, `gte` | decimal string または number 範囲値 |
| `number` | `in` | decimal string または number 範囲値の非空配列 |
| `datetime` | `eq`, `ne`, `lt`, `lte`, `gt`, `gte` | RFC 3339 string または datetime 範囲値 |
| `datetime` | `in` | RFC 3339 string または datetime 範囲値の非空配列 |
| すべて | `is_null`, `is_not_null` | `true` |

`text` の一致と部分一致は Unicode code point に対する case-sensitive な比較とします。`number` は数値として、`datetime` は UTC の時点として比較します。null は `is_null` と `is_not_null` だけで照合し、他の演算子の operand には許可しません。一つの Property 述語には演算子を一つだけ指定します。

number の範囲値は両端を decimal string、datetime の明示的な範囲値は両端を RFC 3339 string で指定します。`lower` は `upper` 以下とし、等しい場合は両端が `closed` のときだけ許可します。

```json
{"range": {"lower": "1000.00", "lower_bound": "closed", "upper": "2000.00", "upper_bound": "open"}}
{"range": {"lower": "2026-07-01T00:00:00+09:00", "lower_bound": "closed", "upper": "2026-08-01T00:00:00+09:00", "upper_bound": "open"}}
```

datetime の calendar 範囲は次の形式とします。`month` の値は `YYYY-MM`、`day` の値は `YYYY-MM-DD` とし、`time_zone` は IANA time zone database の名前を必須とします。夏時間によって一日の長さが変わる場合も、固定時間ではなく現地時刻で次の期間開始を求めます。

```json
{"range": {"unit": "month", "value": "2026-07", "time_zone": "Asia/Tokyo"}}
{"range": {"unit": "day", "value": "2026-07-15", "time_zone": "Asia/Tokyo"}}
```

範囲 `R` の下端を `a`、上端を `b` としたとき、各演算子を次のように評価します。scalar は両端が同じ `closed` の範囲として扱うため、従来の scalar 比較と一致します。

- `eq R`: 値が `R` に含まれる。
- `ne R`: 値が `R` に含まれない。
- `lt R`: 値が下端より前にある。下端が `closed` なら `x < a`、`open` なら `x <= a`。
- `lte R`: 値が上端より後にない。上端が `closed` なら `x <= b`、`open` なら `x < b`。
- `gt R`: 値が上端より後にある。上端が `closed` なら `x > b`、`open` なら `x >= b`。
- `gte R`: 値が下端より前にない。下端が `closed` なら `x >= a`、`open` なら `x > a`。
- `in [R1, R2]`: 各要素への `eq` を OR で評価する。

calendar 範囲は常に下端が `closed`、上端が `open` です。たとえば `eq 2026-07` は 7 月内、`lt 2026-07` は 7 月より前、`lte 2026-07` は 7 月末まで、`gt 2026-07` は 8 月以降、`gte 2026-07` は 7 月以降を意味します。

### D-5: pagination と入力制限

#### 判断基準

- page 間で検索条件が変化しないこと
- 過大な式による DB と API への負荷を制限できること
- client が入力修正可能な error として扱えること

#### 選択肢

- **cursor を filter に束縛して上限を設ける:** client に制約が増えるが、page の一貫性と負荷を管理できる。
- **cursor と filter を独立させる:** 自由度は高いが、異なる条件へ cursor を再利用できてしまう。
- **制限を実装依存にする:** 調整しやすいが、生成 client が事前に把握できない。

#### 採用

**cursor を filter に束縛して上限を設ける**案を選びます。filter を正規化した値を opaque cursor の検証対象に含めます。上限は OpenAPI に生成し、違反は共通 problem detail を伴う `422 Unprocessable Content` とします。

## 影響

generator は Entity 型ごとの Property 述語 schema と、Entity/Link 用の再帰的な論理式 schema を生成します。DB 実装には型に応じた比較と複合条件の組み立てが必要です。NOT、並べ替え、全文検索、Link をたどる graph query はこの ADR の対象外です。
