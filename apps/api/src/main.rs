mod config;

use std::{error::Error, net::SocketAddr};

use axum::{
    BoxError, Router, error_handling::HandleErrorLayer, extract::DefaultBodyLimit, http::StatusCode,
};
use config::Config;
use tokio::{net::TcpListener, signal, sync::oneshot, time};
use tower::{ServiceBuilder, limit::ConcurrencyLimitLayer, timeout::TimeoutLayer};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    init_logging();
    if let Err(error) = run().await {
        error!(%error, "API server stopped");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn Error>> {
    let config = Config::from_environment()?;
    let address = SocketAddr::new(config.bind_address, config.port);
    let listener = TcpListener::bind(address).await?;
    let app = empty_router(&config);

    info!(%address, "API server listening");
    let (shutdown_sender, shutdown_receiver) = oneshot::channel();
    let server = axum::serve(listener, app).with_graceful_shutdown(async {
        let _ = shutdown_receiver.await;
    });
    tokio::pin!(server);

    tokio::select! {
      result = &mut server => result?,
      () = shutdown_signal() => {
        info!(timeout_seconds = config.shutdown_timeout.as_secs(), "graceful shutdown started");
        let _ = shutdown_sender.send(());
        match time::timeout(config.shutdown_timeout, &mut server).await {
          Ok(result) => result?,
          Err(_) => error!("graceful shutdown deadline exceeded; terminating remaining connections"),
        }
      }
    }

    Ok(())
}

fn empty_router(config: &Config) -> Router {
    Router::new().layer(
        ServiceBuilder::new()
            .layer(HandleErrorLayer::new(|_: BoxError| async {
                StatusCode::REQUEST_TIMEOUT
            }))
            .layer(TimeoutLayer::new(config.request_timeout))
            .layer(ConcurrencyLimitLayer::new(config.max_concurrent_requests))
            .layer(DefaultBodyLimit::max(config.max_request_body_bytes)),
    )
}

fn init_logging() {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();
}

async fn shutdown_signal() {
    let interrupt = async {
        if let Err(error) = signal::ctrl_c().await {
            error!(%error, "failed to install interrupt signal handler");
            std::future::pending::<()>().await;
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match signal::unix::signal(signal::unix::SignalKind::terminate()) {
            Ok(mut stream) => {
                stream.recv().await;
            }
            Err(error) => {
                error!(%error, "failed to install termination signal handler");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
      () = interrupt => {},
      () = terminate => {},
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    #[tokio::test]
    async fn router_without_routes_returns_not_found() {
        let config = Config::default();
        let request = Request::builder()
            .uri("/not-implemented")
            .body(Body::empty())
            .expect("test request must be valid");

        let response = empty_router(&config)
            .oneshot(request)
            .await
            .expect("router must produce a response");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
