# 0016: リポジトリのフォルダ構成

- 日付: 2026-09-06
- 意思決定者: プロジェクト
- 関連 ADR: 0001, 0002, 0004, 0010, 0012, 0013, 0015

## コンテキスト

このリポジトリには、オントロジー定義、compiler、API、migration runner、生成物、インフラストラクチャ定義、文書が共存します。実装の追加前に置き場所と依存方向を定め、実行単位、再利用可能な code、入力、生成物をフォルダ名から区別できるようにします。

## 用語

| 用語 | 定義 |
| --- | --- |
| workspace | 一つの root 設定で build と依存関係を管理する Rust package の集合。 |
| application | 個別に実行または配布できる binary の entry point。 |
| crate | Rust の compilation unit となる package。 |
| 生成物 | オントロジー定義などの入力から tool によって再生成できる file。 |
| fixture | test の入力または期待結果として version 管理する固定データ。 |
| Terraform root module | 一つの state として初期化、plan、apply する Terraform 構成の単位。 |

## 決定

- リポジトリ root を Rust workspace の root とし、共通の build、lint、依存 version を root の `Cargo.toml` で管理します。
- top-level folder は次の責務に固定します。新しい top-level folder は、既存の責務へ配置できないことを確認して ADR で追加します。

```text
.
├── apps/          # 実行・配布する application
├── crates/        # 複数箇所から利用できる Rust library と開発用 CLI
├── ontology/      # 人が編集するオントロジー定義
├── generated/     # オントロジーから作る、commit 対象の生成物
├── infra/         # cloud resource と deploy の構成
├── tests/         # workspace をまたぐ test と fixture
└── docs/          # ADR、規約、説明文書
```

- `apps/` は application ごとに一つの crate を置き、`apps/api/` と `apps/migration-runner/` を最初の単位とします。business rule や生成処理を application crate に重複させません。
- `crates/` は責務ごとに一つの library crate を置きます。compiler は parser、validator、IR、diff、generator を内部 module として分離した `crates/ontology-compiler/` から開始し、独立した公開境界または依存関係が必要になった場合だけ crate を分割します。compiler CLI は `crates/ontology-cli/` に置きます。
- `ontology/` の YAML を人が編集する原典とします。共通定義は `ontology/` 直下、環境固有の値が必要になった場合は `ontology/environments/<environment>/` に置き、原典と secret を混在させません。
- `generated/` は generator ごとの `openapi/`、`migrations/`、`snapshots/`、`rust/` に分けます。各 file の先頭または同じ folder の README に生成 command と手修正可否を記載します。生成物から原典や手書き code を参照してよい一方、手書き code を `generated/` に置きません。
- `infra/` は Terraform を使う infrastructure as code の root とします。再利用する構成は `infra/modules/<name>/`、環境ごとに独立した state と変数を持つ root module は `infra/environments/<environment>/` に置きます。provider と backend の設定は各 root module に明示し、環境差分を conditional resource ではなく root module と入力変数で表現します。credential、secret、`.tfvars`、plan、local state は commit しません。
- unit test は対象 module と同じ crate に置きます。crate 内だけで完結する integration test は `<crate>/tests/`、workspace をまたぐ end-to-end test と共有 fixture は root の `tests/` に置きます。
- `docs/adr/` は意思決定、`docs/rules/` は作業時に守る規約、その他の `docs/` は現状の説明に使用します。規約は ADR の決定を上書きできません。
- folder と crate の名前には小文字 ASCII の kebab-case を使います。Rust module と source file には snake_case を使います。
- 依存方向は `apps` から `crates`、`infra` から配布物、generator から `ontology` へ向けます。`crates` は `apps` または `infra` に依存せず、手書き code は生成済み server/client interface を除いて `generated` に依存しません。循環依存を許可しません。

実装開始時の想定構成を示します。未使用の folder は空のまま作成しません。

