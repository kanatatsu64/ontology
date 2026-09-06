# 0006: オントロジーデータの永続化方式

- 日付: 2026-09-06
- 意思決定者: プロジェクト
- 関連 ADR: 0001, 0003

## コンテキスト

型、Link の参照整合性、transaction、migration を支える書き込み元を選びます。

## 用語

- **参照整合性:** Link が存在する Entity だけを参照することを保証する性質。
- **transaction:** 複数の変更をすべて成功またはすべて不成立として扱う単位。
- **書き込み元:** データ更新の正として扱い、他の表現の基準となる保存先。
- **読み取りモデル:** 特定の参照方法に最適化して、書き込み元から派生させたデータ表現。

## 決定

PostgreSQL を固定 major version で書き込み元として使用し、各 API 操作を一つの DB transaction で確定します。graph 読み取りも SQL で開始し、別の読み取りモデルを追加しても PostgreSQL を書き込み元に保ちます。

## 検討

### D-1: データベース製品

#### 判断基準

- Property の型と Link の参照を制約できること
- 複数操作を transaction で確定できること
- SQL migration を運用できること

#### 選択肢

- **PostgreSQL:** 制約、transaction、numeric、運用手段が揃うが、SQL が製品依存になる。
- **graph DB:** Link 走査に適するが、schema migration と Property 制約が製品固有になる。
- **SQLite:** local 配布は容易だが、共有 service の並行処理と運用が限られる。

#### 採用

**PostgreSQL**を選びます。API 操作は一つの DB transaction で確定し、generator も固定 major version だけを対象にします。

### D-2: graph 読み取りの扱い

#### 判断基準

- 書き込み整合性の正を一つに保つこと
- 必要性が確認される前に同期機構を増やさないこと

#### 選択肢

- **PostgreSQL の SQL:** 追加基盤は不要だが、深い走査に限界があり得る。
- **graph DB へ二重書き込み:** 走査は強いが、同期と障害処理が必要になる。

#### 採用

**PostgreSQL の SQL**を選びます。計測で不足した場合のみ別の読み取りモデルを検討し、PostgreSQL は書き込み元に保ちます。

## 影響

生成 SQL、型、backup、運用が PostgreSQL に依存します。
