terraform {
  required_version = "= 1.9.8"

  required_providers {
    google = {
      source  = "hashicorp/google"
      version = "= 6.0.1"
    }
  }

  backend "gcs" {
    prefix = "ontology/production"
  }
}

provider "google" {
  project = var.project_id
  region  = var.region
}
