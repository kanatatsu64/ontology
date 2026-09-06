# 0009: Entity と Link の API 操作およびライフサイクル

- 日付: 2026-09-06
- 意思決定者: プロジェクト
- 関連 ADR: 0003, 0007, 0008

## コンテキスト

API の粒度に加え、ID、再送、更新、重複 Link、Entity 削除時の Link、一覧取得、エラーの挙動を定めます。

## 用語

- **再送:** 応答を確認できなかった操作と同じ内容を、もう一度要求すること。
- **PATCH:** Entity 全体ではなく、指定した Property だけを更新する操作。
- **opaque cursor:** 利用者が内部を解釈せず、次の page の取得にだけ使う値。
- **keyset pagination:** 最後に取得した位置を基準に、次の page を取得する方式。
- **cascade:** 一つの削除に伴って、関連するデータも自動的に削除する動作。

## 決定

- Entity/Link 型ごとの create、get、delete、list と、Entity の patch を `/v1/entity-types/{type}/entities` および `/v1/link-types/{type}/links` 以下に生成します。
- ID は client-generated UUID とし、同じ ID・同じ内容の再送は成功、異なる内容は `409` とします。ID、型、Link 端点は変更不可とします。
- PATCH は未指定と明示的 `null` を区別し、default は作成時だけ適用します。
- 同じ Link 型・from・to の重複を `409` とします。関連 Link が残る Entity の削除も `409` とし、cascade は提供しません。
- 一覧は ID 昇順の opaque cursor/keyset pagination とし、page size の既定値と上限を設けます。
- 不在は `404`、入力形式違反と参照先不在は `422`、ID・一意性・参照 Link の競合は `409` とします。

## 検討

### D-1: API の型付け

#### 判断基準

- 生成 client が Property の型を静的に扱えること
- 未知の型と Property を受け入れないこと
- endpoint の命名を機械生成できること

#### 選択肢

- **型別 operation:** 仕様は増えるが、具体的な schema を生成できる。
- **完全な汎用 API:** endpoint は少ないが、検証が実行時になる。
- **両方:** 用途は広がるが、二つの契約を維持する必要がある。

#### 採用

**型別 operation**を選びます。path は複数形の言語規則に依存しない固定構造にし、仕様上は各 `{type}` を具体的な型 ID で展開します。

- `POST /v1/entity-types/{type}/entities`
- `GET|PATCH|DELETE /v1/entity-types/{type}/entities/{id}`
- `GET /v1/entity-types/{type}/entities`
- `POST /v1/link-types/{type}/links`
- `GET|DELETE /v1/link-types/{type}/links/{id}`
- `GET /v1/link-types/{type}/links` with from/to filter

### D-2: ID の生成と作成の再送

#### 判断基準

- retry で重複データを作らないこと
- offline の入力元でも ID を先に参照できること
- 同じ ID の異なる内容を黙って上書きしないこと

#### 選択肢

- **client-generated UUID:** retry は容易だが、client に生成責務がある。
- **server-generated UUID:** client は単純だが、timeout 後の再送判定が必要になる。
- **自然 key:** 可読だが、変更と衝突の扱いが難しい。

#### 採用

**client-generated UUID**を選びます。同じ ID・同じ内容の再送は成功、同じ ID・異なる内容は `409 Conflict` とします。Entity/Link の ID と型は変更できません。

### D-3: Entity の部分更新

#### 判断基準

- 未指定と明示的な `null` を区別できること
- 全 Property の再送を要求しないこと
- default の再適用で既存値を変えないこと

#### 選択肢

- **PATCH:** presence を扱えるが、生成 request 型に三状態が必要になる。
- **PUT:** 全体置換は明確だが、すべての Property の送信が必要になる。

#### 採用

**PATCH**を選び、指定された Property だけを置換します。明示的な `null` と未指定を区別し、任意 Property の default は作成時だけ適用します。

### D-4: Link の端点と重複

#### 判断基準

- Link の同一性が明確であること
- 同じ関係の重複を防ぐこと
- 有向性を維持すること

#### 選択肢

- **端点を不変かつ一意にする:** 変更には再作成が必要だが、同一性が明確になる。
- **端点を更新可能にする:** 操作は少ないが、Link の意味が途中で変わる。
- **端点重複を許す:** 多重辺を表現できるが、現在の Link は Property を持たず区別できない。

#### 採用

**端点を不変かつ一意にする**案を選びます。from/to は作成後に変更できず、同じ Link 型・from・to の別 ID による作成は `409 Conflict` とします。逆方向の検索でも Link の意味は反転しません。

### D-5: Entity 削除時の Link

#### 判断基準

- 意図しない関連データの消失を防ぐこと
- API と DB 外部キーの動作を一致させること
- 削除の影響範囲を呼び出し側が把握できること

#### 選択肢

- **削除を拒否:** Link の先行削除が必要だが、暗黙の変更がない。
- **Link を cascade 削除:** 操作は簡単だが、複数の Link が暗黙に消える。
- **Link の端点を null にする:** Link は残るが、ontology の端点制約を破る。
- **soft delete:** 復元できるが、全 query と unique 制約が複雑になる。

#### 採用

**削除を拒否**する案を選びます。from/to のいずれかに対象 Entity を持つ Link があれば `409 Conflict` とし、何も変更しません。cascade option は設けず、Link を明示的に削除させます。Link 削除が Entity を削除することもありません。

### D-6: 一覧取得

#### 判断基準

- 並行追加時の重複と欠落を抑えること
- 大きな offset の走査を避けること
- 順序が決定的であること

#### 選択肢

- **opaque cursor の keyset pagination:** 任意 page へ飛べないが、更新に比較的強い。
- **offset pagination:** 実装は単純だが、並行更新と大きな offset に弱い。
- **無制限一覧:** client は単純だが、data 増加に耐えない。

#### 採用

**opaque cursor の keyset pagination**を選び、ID 昇順、既定 page size、最大 page size を固定します。

### D-7: エラー分類

#### 判断基準

- client が再試行、入力修正、競合解消を区別できること
- 同じ種類の失敗が operation 間で同じ status になること

#### 選択肢

- **HTTP status と共通 problem detail:** client が分岐できるが、error code の管理が必要になる。
- **常に 400:** 単純だが、失敗理由を機械的に区別しにくい。

#### 採用

**HTTP status と共通 problem detail**を選びます。不在は `404`、入力形式違反と参照先不在は `422`、ID、unique、参照 Link の競合は `409` とします。

## 影響

Entity 削除には Link の検索と明示削除が必要です。bulk 操作、任意条件式、graph query、履歴保持は別の ADR で検討します。
