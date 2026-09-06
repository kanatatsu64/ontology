# 0007: PostgreSQL におけるオントロジーの物理スキーマ

- 日付: 2026-09-06
- 意思決定者: プロジェクト
- 関連 ADR: 0003, 0006

## コンテキスト

migration を生成するため、Entity、Property、Link、必須性、デフォルト値を PostgreSQL の具体的なオブジェクトへ写像します。

## 用語

| 用語 | 定義 |
| --- | --- |
| 物理 schema | オントロジーを DB の table、column、制約として表した構造。 |
| primary key / foreign key | 行を一意に識別する制約と、別の行への参照を保証する制約。 |
| unique 制約 | 指定した値の組が重複しないことを保証する制約。 |
| index | 条件に合う行を効率よく見つけるための DB の構造。 |
| `ON DELETE RESTRICT` | 参照されている行の削除を拒否する規則。 |

## 決定

- Entity 型を UUID primary key の `entity_<type_id>` table、Property を nullable な同名 column として生成します。Property ID の `id` と `type` は ADR 0003 に従って生成前に拒否します。`text`、`number`、`datetime` は `text`、`numeric`、`timestamptz` に写像し、任意 Property の DB default を生成します。
- Link 型を UUID primary key、non-null の `from_id`/`to_id`、両端の foreign key、`UNIQUE (from_id, to_id)` を持つ `link_<type_id>` table として生成します。
- foreign key は両端とも `ON DELETE RESTRICT` とし、from 側は unique index、to 側は追加 index で検索します。
- SQL 識別子は小文字 ASCII の prefix 付き ID とし、PostgreSQL の長さ上限を超える場合は安定 hash で短縮します。

## 検討

### D-1: Entity の格納方式

#### 判断基準

- PostgreSQL の型と制約を利用できること
- 通常の SQL で読み取れること
- ontology 変更から migration を生成できること

#### 選択肢

- **型別テーブル:** DB の型を利用できるが、型追加が DDL になる。
- **共通テーブルと JSONB:** 型追加は容易だが、Property 制約が弱くなる。
- **EAV:** 動的だが、query と整合性制約が複雑になる。

#### 採用

**型別テーブル**を選び、`entity_<entity_type_id>` と命名します。Entity の型はテーブルで確定するため、`type` 列は保存しません。ID は client-generated UUID の primary key とします。

```sql
CREATE TABLE entity_expense (
    id uuid PRIMARY KEY,
    amount numeric NULL,
    memo text NULL DEFAULT NULL,
    spent_at timestamptz NULL DEFAULT TIMESTAMPTZ '1970-01-01T00:00:00Z'
);
```

### D-2: Property の列型と制約

#### 判断基準

- 論理型の値域と精度を維持できること
- null と未指定の要件を維持できること
- DB への直接書き込みでもデフォルトが一致すること

#### 選択肢

- **PostgreSQL の対応型:** DB 機能を活用できるが、PostgreSQL に依存する。
- **すべて text:** 共通化できるが、型検査と数値・日時演算を失う。
- **JSONB 一列:** presence を保持できるが、列単位の制約と index が複雑になる。

#### 採用

**PostgreSQL の対応型**を選び、`text`、`number`、`datetime` を `text`、`numeric`、`timestamptz` へ写像します。全 Property 列を nullable とし、任意 Property の default を `DEFAULT` に生成します。必須 Property の presence は API で検証します。

### D-3: Link の格納方式

#### 判断基準

- Link 型ごとの from/to Entity 型を DB で保証できること
- 両方向から index を利用して検索できること
- 同じ型・同じ端点の重複を防げること

#### 選択肢

- **Link 型別テーブル:** 通常の外部キーを利用できるが、型追加が DDL になる。
- **全 Link 共通テーブル:** 横断走査は容易だが、型ごとの端点を外部キーで保証できない。
- **Entity 行内の配列:** 読み取りは局所的だが、参照整合性と逆方向検索が難しい。

#### 採用

**Link 型別テーブル**を選び、`link_<link_type_id>` と命名します。Link 型はテーブルで確定するため、`type` 列は保存しません。ID を UUID primary key とし、from/to を外部キー、両端を unique 制約、to 側を追加 index にします。

```sql
CREATE TABLE link_paid_to (
    id uuid PRIMARY KEY,
    from_id uuid NOT NULL,
    to_id uuid NOT NULL,
    CONSTRAINT link_paid_to_from_fk
        FOREIGN KEY (from_id) REFERENCES entity_expense (id) ON DELETE RESTRICT,
    CONSTRAINT link_paid_to_to_fk
        FOREIGN KEY (to_id) REFERENCES entity_merchant (id) ON DELETE RESTRICT,
    CONSTRAINT link_paid_to_endpoints_unique UNIQUE (from_id, to_id)
);
CREATE INDEX link_paid_to_to_idx ON link_paid_to (to_id);
```

### D-4: Entity 削除時の参照動作

#### 判断基準

- 意図しないデータ消失を防ぐこと
- API と DB の動作が一致すること
- 一回の削除が影響する範囲を利用者が把握できること

#### 選択肢

- **`RESTRICT`:** Link の明示削除が必要だが、暗黙の消失を防げる。
- **`CASCADE`:** 操作は簡単だが、複数 Link が暗黙に消える。
- **`SET NULL`:** Link を残せるが、端点のない Link がモデルに反する。

#### 採用

**`RESTRICT`**を選び、from/to の両外部キーに指定します。

### D-5: SQL 識別子

#### 判断基準

- 任意の有効な ontology ID から生成できること
- PostgreSQL の長さ制限内で衝突しないこと
- 同じ入力から同じ名前を得られること

#### 選択肢

- **接頭辞と安定 hash:** 名前が長い場合に読みにくくなるが、決定的に生成できる。
- **連番:** 短いが、定義順の変更で不安定になる。
- **常に quoted identifier:** 元 ID を残せるが、手書き SQL が扱いにくい。

#### 採用

**接頭辞と安定 hash**を選びます。小文字 ASCII の ID に `entity_` または `link_` を付け、PostgreSQL の上限を超える場合は末尾を hash に置換します。制約名と index 名にも同じ規則を適用します。

## 影響

ontology 変更は DDL を発生させます。API は必須 Property の presence を検証します。Rust 側も `number` を二進浮動小数点ではなく十進数として扱います。
