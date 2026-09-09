resource "google_cloud_run_v2_service" "api" {
  name                = "${local.name}-api"
  location            = var.region
  deletion_protection = true
  ingress             = "INGRESS_TRAFFIC_ALL"
  labels              = local.labels

  template {
    service_account = google_service_account.runtime.email

    containers {
      image = var.api_image

      env {
        name  = "DATABASE_HOST"
        value = google_sql_database_instance.application.private_ip_address
      }
      env {
        name  = "DATABASE_NAME"
        value = google_sql_database.application.name
      }
      env {
        name  = "DATABASE_USER"
        value = google_sql_user.runtime.name
      }
      env {
        name = "APPLICATION_SECRET"
        value_source {
          secret_key_ref {
            secret  = google_secret_manager_secret.application.secret_id
            version = var.application_secret_version
          }
        }
      }
    }

    vpc_access {
      network_interfaces {
        network    = google_compute_network.application.name
        subnetwork = google_compute_subnetwork.application.name
      }
      egress = "PRIVATE_RANGES_ONLY"
    }
  }

  depends_on = [
    google_project_service.required,
    google_secret_manager_secret_iam_member.runtime,
  ]
}

resource "google_cloud_run_v2_job" "migration" {
  name                = "${local.name}-migration"
  location            = var.region
  deletion_protection = true
  labels              = local.labels

  template {
    template {
      service_account = google_service_account.runtime.email
      max_retries     = 1

      containers {
        image   = var.api_image
        command = ["/app/migration-runner"]

        env {
          name  = "DATABASE_HOST"
          value = google_sql_database_instance.application.private_ip_address
        }
        env {
          name  = "DATABASE_NAME"
          value = google_sql_database.application.name
        }
        env {
          name  = "DATABASE_USER"
          value = google_sql_user.runtime.name
        }
        env {
          name = "APPLICATION_SECRET"
          value_source {
            secret_key_ref {
              secret  = google_secret_manager_secret.application.secret_id
              version = var.application_secret_version
            }
          }
        }
      }

      vpc_access {
        network_interfaces {
          network    = google_compute_network.application.name
          subnetwork = google_compute_subnetwork.application.name
        }
        egress = "PRIVATE_RANGES_ONLY"
      }
    }
  }

  depends_on = [
    google_project_service.required,
    google_secret_manager_secret_iam_member.runtime,
  ]
}
