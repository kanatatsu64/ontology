# Production infrastructure

ADR 0012 の Cloud Run、Cloud Run Job、Artifact Registry、private IP の Cloud SQL for PostgreSQL、Secret Manager を構築する Terraform root module です。

state bucket と GitHub Workload Identity Federation は、この root module を実行する前に bootstrap してください。手順と GitHub Variables は pull request の説明に記載します。

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
  -var="api_image=${API_IMAGE}"
```

`api_image` は `@sha256:` を含む immutable digest で指定します。application secret の payload は Terraform state に残さないため、この module は secret container だけを作ります。必要な version は `gcloud secrets versions add` などの別経路で登録してください。
