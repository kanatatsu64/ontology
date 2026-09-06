# 0015: 正規化済みオントロジー snapshot の形式

- 日付: 2026-09-06
- 意思決定者: プロジェクト
- 関連 ADR: 0002, 0003, 0004, 0005

## コンテキスト

migration の差分比較元を環境に依存せず再現するには、検証済みのオントロジー定義を意味の違いだけが残る形式で保存する必要があります。compiler 内の Rust 型をそのまま永続化すると、実装変更が snapshot の互換性と差分へ影響します。

## 用語

| 用語 | 定義 |
| --- | --- |
| IR | 検証済みのオントロジー定義を generator が共通利用する、正規化済みの内部表現。 |
| snapshot | ある時点の IR を、後から同じ意味で復元できるよう永続化した記録。 |
| 正規化 | 同じ意味を持つ表記の違いを取り除き、一つの表現に揃えること。 |
| canonical JSON | key、配列要素、scalar、空白、改行の表現を一つに固定した JSON。 |
| format version | snapshot の構造と解釈規則の世代を表す整数。 |
| digest | canonical JSON の内容を識別するために計算する SHA-256 hash。 |

## 決定

- IR の意味を欠落なく保存する canonical JSON を snapshot の永続形式とします。Rust の型名、enum 表現、memory layout は snapshot の契約に含めません。
- root は `snapshot_version`、`ontology_version`、`entities`、`links` をこの順で持ちます。`snapshot_version` は `1`、`ontology_version` は原典 YAML の `version` とします。
- `entities`、各 Entity 型の `properties`、`links` は配列とし、ID の UTF-8 byte 順に昇順で並べます。各要素は ID を明示的な field として持ちます。
- Entity 型は `id`、`description`、`properties`、Property は `id`、`description`、`type`、`required`、任意 Property のみ `default`、Link 型は `id`、`description`、`from`、`to` をこの順で持ちます。未知 field と重複 ID を拒否します。
- `type` は `text`、`number`、`datetime` のいずれかです。default は `text` を JSON string、`number` を正規化した decimal string、`datetime` を UTC の RFC 3339 string、null を JSON null として保存します。
- UTF-8、2 space indent、LF、末尾改行一つを使用します。JSON string は JSON が要求する文字だけを escape します。`description` は NFC に正規化し、`text` の default は Unicode 正規化せず code point の列を保持します。空の配列も省略しません。
- snapshot に時刻、tool version、source path など生成ごとに変わる metadata を含めません。digest は snapshot 自身に含めず、canonical JSON の全 byte から SHA-256 を計算し、小文字 hexadecimal で表します。
- 未対応の `snapshot_version` は読み込まず、明示的な変換を要求します。同じ version の schema を意味変更しません。

snapshot version 1 の構造例を示します。この例の field 順と format は規範の一部です。

```json
{
  "snapshot_version": 1,
  "ontology_version": 1,
  "entities": [
    {
      "id": "expense",
      "description": "支出",
      "properties": [
        {
          "id": "amount",
          "description": "金額",
          "type": "number",
          "required": true
        }
      ]
    },
    {
      "id": "merchant",
      "description": "支払先",
      "properties": []
    }
  ],
  "links": [
    {
      "id": "paid_to",
      "description": "支出の支払先",
      "from": "expense",
      "to": "merchant"
    }
  ]
}
```

scalar は次のように正規化します。

| scalar | 正規化規則 |
| --- | --- |
| ID | 検証済みの小文字 ASCII をそのまま保存する。 |
| `description` | Unicode を NFC に正規化する。 |
| `text` | Unicode 正規化を行わず、code point の列をそのまま保存する。 |
| `number` | `+` と指数表記を使わず、整数部の不要な先頭 `0` と小数部の末尾 `0` を除く。`-0` は `0` とする。 |
| `datetime` | UTC の `Z` 表記へ変換し、小数秒の末尾 `0` を除く。小数部が空なら小数点も除く。 |
| null | JSON null として保存する。 |

`text` は ADR 0014 の code point 比較と意味を揃えます。例えば `"\u00e9"` と `"e\u0301"` は異なる default として保存し、変更時は異なる digest と migration 差分を生じさせます。この組を snapshot の round-trip と差分検出の fixture に含めます。

## 検討

### D-1: 永続形式

#### 判断基準

- Git の差分を人がレビューできること
- 実装言語を変更しても読み書きできること
- 同じ IR から byte 単位で同じ結果を生成できること

#### 選択肢

- **canonical JSON:** 冗長だが、人と複数言語から読み取れ、表現を固定できる。
- **YAML:** 原典と似て読みやすいが、暗黙変換や複数の等価表現を再び持ち込む。
- **Rust 固有 binary:** 小さく高速だが、実装型と serializer version に結合する。

#### 採用

**canonical JSON**を採用します。

- 判断基準「レビュー可能性」: text の差分として、型、Property、Link の変更を確認できます。
- 判断基準「実装言語からの独立」: JSON の field と scalar の規則を契約とし、Rust の型表現を含めません。
- 判断基準「再現性」: field 順、配列順、scalar、空白、改行を固定して同じ IR を同じ byte 列にします。

### D-2: collection の表現

#### 判断基準

- 順序に意味を持たせないこと
- JSON Schema で要素の構造を検証できること
- ID の追加と変更を差分で識別できること

#### 選択肢

- **ID 順の配列と明示的な ID:** 並べ替えが必要だが、要素 schema を固定できる。
- **ID を key にした object:** lookup は直接できるが、動的 key と field 順の規則が増える。

#### 採用

**ID 順の配列と明示的な ID**を採用します。

- 判断基準「順序の非意味化」: compiler が ID 順へ並べ、原典の記述順を破棄します。
- 判断基準「検証可能性」: 配列要素を固定 schema で検証し、重複 ID を別途拒否します。
- 判断基準「差分の識別」: 各要素が ID を持つため、追加、削除、内容変更を区別できます。

### D-3: format の進化

#### 判断基準

- 古い snapshot を異なる意味で誤読しないこと
- format 変更を migration 生成前に検出できること
- 同じ version の結果を安定させること

#### 選択肢

- **整数 version と明示変換:** 変換処理が必要だが、互換性境界が明確になる。
- **常に最新版として解釈:** version 管理は不要だが、古い snapshot を誤読し得る。

#### 採用

**整数 version と明示変換**を採用します。

- 判断基準「誤読防止」: 未対応 version を即座に拒否します。
- 判断基準「変更検出」: version の差を ontology の差分比較より先に検出します。
- 判断基準「安定性」: 同じ version の field と正規化規則を変更しません。

## 影響

snapshot reader と writer は同じ fixture に対して round-trip と byte 単位の一致を検査します。format を変更する場合は新しい snapshot version と変換規則を別の ADR で定めます。migration metadata が参照する digest は、この ADR の canonical JSON に対して計算します。
