# 0003: オントロジー YAML の構造と型体系

- 日付: 2026-09-06
- 意思決定者: プロジェクト
- 関連 ADR: 0002

## コンテキスト

YAML から DB と API を一意に生成するため、構文、識別子、型、デフォルト値、Link の向きを定めます。

## 用語

| 用語 | 定義 |
| --- | --- |
| mapping | key と値の組を集めた YAML の構造。 |
| nullable | 値として null を許す性質。 |
| RFC 3339 | 日時と UTC からの offset を表す文字列表現の規格。 |
| anchor / alias | YAML 内の値を再利用するための参照機能。 |
| 暗黙の型変換 | 明示された型指定なしに、文字列などを別の型として解釈すること。 |

## 決定

- YAML のトップレベルを `version: 1`、`entities`、`links` の mapping とします。
- Entity 型、Property、Link 型の ID は `^[a-z][a-z0-9_]*$` とし、各定義に空でない `description` を必須とします。Property ID は Entity 型内で一意とし、Entity の識別情報に使う `id` と `type` を予約名として拒否します。
- Entity 実データは `id`、`type`、Property を、Link 実データは `id`、`type`、from/to Entity ID を持ちます。Property は Entity だけが持ちます。
- Property 型を nullable な `text`、任意精度十進数の `number`、RFC 3339 の時点を表す `datetime` に限定します。`text` は default、実データ、検索の比較値を通じて Unicode code point の列を保持し、Unicode 正規化を行いません。
- `required` は作成時の指定有無を表します。任意 Property には同じ型または `null` の `default` を必須とし、必須 Property に `default` を許可しません。
- Link は有向とし、Link 型ごとに from/to Entity 型を固定します。双方向関係は逆向きの二つの Link 型で表します。
- 未知 field、重複 key、未定義 Entity 型への Link、型と不一致な default、anchor、alias、暗黙の型変換を拒否します。

## 検討

### D-1: YAML の全体構造

#### 判断基準

- ID の一意性を検証しやすいこと
- 人が差分を読みやすいこと
- 将来の構文変更を識別できること

#### 選択肢

- **ID をキーにした mapping:** 構文上重複しにくいが、順序に意味を持たせられない。
- **`id` を持つ配列:** 順序を保持できるが、重複 ID を別途検出する必要がある。

#### 採用

**ID をキーにした mapping**を選びます。定義順に意味はなく、一意性と差分の可読性を優先します。トップレベルは `version`、`entities`、`links` だけを許可します。

```yaml
version: 1
entities:
  expense:
    description: 支出
    properties:
      amount:
        description: 金額
        type: number
        required: true
      memo:
        description: メモ
        type: text
        required: false
        default: null
      spent_at:
        description: 支出日時
        type: datetime
        required: false
        default: "1970-01-01T00:00:00Z"
  merchant:
    description: 支払先
    properties:
      name:
        description: 名称
        type: text
        required: true
links:
  paid_to:
    description: 支出の支払先
    from: expense
    to: merchant
```

### D-2: ID と description

#### 判断基準

- 生成物で安定して参照できること
- 定義の意図をレビュー時に理解できること
- DB 識別子へ安全に変換できること

#### 選択肢

- **制約付き ID と必須 description:** 記述量は増えるが、機械処理と理解の両方を満たす。
- **自由な名前だけ:** 記述は短いが、改名と同一性を区別しにくい。
- **自動採番 ID:** 一意だが、YAML と生成物の対応を読み取りにくい。

#### 採用

**制約付き ID と必須 description**を選びます。Entity 型、Property、Link 型の ID は `^[a-z][a-z0-9_]*$` とし、Entity 型、Property、Link 型の定義には空でない `description` を必須とします。Property ID は Entity 型内で一意とします。

Property ID の `id` と `type` は検証時に拒否します。Entity の識別情報との衝突を防ぎ、同名 column と API field への安定した写像を維持します。この予約は Entity 型と Link 型の ID には適用しません。

### D-3: Entity と Link の実データ

#### 判断基準

- 実データから型を一意に特定できること
- Link の端点と向きが曖昧でないこと
- Link の属性モデルを増やさないこと

#### 選択肢

- **ID と型を実データに持たせる:** 自己記述的だが、型ごとの保存先では型が重複する。
- **保存場所から型を推測する:** 保存は短いが、API payload 単体で型を特定できない。

#### 採用

**ID と型を実データに持たせる**を選びます。Entity は `id`、`type`、定義された Property を持ちます。Link は `id`、`type`、from Entity ID、to Entity ID を持ちます。Property は Entity だけが持ちます。

### D-4: Property の型

#### 判断基準

- 初期ユースケースの支出、金額、日時、説明を表現できること
- DB、OpenAPI、Rust へ精度を失わず写像できること
- 初期 generator の実装範囲を抑えること

#### 選択肢

- **`text`、`number`、`datetime`:** 初期用途を満たすが、真偽値は直接表現できない。
- **汎用 JSON Schema 型:** 表現力は高いが、全 generator の対応範囲が広がる。
- **すべて text:** 実装は容易だが、検証と DB の型を活用できない。

#### 採用

**`text`、`number`、`datetime`**を選び、すべて値として `null` を許します。`text` は Unicode 文字列、`number` は有限の任意精度十進数、`datetime` は offset を必須とする RFC 3339 の時点とします。小数秒は最大 6 桁とし、受信時に UTC へ正規化します。

### D-5: Property の必須性とデフォルト値

#### 判断基準

- 未指定と明示的な `null` を区別できること
- 任意 Property の省略結果が決定的であること
- 型に合わないデフォルトを生成前に拒否できること

#### 選択肢

- **必須性と nullability を分離:** 状態を正確に表せるが、API と DB の検証が増える。
- **必須を non-null と同義にする:** 単純だが、すべての型が `null` を含む要件を満たさない。

#### 採用

**必須性と nullability を分離**します。`required` は作成時の指定有無だけを表します。任意 Property は同じ型または `null` の `default` を必須とし、必須 Propertyには `default` を許可しません。

### D-6: Link の方向と型

#### 判断基準

- from/to の意味が常に一意であること
- Link 型から端点の Entity 型を検証できること
- 双方向関係でも各方向の意味を記述できること

#### 選択肢

- **有向 Link:** 二方向には二定義が必要だが、意味と走査方向が明確になる。
- **無向 Link:** 定義は短いが、非対称な関係を扱えない。
- **一つの双方向 Link:** 走査は容易だが、方向ごとに異なる意味を持たせにくい。

#### 採用

**有向 Link**を選びます。同じ Link 型は一つの from Entity 型と一つの to Entity 型を固定します。双方向関係は、向きが逆で ID の異なる二つの Link 型で表します。

### D-7: YAML の入力制約

#### 判断基準

- 同じ意味から同じ中間表現を生成できること
- YAML parser 固有の挙動を定義へ持ち込まないこと
- 誤記を黙って受け入れないこと

#### 選択肢

- **制限した YAML subset:** 表現力は下がるが、入力が決定的になる。
- **YAML の全機能:** 再利用しやすいが、anchor や暗黙変換がレビューを難しくする。

#### 採用

**制限した YAML subset**を選びます。未知フィールド、重複キー、未定義 Entity 型への Link、型と不一致な default をエラーにします。anchor、alias、暗黙の型変換は許可しません。

## 影響

YAML の互換性は `version` で管理します。ID の変更は削除と追加として扱います。型の追加や Link の Property が必要になった場合は、新しい ADR でこのモデルを変更します。
