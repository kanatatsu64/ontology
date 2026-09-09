# 0017: Google Cloud infrastructure の Terraform 管理と delivery

- 日付: 2026-09-09
- 意思決定者: プロジェクト
- 関連 ADR: 0012, 0016

## コンテキスト

ADR 0012 で決定した Google Cloud の実行基盤を、再現可能かつ review 可能に構築し、安全に継続変更する方法が必要です。

## 用語

| 用語 | 定義 |
| --- | --- |
| Terraform | 宣言した構成と provider API を使い infrastructure を管理する tool。 |
| plan | 現在の state と構成を比較し、予定する変更を表示する処理。 |
| apply | plan に基づいて実 resource を変更する処理。 |
| remote state | 複数の実行主体が共有する、cloud storage 上の Terraform state。 |
| Workload Identity Federation | 長期 service account key を保存せず、GitHub の OIDC token を Google Cloud の短期 credential と交換する仕組み。 |

## 決定

- Google Cloud resource を Terraform で管理し、production は `infra/environments/production/` を root module とします。
- state は versioning を有効にした専用 Google Cloud Storage bucket に保存します。state bucket と GitHub OIDC の初期設定は循環依存を避けるため bootstrap 手順で作成します。
- pull request で `terraform fmt -check`、`terraform validate`、`terraform plan` を実行し、plan を workflow summary と artifact に残します。plan と apply は production state 単位で直列化します。plan 専用 service account に state lock の書き込み権限を与えないため、PR plan だけは lock を作成せずに実行します。
- `main` への merge で最新 commit から改めて plan を作成し、その保存済み plan だけを自動 apply します。GitHub Environment の保護規則を apply の承認境界にできます。
- GitHub Actions は Workload Identity Federation で短期 credential を取得し、長期 service account key を repository に保存しません。PR の plan は resource と state の読み取りだけを許可する plan 専用 service account、main の apply は resource を変更できる apply 専用 service account に分離します。
- provider と Terraform の version constraint を commit します。secret payload、credential、変数ファイル、local state、plan は commit しません。
- production の Cloud Run service/job と Cloud SQL に deletion protection を設定し、削除時は保護を無効化する先行変更の review を必要とします。

## 検討

### D-1: infrastructure as code tool

#### 判断基準

- Google Cloud resource と依存関係を宣言的に review できること
- local と CI で同じ plan を生成できること
- state と provider version を明示的に管理できること

#### 選択肢

- **Terraform:** state 運用が必要だが、Google provider の resource を宣言的に管理できる。
- **gcloud script:** state は不要だが、差分と削除を安全に review しにくい。
- **手動構築:** 初期操作は少ないが、再現性と監査可能性がない。

#### 採用

**Terraform**を採用します。構成差分を PR の plan で確認でき、同じ root module を local と CI の双方で利用できます。

### D-2: CI/CD の認証と適用契機

#### 判断基準

- 長期 credential を GitHub に保存しないこと
- review 前に変更内容を確認できること
- review 済みの main だけが production を変更できること

#### 選択肢

- **OIDC、PR plan、main apply:** bootstrap は必要だが、短期 credential と branch protection を利用できる。
- **service account key:** 設定は容易だが、長期 secret の保管と rotation が必要になる。
- **手動 apply:** 操作者を限定できるが、適用漏れと local 環境差が生じる。

#### 採用

**OIDC、PR plan、main apply**を採用します。PR の code が apply 権限を取得しないよう service account を分離します。GitHub Environment の承認を追加できる構成とし、apply 時には main の構成から plan を再作成して実行します。

## 影響

state bucket、OIDC trust、Terraform service account は事前に bootstrap する必要があります。read-only plan は state lock を取得できないため、CI の concurrency 制御外で apply しません。production apply は main の変更で自動実行されるため、branch protection、CODEOWNERS、GitHub Environment の reviewer と IAM 権限を運用上の防御層にします。
