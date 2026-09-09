variable "project_id" {
  description = "Google Cloud project ID."
  type        = string
}

variable "region" {
  description = "Region in which regional resources are created."
  type        = string
  default     = "asia-northeast1"
}

variable "environment" {
  description = "Environment suffix used in resource names and labels."
  type        = string
  default     = "production"
}

variable "api_image" {
  description = "Immutable API OCI image reference, preferably pinned by digest."
  type        = string

  validation {
    condition     = strcontains(var.api_image, "@sha256:")
    error_message = "api_image must be pinned to an immutable sha256 digest."
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
