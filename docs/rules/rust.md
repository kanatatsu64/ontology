# Rust コーディング規約

この文書は[共通規約](general.md)に加えて Rust code へ適用します。

## Toolchain と自動検査

- Rust edition と toolchain version は workspace で一つに固定します。
- `cargo fmt --all --check` を通し、`rustfmt` の標準設定から変更する場合は workspace 全体で設定します。
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` を通します。lint を抑制する場合は最小 scope にし、理由を comment に記載します。
- `cargo test --workspace --all-features` を通します。feature ごとの差がある場合は、必要な組み合わせも CI で検査します。
- dependency は workspace で version を揃え、追加時に保守状況、license、脆弱性、必要な feature を確認します。default feature を無条件に有効にしません。

## 設計

- module は一つの責務を持ち、既定では item を非公開にします。必要な最小範囲だけ `pub(crate)` または `pub` にします。
- domain の型で不正な状態を表現しにくくします。意味の異なる ID や値を同じ primitive のまま受け渡さず、newtype や enum を使います。
- I/O、時刻、乱数、環境設定を domain logic から分離し、test で制御できる境界を設けます。
- trait は実際に複数実装、test double、または layer 境界が必要な場所で導入し、将来の可能性だけで抽象化しません。
- `unsafe` は原則禁止します。必要な場合は安全性の前提、不変条件、代替案を文書化し、対象を最小 scope に限定します。

## 命名と API

- Rust API Guidelines と Rust の慣例に従い、type と trait は `UpperCamelCase`、function、variable、module は `snake_case`、constant は `SCREAMING_SNAKE_CASE` にします。
- boolean は `is_`、`has_`、`can_` など真偽が読める名前にします。単位を持つ数値は型または名前で単位を明示します。
- public item には利用者の判断に必要な rustdoc を付けます。error、panic、安全性の条件、重要な例を必要に応じて記述します。
- 所有権を明確にし、不要な `clone`、allocation、`to_owned` で borrow の問題を回避しません。性能上重要な妥協には根拠を残します。

## Error 処理

- 回復可能な失敗は `Result` で返し、library code では通常の入力や I/O error に `panic!`、`unwrap`、`expect` を使いません。
- `unwrap` と `expect` は test、または不変条件により失敗不能であることを直前に説明できる場合だけ使用します。`expect` の message は成立すべき条件を記述します。
- library の error は呼び出し側が分類できる型にし、application boundary で context を加えて利用者向け応答や終了 code に変換します。
- error の変換で原因 chain を失わず、機密情報を `Display` や log に含めません。

## Test

- private な振る舞いの unit test は同じ source file の `#[cfg(test)] mod tests`、public boundary の integration test は crate の `tests/` に置きます。
- test 名は条件と期待結果が分かる snake_case にします。
- compiler の各段階は正常入力と診断を個別に検査し、generator は fixture による golden test と再実行時の一致を検査します。
- 非同期 test に任意の sleep を使わず、clock、通知、timeout を制御します。
