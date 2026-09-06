# 0004: オントロジーコンパイラーの実装言語と内部構造

- 日付: 2026-09-06
- 意思決定者: プロジェクト
- 関連 ADR: 0002, 0003

## コンテキスト

複数成果物を同じ解釈から決定的に生成する CLI が必要です。

## 用語

| 用語 | 定義 |
| --- | --- |
| compiler | オントロジー定義を検証し、別の利用可能な表現へ変換する tool。 |
| CLI | command line から compiler の操作を実行する入口。 |
| IR | 検証済みのオントロジーを各 generator が共通利用する内部表現。 |
| generator | IR や OpenAPI などの定義から特定の成果物を作る処理。 |
| byte 単位の再現性 | 同じ入力から内容が完全に同一の file を生成できる性質。 |

## 決定

- compiler と CLI を Rust で実装します。
- parser、validator、正規化済み IR、diff、各 generator、CLI を分離します。snapshot と OpenAPI は同じ IR から、SQL migration は旧 snapshot と現在の IR の意味的な差分から生成します。
- Rust server/client は ADR 0010 に従い、生成した OpenAPI を入力に生成します。CLI が各段階を順に実行し、generator 同士は直接呼び出しません。
- CLI は `validate`、`generate`、`check` を提供し、診断位置、出力順、format、改行を固定して byte 単位の再現性を保証します。
- IR は公開契約にせず、保存する snapshot 形式だけを version 管理します。

## 検討

### D-1: 実装言語

#### 判断基準

- 不変条件を型で表せること
- CI と local へ同じ実行物を配布できること
- platform の実装言語と統一できること

#### 選択肢

- **Rust:** 単一 binary と強い型を得られるが、実装量と compile 時間が増える。
- **TypeScript:** Web 周辺の資産は多いが、platform と toolchain が分かれる。
- **Python:** 試作しやすいが、型制約と配布方法が弱い。

#### 採用

**Rust**を選びます。

### D-2: generator の入力

#### 判断基準

- SQL と API で同じ ontology 解釈を共有できること
- YAML の表記差を成果物へ伝播させないこと
- generator を独立して test できること

#### 選択肢

- **共通の正規化済み IR:** 構築工程は増えるが、意味を一元化できる。
- **各 generator が YAML を読む:** 単純だが、解釈が分岐し得る。
- **SQL から API を生成:** DB の物理表現に依存し、作成時の必須性などの意味が失われる。

#### 採用

**共通の正規化済み IR**を選びます。snapshot と OpenAPI は IR を直接入力とし、SQL migration は ADR 0005 の意味的な差分を入力とします。SQL と API の解釈を揃え、各段階を独立して test できる構造にします。

Rust server/client は公開 API 契約を共有するため `IR → OpenAPI → Rust` の順で生成します。parser、validator、IR、diff、generator、CLI を crate 上で分離し、段階間の受け渡しは CLI が担います。generator 間の直接呼び出しは禁止します。

### D-3: CLI と再現性

#### 判断基準

- CI と手動操作が同じ処理を使うこと
- 不正箇所を利用者が特定できること
- 同じ入力から同じ bytes を得られること

#### 選択肢

- **単一 CLI:** 境界が一つになるが、subcommand の互換性管理が必要になる。
- **CI script と local tool を分離:** 個別最適化できるが、挙動がずれ得る。

#### 採用

**単一 CLI**を選び、`validate`、`generate`、`check` を提供します。診断位置、出力順、format、改行を固定します。

## 影響

IR は公開契約にせず、保存する snapshot 形式だけを version 管理します。