```text
.
├── Cargo.toml
├── apps/
│   ├── api/
│   └── migration-runner/
├── crates/
│   ├── ontology-cli/
│   └── ontology-compiler/
├── ontology/
├── generated/
│   ├── migrations/
│   ├── openapi/
│   ├── rust/
│   └── snapshots/
├── infra/
│   ├── environments/
│   │   └── production/
│   └── modules/
├── tests/
└── docs/
    ├── adr/
    ├── rules/
    └── ubiquitous/
```

## 検討

### D-1: top-level の分類軸

#### 判断基準

- file が手書きか生成物かを明確に識別できること
- application と再利用可能な library の境界を保てること
- build、review、CODEOWNERS の対象を folder 単位で指定できること

#### 選択肢

- **責務別の top-level folder:** folder は増えるが、変更理由と ownership を分離できる。
- **すべてを Rust workspace の `crates/` に置く:** Rust code は単純だが、入力、生成物、infra の性質が名前から分からない。
- **機能別の縦割り:** 一機能の変更はまとまるが、共有 generator と複数 application の境界が曖昧になる。

#### 採用

**責務別の top-level folder**を採用します。

- 判断基準「原典と生成物の識別」: `ontology/` と `generated/` を分離し、review 時に手修正の可否を判断できます。
- 判断基準「実行境界」: `apps/` と `crates/` を分け、配布単位と library を区別できます。
- 判断基準「運用単位」: code、生成物、infra、文書を独立した path として検査できます。

### D-2: compiler の crate 分割

#### 判断基準

- parser、validator、IR、diff、generator の責務を分離できること
- 初期実装の package 管理と公開 API を必要以上に増やさないこと
- 将来、必要な境界だけを独立させられること

#### 選択肢

- **一つの library crate 内で module 分割:** crate 内 API の規律は必要だが、依存 version と公開面を抑えられる。
- **責務ごとに最初から crate 分割:** 依存方向を強制できるが、package と公開 API が増える。
- **CLI crate にすべて実装:** 構成は小さいが、application や test から compiler を再利用しにくい。

#### 採用

**一つの library crate 内で module 分割**を採用し、CLI の entry point だけを別 crate にします。

- 判断基準「責務分離」: module 境界と非公開 API で各段階を分離します。
- 判断基準「単純さ」: 安定していない内部境界を workspace package として公開しません。
- 判断基準「発展性」: compile time、ownership、再利用の必要性が明確になった module は後から crate に切り出せます。

### D-3: test の配置

#### 判断基準

- test と対象 code の距離を短くすること
- workspace 全体の振る舞いを一つの場所で確認できること
- fixture を目的不明の共有物にしないこと

#### 選択肢

- **scope に応じて colocate と root を使い分ける:** 配置判断が必要だが、test の対象範囲を表現できる。
- **すべて root の `tests/` に集約:** 一覧性はあるが、crate 固有 test と実装の距離が離れる。
- **すべて各 crate に配置:** unit test は近いが、application 間の end-to-end test の所属が曖昧になる。

#### 採用

**scope に応じて colocate と root を使い分ける**方式を採用します。

- 判断基準「近接性」: unit test と crate integration test は対象 crate に置きます。
- 判断基準「全体検証」: 複数 crate または application を通る test だけを root に置きます。
- 判断基準「fixture の明確さ」: fixture は利用する test scope の配下に置き、広い scope へ無条件に共有しません。

## 影響

### 良い影響

- 原典、生成物、実行単位、library、infra の違いを path から判断できます。
- CI と review rule を変更種別ごとに適用できます。
- application を追加しても、platform の共通 code と deploy 単位を分離できます。

### 悪い影響・トレードオフ

- 一つの機能変更が複数の top-level folder にまたがる場合があります。
- 小規模な初期段階でも配置規則を守り、crate 分割の妥当性を継続して見直す必要があります。
- 新しい種類の artifact を追加する際、既存分類への適合を検討する必要があります。
