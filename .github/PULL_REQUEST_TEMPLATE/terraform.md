## Terraform change

### Summary

- Describe the infrastructure change and the expected plan.

### GCP and GitHub setup

- [ ] Enable Artifact Registry, Compute Engine, IAM, IAM Credentials, Cloud Run,
      Secret Manager, Service Networking, Cloud SQL Admin, Service Usage, and
      Security Token Service APIs.
- [ ] Create a dedicated, versioned GCS state bucket with uniform bucket-level
      access and public access prevention.
- [ ] Create a plan service account with project Viewer, Service Usage Consumer,
      and state-bucket Object Viewer roles.
- [ ] Create an apply service account with state-bucket Object Admin and only the
      resource-management roles required by this Terraform root module.
- [ ] Configure GitHub Workload Identity Federation. Restrict the provider to
      this repository, the plan identity to pull requests, and the apply identity
      to `refs/heads/main`.
- [ ] Add the repository variables listed below.
- [ ] Create the protected `production` GitHub Environment, restrict deployments
      to `main`, and configure required reviewers.
- [ ] Protect `main` and require pull-request review and the Terraform plan check.

Repository variables:

| Variable | Value |
| --- | --- |
| `GCP_PROJECT_ID` | Google Cloud project ID |
| `GCP_REGION` | Production region, for example `asia-northeast1` |
| `TF_STATE_BUCKET` | GCS state bucket name |
| `GCP_WORKLOAD_IDENTITY_PROVIDER` | Full Workload Identity provider name |
| `GCP_TERRAFORM_PLAN_SERVICE_ACCOUNT` | Read-only plan service account email |
| `GCP_TERRAFORM_SERVICE_ACCOUNT` | Apply service account email |
| `API_IMAGE` | Existing OCI image pinned to a 64-digit SHA-256 digest |
| `APPLICATION_SECRET_VERSION` | Positive numeric Secret Manager version |

### First deployment only

- [ ] Create the `ontology` Artifact Registry repository, push the initial image,
      and import the repository as
      `google_artifact_registry_repository.applications` before the first plan.
- [ ] Create the `ontology-production-application` secret and its first payload,
      then import the secret as `google_secret_manager_secret.application`.
- [ ] Keep the secret payload outside Terraform, tfvars, GitHub variables, and
      the pull-request body.
- [ ] Confirm the PR plan does not try to recreate either imported resource.

Artifact Registry import ID:

```text
projects/PROJECT_ID/locations/REGION/repositories/ontology
```

### Verification

- [ ] `terraform fmt -check -recursive`
- [ ] `terraform validate`
- [ ] Review the complete `terraform plan` artifact.
- [ ] Confirm destructive changes are intentional and separately approved.
