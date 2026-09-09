variable "project_id" {
  description = "Google Cloud project ID."
  type        = string
}

variable "region" {
  description = "Region in which regional resources are created."
  type        = string
  default     = "asia-northeast1"
}

variable "api_image" {
  description = "Immutable API OCI image reference pinned by a SHA-256 digest."
  type        = string

  validation {
    condition     = can(regex("^[^[:space:]@]+@sha256:[0-9a-fA-F]{64}$", var.api_image))
    error_message = "api_image must be an OCI image reference ending in @sha256 followed by 64 hexadecimal characters."
  }
}

variable "application_secret_version" {
  description = "Numeric Secret Manager version exposed to the Cloud Run revisions."
  type        = string

  validation {
    condition     = can(regex("^[1-9][0-9]*$", var.application_secret_version))
    error_message = "application_secret_version must be a positive numeric Secret Manager version."
  }
}

variable "database_tier" {
  description = "Cloud SQL machine tier."
  type        = string
  default     = "db-custom-1-3840"
}

variable "database_deletion_protection" {
  description = "Prevent accidental deletion of the production Cloud SQL instance."
  type        = bool
  default     = true
}
