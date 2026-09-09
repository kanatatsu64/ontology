# Production infrastructure

ADR 0012 の Cloud Run、Cloud Run Job、Artifact Registry、private IP の Cloud SQL for PostgreSQL、Secret Manager を構築する Terraform root module です。

state bucket と GitHub Workload Identity Federation は、この root module を実行する前に [production Terraform bootstrap 手順](../../../docs/operations/terraform-production-bootstrap.md) で構築してください。pull request では `.github/PULL_REQUEST_TEMPLATE/terraform.md` の checklist を確認します。

構成は責務ごとに分割しています。

| ファイル | 責務 |
| --- | --- |
| `variables.tf` | 入力変数 |
| `network.tf` | VPC、subnet、Private Service Access |
| `db.tf` | Cloud SQL instance、database、IAM database user |
| `cloud_run.tf` | API service、migration job |
| `iam.tf` | runtime service account と IAM binding |
| `main.tf` | 共通設定、利用 API、Artifact Registry、Secret Manager |

```bash
terraform init -backend-config="bucket=${TF_STATE_BUCKET}"
terraform plan \
  -var="project_id=${GCP_PROJECT_ID}" \
  -var="region=${GCP_REGION}" \
  -var="api_image=${API_IMAGE}" \
  -var="application_secret_version=${APPLICATION_SECRET_VERSION}"
```

`api_image` は `@sha256:` と 64 桁の 16 進数 digest を持つ immutable OCI image reference で指定します。application secret の payload は Terraform state に残さないため、この module は secret container と Cloud Run からの数値 version 参照だけを管理します。初回 apply の前に secret container と version を作成し、container を `google_secret_manager_secret.application` へ import してください。payload の更新は `gcloud secrets versions add` などの別経路で行い、新しい version 番号を `APPLICATION_SECRET_VERSION` に設定して plan/apply します。API と migration job には `APPLICATION_SECRET` として注入します。

Production の Cloud Run service/job には deletion protection を有効にします。Cloud SQL は Terraform provider 側の削除保護に加え、Google Cloud 側の deletion protection も `database_deletion_protection` で同時に制御します。意図した削除では、保護を無効化する変更を先に review ・ apply してください。
