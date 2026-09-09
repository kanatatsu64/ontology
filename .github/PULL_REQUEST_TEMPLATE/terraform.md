## Terraform change

### Summary

- Describe the infrastructure change and the expected plan.

### Setup

- [ ] Complete the [production Terraform bootstrap runbook](../../docs/operations/terraform-production-bootstrap.md).
- [ ] Confirm that all required GitHub Variables and protection rules remain configured.

### First deployment only

- [ ] Import the bootstrapped Artifact Registry repository and Secret Manager secret.
- [ ] Confirm the plan does not try to recreate either imported resource.
- [ ] Keep secret payloads outside Terraform, `.tfvars`, GitHub Variables, and this PR.

### Verification

- [ ] `terraform fmt -check -recursive`
- [ ] `terraform validate`
- [ ] Review the complete `terraform plan` artifact.
- [ ] Confirm destructive changes are intentional and separately approved.
