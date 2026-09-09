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
    }

    vpc_access {
      network_interfaces {
        network    = google_compute_network.application.name
        subnetwork = google_compute_subnetwork.application.name
      }
      egress = "PRIVATE_RANGES_ONLY"
    }
  }

  depends_on = [google_project_service.required]
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

  depends_on = [google_project_service.required]
}
