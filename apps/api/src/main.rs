mod config;

use std::{error::Error, fmt, io, net::SocketAddr};

use axum::Router;
use config::{Config, ConfigError};
use tokio::{net::TcpListener, signal, sync::oneshot, time};
use tower::{ServiceBuilder, limit::ConcurrencyLimitLayer};
use tower_http::{limit::RequestBodyLimitLayer, timeout::TimeoutLayer};
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

async fn run() -> Result<(), ApplicationError> {
    let config = Config::from_environment().map_err(ApplicationError::Configuration)?;
    let address = SocketAddr::new(config.bind_address, config.port);
    let listener = TcpListener::bind(address)
        .await
        .map_err(|source| ApplicationError::Bind { address, source })?;
    let app = empty_router(&config);

    info!(%address, "API server listening");
    let (shutdown_sender, shutdown_receiver) = oneshot::channel();
    let server = axum::serve(listener, app).with_graceful_shutdown(async {
        let _ = shutdown_receiver.await;
    });
    tokio::pin!(server);

    tokio::select! {
      result = &mut server => result.map_err(ApplicationError::Serve)?,
      signal_result = shutdown_signal() => {
        signal_result.map_err(ApplicationError::Signal)?;
        info!(timeout_seconds = config.shutdown_timeout.as_secs(), "graceful shutdown started");
        let _ = shutdown_sender.send(());
        match time::timeout(config.shutdown_timeout, &mut server).await {
          Ok(result) => result.map_err(ApplicationError::Serve)?,
          Err(_) => return Err(ApplicationError::ShutdownDeadline {
            timeout: config.shutdown_timeout,
          }),
        }
      }
    }

    Ok(())
}

fn empty_router(config: &Config) -> Router {
    Router::new().layer(
        ServiceBuilder::new()
            .layer(TimeoutLayer::new(config.request_timeout))
            .layer(ConcurrencyLimitLayer::new(config.max_concurrent_requests))
            .layer(RequestBodyLimitLayer::new(config.max_request_body_bytes)),
    )
}

#[derive(Debug)]
enum ApplicationError {
    Configuration(ConfigError),
    Bind {
        address: SocketAddr,
        source: io::Error,
    },
    Signal(io::Error),
    ShutdownDeadline {
        timeout: time::Duration,
    },
    Serve(io::Error),
}

impl fmt::Display for ApplicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Configuration(error) => write!(formatter, "load server configuration: {error}"),
            Self::Bind { address, source } => {
                write!(formatter, "bind API listener to {address}: {source}")
            }
            Self::Signal(source) => write!(formatter, "listen for process shutdown: {source}"),
            Self::ShutdownDeadline { timeout } => write!(
                formatter,
                "finish API requests within shutdown deadline of {} seconds",
                timeout.as_secs()
            ),
            Self::Serve(source) => write!(formatter, "serve API requests: {source}"),
        }
    }
}

impl Error for ApplicationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Configuration(error) => Some(error),
            Self::Bind { source, .. } | Self::Signal(source) | Self::Serve(source) => Some(source),
            Self::ShutdownDeadline { .. } => None,
        }
    }
}

fn init_logging() {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();
}

async fn shutdown_signal() -> io::Result<()> {
    let interrupt = signal::ctrl_c();

    #[cfg(unix)]
    let terminate = async {
        let mut stream = signal::unix::signal(signal::unix::SignalKind::terminate())?;
        if stream.recv().await.is_none() {
            return Err(io::Error::other("termination signal stream closed"));
        }
        Ok(())
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<io::Result<()>>();

    tokio::select! {
      result = interrupt => result,
      result = terminate => result,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode, header::CONTENT_LENGTH},
    };
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

    #[tokio::test]
    async fn router_rejects_oversized_body_before_routing() {
        let config = Config {
            max_request_body_bytes: 4,
            ..Config::default()
        };
        let request = Request::builder()
            .uri("/not-implemented")
            .header(CONTENT_LENGTH, "5")
            .body(Body::from("12345"))
            .expect("test request must be valid");

        let response = empty_router(&config)
            .oneshot(request)
            .await
            .expect("router must produce a response");

        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[test]
    fn bind_error_identifies_operation_address_and_cause() {
        let address = SocketAddr::from(([127, 0, 0, 1], 8080));
        let error = ApplicationError::Bind {
            address,
            source: io::Error::new(io::ErrorKind::AddrInUse, "test address is in use"),
        };

        assert_eq!(
            error.to_string(),
            "bind API listener to 127.0.0.1:8080: test address is in use"
        );
    }

    #[test]
    fn shutdown_deadline_error_identifies_configured_timeout() {
        let error = ApplicationError::ShutdownDeadline {
            timeout: time::Duration::from_secs(30),
        };

        assert_eq!(
            error.to_string(),
            "finish API requests within shutdown deadline of 30 seconds"
        );
    }
}
