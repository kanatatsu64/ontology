resource "google_sql_database_instance" "application" {
  name                = local.name
  region              = var.region
  database_version    = "POSTGRES_16"
  deletion_protection = var.database_deletion_protection

  settings {
    tier              = var.database_tier
    availability_type = "REGIONAL"
    disk_type         = "PD_SSD"
    disk_autoresize   = true
    user_labels       = local.labels

    backup_configuration {
      enabled                        = true
      point_in_time_recovery_enabled = true
      start_time                     = "18:00"
    }

    ip_configuration {
      ipv4_enabled                                  = false
      private_network                               = google_compute_network.application.id
      enable_private_path_for_google_cloud_services = true
    }

    database_flags {
      name  = "cloudsql.iam_authentication"
      value = "on"
    }
  }

  depends_on = [google_service_networking_connection.private_services]
}

resource "google_sql_database" "application" {
  name     = "ontology"
  instance = google_sql_database_instance.application.name
}

resource "google_sql_user" "runtime" {
  name     = trimsuffix(google_service_account.runtime.email, ".gserviceaccount.com")
  instance = google_sql_database_instance.application.name
  type     = "CLOUD_IAM_SERVICE_ACCOUNT"
}
