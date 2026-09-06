# 0013: 生成物を PR に反映する自動・手動ワークフロー

- 日付: 2026-09-06
- 意思決定者: プロジェクト
- 関連 ADR: 0002, 0005, 0008, 0010

## コンテキスト

YAML の変更に伴う生成物を PR に含めつつ、自動処理が利用できない経路も保ちます。

## 用語

| 用語 | 定義 |
| --- | --- |
| 変更元 branch | PR の変更が置かれ、生成物の commit 先となる branch。 |
| fork PR | 元 repository とは別の repository から送られる PR。 |
| concurrency 制御 | 同じ branch の古い処理と新しい処理が同時に反映されないようにする仕組み。 |
| rebase | 変更の土台を最新の base branch へ載せ替える操作。 |
| workflow dispatch | GitHub Actions の workflow を利用者が明示的に開始する方法。 |

## 決定

- GitHub Actions と手動操作から同じ Rust CLI を実行し、SQL、snapshot、OpenAPI、Rust interface を変更元 branch に commit します。
- 自動 commit は同一 repository の branch だけに許可し、生成 job だけへ `contents: write` を付与します。fork PR は手動経路を使用します。
- concurrency 制御、bot commit の再実行防止、入力 SHA と toolchain version の記録を行います。
- 既存 migration と、base snapshot が変化した migration は自動更新せず、rebase と明示的な再生成を要求します。
- `check` は再生成差分、snapshot digest、OpenAPI validation、Rust compile を検査します。

## 検討

### D-1: PR への反映方法

#### 判断基準

- YAML と生成物を同じ diff で review できること
- 生成漏れを自動検出できること

#### 選択肢

- **変更元 branch へ commit:** 通常の PR diff になるが、write 権限が必要。
- **CI artifact のみ:** branch を変更しないが、通常の review 対象にならない。
- **常に手動:** 権限は単純だが、生成漏れが起きる。

#### 採用

**変更元 branch へ commit**を選びます。SQL、snapshot、OpenAPI、Rust interface を commit し、`check` で再生成差分を拒否します。

### D-2: 自動処理の権限と競合

#### 判断基準

- fork へ秘密や write 権限を渡さないこと
- 古い run が新しい生成物を上書きしないこと
- bot commit の再帰実行を防ぐこと

#### 選択肢

- **制限した branch write:** fork では自動反映できないが、権限を限定できる。
- **全 PR へ write:** 自動化範囲は広いが、安全でない。

#### 採用

**制限した branch write**を選びます。同一 repository の branch のみに `contents: write` を与え、concurrency 制御と bot commit の除外条件を設けます。

### D-3: 手動経路

#### 判断基準

- GitHub Actions が使えなくても同じ結果になること
- 自動処理専用 logic を持たないこと

#### 選択肢

- **同じ CLI と workflow_dispatch:** 入口は増えるが、生成 logic を共有できる。
- **別 script:** 個別最適化できるが、結果がずれ得る。

#### 採用

**同じ CLI と workflow_dispatch**を選びます。local からも同じ command を実行します。

### D-4: 手修正 migration と rebase

#### 判断基準

- 既存 migration の編集を失わないこと
- 古い base snapshot から生成しないこと

#### 選択肢

- **既存 file を更新せず停止:** 再操作は必要だが、編集を保護できる。
- **自動再生成:** 便利だが、編集を上書きし得る。

#### 採用

**既存 file を更新せず停止**する案を選びます。base snapshot が変わった場合も commit せず、rebase と明示再生成を要求します。

## 影響

bot の権限、loop 防止、concurrency、fork PR の手動対応が必要です。commit message には入力 SHA と toolchain version を含めます。
