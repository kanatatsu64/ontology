# 共通規約

## 変更の原則

- 一つの変更は一つの目的に絞り、無関係な整形や rename を混ぜません。
- public な振る舞いを変更するときは、実装、test、利用者向け文書、契約を同じ変更で更新します。
- 同じ知識を複数箇所へ複製せず、唯一の正を定めて参照または生成します。
- 後方互換性を壊す変更には、影響範囲と移行方法を記録します。architecture 上の判断を変える場合は新しい ADR を作成します。

## 命名と file

- folder と Rust crate は小文字 ASCII の kebab-case、Rust module と source file は snake_case にします。
- 名前は略語より役割を優先します。domain の概念には[ユビキタス言語](../ubiquitous/platform.md)と同じ語を使います。
- text file は UTF-8、LF、末尾改行一つとし、`.editorconfig` に従います。
- credential、token、秘密鍵、個人情報、production data を commit しません。例示値は実在しないことが明確な値にします。
- repository の配置は [ADR 0016](../adr/0016-define-repository-directory-structure.md) に従い、使うまで空 folder を作りません。

## Error と log

- error を無視しません。回復する、呼び出し元へ返す、または終了させるかを明示します。
- error message には失敗した操作と対象を含め、credential や入力データ全体を含めません。
- log は解析可能な構造化形式を基本とし、level を意図して選びます。同じ error を複数 layer で重複して記録しません。
- 利用者向け error と内部原因を分け、外部 API から内部実装、SQL、stack trace を漏らしません。

## Test

- bug 修正では、修正前に失敗し修正後に成功する regression test を追加します。
- test は arrange、act、assert の境界が分かる構造にし、一つの test で一つの振る舞いを検証します。
- 時刻、乱数、環境変数、外部 service に依存する test は入力を固定または注入し、再現可能にします。
- 正常系だけでなく、境界値、不正入力、権限不足、失敗時に状態が壊れないことを対象にします。
- fixture は最小限にし、何を表すか分かる名前を付けます。生成結果は意味の確認と byte 単位の再現性の両方を検査します。

## 自動生成物

- file 内の生成表記または同じ folder の README に、generator、入力、再生成 command、手修正可否を記載します。
- 生成順に依存しない決定的な出力にし、時刻や local path など実行ごとに変わる値を含めません。
- 生成結果を変更する PR では原典または generator の差分も含め、CI で再生成後に差分がないことを確認します。
