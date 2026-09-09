resource "google_compute_network" "application" {
  name                    = local.name
  auto_create_subnetworks = false

  depends_on = [google_project_service.required]
}

resource "google_compute_subnetwork" "application" {
  name          = local.name
  region        = var.region
  network       = google_compute_network.application.id
  ip_cidr_range = "10.10.0.0/24"
}

resource "google_compute_global_address" "private_services" {
  name          = "${local.name}-private-services"
  purpose       = "VPC_PEERING"
  address_type  = "INTERNAL"
  prefix_length = 16
  network       = google_compute_network.application.id
}

resource "google_service_networking_connection" "private_services" {
  network                 = google_compute_network.application.id
  service                 = "servicenetworking.googleapis.com"
  reserved_peering_ranges = [google_compute_global_address.private_services.name]
}
