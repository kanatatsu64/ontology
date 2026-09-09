# Production Terraform bootstrap

この手順書は、repository 管理者と Google Cloud 管理者が production の
Terraform workflow を初めて有効にするために使用します。placeholder を実際の
値へ置き換え、すべての確認項目を完了してから Terraform の PR を merge して
ください。

## 1. Shell 変数

以下は実在しない例です。

```bash
export PROJECT_ID="example-ontology-production"
export REGION="asia-northeast1"
export STATE_BUCKET="example-ontology-production-tfstate"
export GITHUB_REPOSITORY="example/ontology"
export POOL_ID="github-actions"
export PROVIDER_ID="ontology"
export PLAN_SA_NAME="terraform-plan"
export APPLY_SA_NAME="terraform-apply"
export PLAN_SA="${PLAN_SA_NAME}@${PROJECT_ID}.iam.gserviceaccount.com"
export APPLY_SA="${APPLY_SA_NAME}@${PROJECT_ID}.iam.gserviceaccount.com"
export PROJECT_NUMBER="$(gcloud projects describe "${PROJECT_ID}" --format='value(projectNumber)')"
gcloud config set project "${PROJECT_ID}"
```

## 2. Bootstrap API

```bash
gcloud services enable \
  artifactregistry.googleapis.com \
  cloudresourcemanager.googleapis.com \
  compute.googleapis.com \
  iam.googleapis.com \
  iamcredentials.googleapis.com \
  run.googleapis.com \
  secretmanager.googleapis.com \
  serviceusage.googleapis.com \
  servicenetworking.googleapis.com \
  sqladmin.googleapis.com \
  sts.googleapis.com
```

## 3. Remote state

```bash
gcloud storage buckets create "gs://${STATE_BUCKET}" \
  --project="${PROJECT_ID}" \
  --location="${REGION}" \
  --uniform-bucket-level-access \
  --public-access-prevention
gcloud storage buckets update "gs://${STATE_BUCKET}" --versioning
```

組織の復旧要件に従って retention policy と lifecycle rule も設定します。state
bucket を Terraform の同じ state では管理しません。

## 4. Workflow service account

```bash
gcloud iam service-accounts create "${PLAN_SA_NAME}" \
  --display-name="Terraform production plan"
gcloud iam service-accounts create "${APPLY_SA_NAME}" \
  --display-name="Terraform production apply"

gcloud projects add-iam-policy-binding "${PROJECT_ID}" \
  --member="serviceAccount:${PLAN_SA}" \
  --role="roles/viewer"
gcloud projects add-iam-policy-binding "${PROJECT_ID}" \
  --member="serviceAccount:${PLAN_SA}" \
  --role="roles/serviceusage.serviceUsageConsumer"
gcloud storage buckets add-iam-policy-binding "gs://${STATE_BUCKET}" \
  --member="serviceAccount:${PLAN_SA}" \
  --role="roles/storage.objectViewer"
```

Apply service account には state 更新権限と、この root module が作成する resource
の管理権限を付与します。利用できる場合は、次の predefined role をそのまま使わず
必要な permission だけを持つ organization custom role を作成します。

```bash
gcloud storage buckets add-iam-policy-binding "gs://${STATE_BUCKET}" \
  --member="serviceAccount:${APPLY_SA}" \
  --role="roles/storage.objectAdmin"

for role in \
  roles/artifactregistry.admin \
  roles/cloudsql.admin \
  roles/compute.networkAdmin \
  roles/iam.serviceAccountAdmin \
  roles/iam.serviceAccountUser \
  roles/resourcemanager.projectIamAdmin \
  roles/run.admin \
  roles/secretmanager.admin \
  roles/servicenetworking.networksAdmin \
  roles/serviceusage.serviceUsageAdmin
do
  gcloud projects add-iam-policy-binding "${PROJECT_ID}" \
    --member="serviceAccount:${APPLY_SA}" \
    --role="${role}"
done
```

## 5. GitHub Workload Identity Federation

OIDC claim から `terraform_role` を導出します。`main` だけが `apply`、それ以外は
`plan` となり、service account の impersonation binding でもこの属性を検査します。
provider condition は対象 repository の PR と `main` 以外を拒否します。

