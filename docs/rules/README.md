# コーディング規約

この folder は、日々の実装と review で守る規約をまとめます。設計判断は [ADR](../adr/README.md) が優先し、規約と ADR が矛盾する場合は ADR に従います。

## 規約一覧

| 文書 | 対象 |
| --- | --- |
| [共通規約](general.md) | すべての source、設定、test、生成物 |
| [Rust 規約](rust.md) | Rust crate と Rust code |
| [ドキュメント規約](documentation.md) | Markdown、ADR、code に付随する説明 |

## 適用方法

1. 変更する file に対応する規約を確認します。
2. formatter、lint、test を local で実行します。
3. 例外が必要な場合は、理由と適用範囲を code comment または PR に記録します。恒久的または repository 全体に及ぶ例外は ADR で決定します。
4. 自動生成 file は直接直さず、原典または generator を修正して再生成します。SQL migration の手修正は ADR 0002 の例外に従います。

規約は自動化できるものから CI に移し、人による review だけに依存させません。
