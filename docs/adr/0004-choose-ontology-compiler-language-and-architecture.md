# 0004: オントロジーコンパイラーの実装言語と内部構造

- 日付: 2026-09-06
- 意思決定者: プロジェクト
- 関連 ADR: 0002, 0003

## コンテキスト

複数成果物を同じ解釈から決定的に生成する CLI が必要です。

## 決定

- compiler と CLI を Rust で実装します。
- parser、validator、正規化済み IR、diff、各 generator、CLI を分離し、全 generator は同じ IR を直接入力にします。
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
- **generator を直列接続:** 中間成果物に依存し、変更が伝播する。

#### 採用

**共通の正規化済み IR**を選びます。parser、validator、IR、diff、generator、CLI を crate 上で分離し、generator 間の呼び出しを禁止します。

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