```bash
gcloud iam workload-identity-pools create "${POOL_ID}" \
  --location="global" \
  --display-name="GitHub Actions"

gcloud iam workload-identity-pools providers create-oidc "${PROVIDER_ID}" \
  --location="global" \
  --workload-identity-pool="${POOL_ID}" \
  --issuer-uri="https://token.actions.githubusercontent.com" \
  --attribute-mapping="google.subject=assertion.sub,attribute.repository=assertion.repository,attribute.terraform_role=assertion.ref=='refs/heads/main'?'apply':'plan'" \
  --attribute-condition="assertion.repository=='${GITHUB_REPOSITORY}' && (assertion.event_name=='pull_request' || assertion.ref=='refs/heads/main')"

export PLAN_PRINCIPAL="principalSet://iam.googleapis.com/projects/${PROJECT_NUMBER}/locations/global/workloadIdentityPools/${POOL_ID}/attribute.terraform_role/plan"
export APPLY_PRINCIPAL="principalSet://iam.googleapis.com/projects/${PROJECT_NUMBER}/locations/global/workloadIdentityPools/${POOL_ID}/attribute.terraform_role/apply"

gcloud iam service-accounts add-iam-policy-binding "${PLAN_SA}" \
  --member="${PLAN_PRINCIPAL}" \
  --role="roles/iam.workloadIdentityUser"
gcloud iam service-accounts add-iam-policy-binding "${APPLY_SA}" \
  --member="${APPLY_PRINCIPAL}" \
  --role="roles/iam.workloadIdentityUser"
```

## 6. First Artifact Registry image

Cloud Run が参照する image は最初の apply より前に必要です。repository と image
を作成し、repository を Terraform state へ import します。

```bash
gcloud artifacts repositories create ontology \
  --location="${REGION}" \
  --repository-format="docker"
gcloud auth configure-docker "${REGION}-docker.pkg.dev"

# Application の build 手順で image を push した後、その digest を指定します。
export API_IMAGE="${REGION}-docker.pkg.dev/${PROJECT_ID}/ontology/api@sha256:REPLACE_WITH_64_HEX_DIGEST"
```

## 7. First application secret

Payload を command line argument、Terraform、`.tfvars`、GitHub Variables、または
PR に書きません。標準入力で最初の version を追加します。

```bash
gcloud secrets create ontology-production-application \
  --replication-policy="automatic"
read -rsp "Application secret: " APPLICATION_SECRET && printf '\n'
printf '%s' "${APPLICATION_SECRET}" | \
  gcloud secrets versions add ontology-production-application --data-file=-
unset APPLICATION_SECRET

export APPLICATION_SECRET_VERSION="$(
  gcloud secrets versions list ontology-production-application \
    --filter='state=ENABLED' \
    --sort-by='~createTime' \
    --limit=1 \
    --format='value(name.basename())'
)"
```

## 8. Bootstrap import

Import 操作者には state bucket と import 対象 resource への権限が必要です。
Terraform provider は Application Default Credentials を使うため、組織の
認証手順に従って操作者または bootstrap 用 service account で認証します。
service account key file は作成しません。

```bash
gcloud auth application-default login

terraform -chdir=infra/environments/production init \
  -backend-config="bucket=${STATE_BUCKET}"

terraform -chdir=infra/environments/production import \
  -var="project_id=${PROJECT_ID}" \
  -var="region=${REGION}" \
  -var="api_image=${API_IMAGE}" \
  -var="application_secret_version=${APPLICATION_SECRET_VERSION}" \
  google_artifact_registry_repository.applications \
  "projects/${PROJECT_ID}/locations/${REGION}/repositories/ontology"

terraform -chdir=infra/environments/production import \
  -var="project_id=${PROJECT_ID}" \
  -var="region=${REGION}" \
  -var="api_image=${API_IMAGE}" \
  -var="application_secret_version=${APPLICATION_SECRET_VERSION}" \
  google_secret_manager_secret.application \
  "projects/${PROJECT_ID}/secrets/ontology-production-application"
```

## 9. GitHub settings

`Settings` → `Secrets and variables` → `Actions` → `Variables` に登録します。

| Variable | Value |
| --- | --- |
| `GCP_PROJECT_ID` | `${PROJECT_ID}` |
| `GCP_REGION` | `${REGION}` |
| `TF_STATE_BUCKET` | `${STATE_BUCKET}` |
| `GCP_WORKLOAD_IDENTITY_PROVIDER` | `projects/PROJECT_NUMBER/locations/global/workloadIdentityPools/POOL_ID/providers/PROVIDER_ID` |
| `GCP_TERRAFORM_PLAN_SERVICE_ACCOUNT` | `${PLAN_SA}` |
| `GCP_TERRAFORM_SERVICE_ACCOUNT` | `${APPLY_SA}` |
| `API_IMAGE` | `${API_IMAGE}` |
| `APPLICATION_SECRET_VERSION` | `${APPLICATION_SECRET_VERSION}` |

`production` GitHub Environment を作り、required reviewer と `main` だけを許可する
deployment branch rule を設定します。`main` の branch protection では PR review と
Terraform plan check を必須にします。

## 10. Verification

PR を作成し、次を確認します。

1. `terraform fmt` と `terraform validate` が成功する。
2. Plan が import 済み repository と secret の再作成を含まない。
3. `API_IMAGE` が 64 桁 SHA-256 digest に固定されている。
4. `APPLICATION_SECRET_VERSION` が意図した数値 version である。
5. Plan artifact に意図しない削除や置換がない。

Secret を rotate するときは新しい payload version を別経路で追加し、payload では
なく数値 version だけを `APPLICATION_SECRET_VERSION` に設定して PR を作成します。
