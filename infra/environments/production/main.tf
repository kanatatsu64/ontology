locals {
  environment = "production"
  name        = "ontology-${local.environment}"
  labels = {
    application = "ontology"
    environment = local.environment
    managed_by  = "terraform"
  }
  required_services = toset([
    "artifactregistry.googleapis.com",
    "compute.googleapis.com",
    "iam.googleapis.com",
    "run.googleapis.com",
    "secretmanager.googleapis.com",
    "servicenetworking.googleapis.com",
    "sqladmin.googleapis.com",
  ])
}

resource "google_project_service" "required" {
  for_each = local.required_services

  project            = var.project_id
  service            = each.value
  disable_on_destroy = false
}

resource "google_artifact_registry_repository" "applications" {
  location      = var.region
  repository_id = "ontology"
  description   = "Ontology application OCI images"
  format        = "DOCKER"
  labels        = local.labels

  depends_on = [google_project_service.required]
}

resource "google_secret_manager_secret" "application" {
  secret_id = "${local.name}-application"
  labels    = local.labels

  replication {
    auto {}
  }

  depends_on = [google_project_service.required]
}
