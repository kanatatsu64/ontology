use std::{
    env,
    error::Error,
    fmt,
    net::{IpAddr, Ipv4Addr},
    time::Duration,
};

const DEFAULT_BIND_ADDRESS: &str = "0.0.0.0";
const DEFAULT_PORT: u16 = 8080;
const DEFAULT_REQUEST_TIMEOUT_SECONDS: u64 = 30;
const DEFAULT_MAX_CONCURRENT_REQUESTS: usize = 256;
const DEFAULT_MAX_REQUEST_BODY_BYTES: usize = 1_048_576;
const DEFAULT_SHUTDOWN_TIMEOUT_SECONDS: u64 = 30;
const MAX_REQUEST_TIMEOUT_SECONDS: u64 = 3_600;
const MAX_CONCURRENT_REQUESTS_LIMIT: usize = 10_000;
const MAX_REQUEST_BODY_BYTES_LIMIT: usize = 16 * 1_048_576;
const MAX_SHUTDOWN_TIMEOUT_SECONDS: u64 = 300;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Config {
    pub(crate) bind_address: IpAddr,
    pub(crate) port: u16,
    pub(crate) request_timeout: Duration,
    pub(crate) max_concurrent_requests: usize,
    pub(crate) max_request_body_bytes: usize,
    pub(crate) shutdown_timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind_address: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            port: DEFAULT_PORT,
            request_timeout: Duration::from_secs(DEFAULT_REQUEST_TIMEOUT_SECONDS),
            max_concurrent_requests: DEFAULT_MAX_CONCURRENT_REQUESTS,
            max_request_body_bytes: DEFAULT_MAX_REQUEST_BODY_BYTES,
            shutdown_timeout: Duration::from_secs(DEFAULT_SHUTDOWN_TIMEOUT_SECONDS),
        }
    }
}

impl Config {
    pub(crate) fn from_environment() -> Result<Self, ConfigError> {
        Self::from_reader(|name| match env::var(name) {
            Ok(value) => Ok(Some(value)),
            Err(env::VarError::NotPresent) => Ok(None),
            Err(env::VarError::NotUnicode(_)) => Err(ConfigError::Invalid { name }),
        })
    }

    fn from_reader(
        read: impl Fn(&'static str) -> Result<Option<String>, ConfigError>,
    ) -> Result<Self, ConfigError> {
        let bind_address = parse_or_default(&read, "BIND_ADDRESS", DEFAULT_BIND_ADDRESS)?;
        let port = positive("PORT", parse_or_default(&read, "PORT", DEFAULT_PORT)?)?;
        let request_timeout_seconds = bounded(
            "REQUEST_TIMEOUT_SECONDS",
            parse_or_default(
                &read,
                "REQUEST_TIMEOUT_SECONDS",
                DEFAULT_REQUEST_TIMEOUT_SECONDS,
            )?,
            MAX_REQUEST_TIMEOUT_SECONDS,
        )?;
        let max_concurrent_requests = bounded(
            "MAX_CONCURRENT_REQUESTS",
            parse_or_default(
                &read,
                "MAX_CONCURRENT_REQUESTS",
                DEFAULT_MAX_CONCURRENT_REQUESTS,
            )?,
            MAX_CONCURRENT_REQUESTS_LIMIT,
        )?;
        let max_request_body_bytes = bounded(
            "MAX_REQUEST_BODY_BYTES",
            parse_or_default(
                &read,
                "MAX_REQUEST_BODY_BYTES",
                DEFAULT_MAX_REQUEST_BODY_BYTES,
            )?,
            MAX_REQUEST_BODY_BYTES_LIMIT,
        )?;
        let shutdown_timeout_seconds = bounded(
            "SHUTDOWN_TIMEOUT_SECONDS",
            parse_or_default(
                &read,
                "SHUTDOWN_TIMEOUT_SECONDS",
                DEFAULT_SHUTDOWN_TIMEOUT_SECONDS,
            )?,
            MAX_SHUTDOWN_TIMEOUT_SECONDS,
        )?;

        Ok(Self {
            bind_address,
            port,
            request_timeout: Duration::from_secs(request_timeout_seconds),
            max_concurrent_requests,
            max_request_body_bytes,
            shutdown_timeout: Duration::from_secs(shutdown_timeout_seconds),
        })
    }
}

fn parse_or_default<T>(
    read: &impl Fn(&'static str) -> Result<Option<String>, ConfigError>,
    name: &'static str,
    default: T,
) -> Result<T, ConfigError>
where
    T: std::str::FromStr,
{
    match read(name)? {
        Some(value) => value.parse().map_err(|_| ConfigError::Invalid { name }),
        None => Ok(default),
    }
}

fn positive<T>(name: &'static str, value: T) -> Result<T, ConfigError>
where
    T: Default + PartialEq,
{
    if value == T::default() {
        Err(ConfigError::Zero { name })
    } else {
        Ok(value)
    }
}

fn bounded<T>(name: &'static str, value: T, maximum: T) -> Result<T, ConfigError>
where
    T: Copy + Default + fmt::Display + PartialEq + PartialOrd,
{
    let value = positive(name, value)?;
    if value > maximum {
        Err(ConfigError::TooLarge {
            name,
            maximum: maximum.to_string(),
        })
    } else {
        Ok(value)
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum ConfigError {
    Invalid { name: &'static str },
    TooLarge { name: &'static str, maximum: String },
    Zero { name: &'static str },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid { name } => write!(
                formatter,
                "environment variable {name} has an invalid value"
            ),
            Self::TooLarge { name, maximum } => write!(
                formatter,
                "environment variable {name} must not exceed {maximum}"
            ),
            Self::Zero { name } => write!(
                formatter,
                "environment variable {name} must be greater than zero"
            ),
        }
    }
}

impl Error for ConfigError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn defaults_are_safe_and_cloud_run_compatible() {
        let config =
            Config::from_reader(|_| Ok(None)).expect("defaults must form valid configuration");

        assert_eq!(config.bind_address.to_string(), "0.0.0.0");
        assert_eq!(config.port, 8080);
        assert_eq!(config.request_timeout, Duration::from_secs(30));
        assert_eq!(config.max_concurrent_requests, 256);
        assert_eq!(config.max_request_body_bytes, 1_048_576);
        assert_eq!(config.shutdown_timeout, Duration::from_secs(30));
    }

    #[test]
    fn environment_overrides_every_setting() {
        let values = HashMap::from([
            ("BIND_ADDRESS", "127.0.0.1"),
            ("PORT", "9090"),
            ("REQUEST_TIMEOUT_SECONDS", "5"),
            ("MAX_CONCURRENT_REQUESTS", "10"),
            ("MAX_REQUEST_BODY_BYTES", "2048"),
            ("SHUTDOWN_TIMEOUT_SECONDS", "8"),
        ]);

        let config = Config::from_reader(|name| Ok(values.get(name).map(ToString::to_string)))
            .expect("test values must form valid configuration");

        assert_eq!(config.bind_address.to_string(), "127.0.0.1");
        assert_eq!(config.port, 9090);
        assert_eq!(config.request_timeout, Duration::from_secs(5));
        assert_eq!(config.max_concurrent_requests, 10);
        assert_eq!(config.max_request_body_bytes, 2048);
        assert_eq!(config.shutdown_timeout, Duration::from_secs(8));
    }

    #[test]
    fn zero_resource_limit_is_rejected() {
        let result = Config::from_reader(|name| {
            Ok((name == "MAX_CONCURRENT_REQUESTS").then(|| "0".to_owned()))
        });

        assert_eq!(
            result,
            Err(ConfigError::Zero {
                name: "MAX_CONCURRENT_REQUESTS"
            })
        );
    }

    #[test]
    fn excessive_concurrency_is_rejected_before_middleware_construction() {
        let result = Config::from_reader(|name| {
            Ok((name == "MAX_CONCURRENT_REQUESTS").then(|| "10001".to_owned()))
        });

        assert_eq!(
            result,
            Err(ConfigError::TooLarge {
                name: "MAX_CONCURRENT_REQUESTS",
                maximum: "10000".to_owned(),
            })
        );
    }

    #[test]
    fn maximum_resource_limits_are_accepted() {
        let values = HashMap::from([
            ("REQUEST_TIMEOUT_SECONDS", "3600"),
            ("MAX_CONCURRENT_REQUESTS", "10000"),
            ("MAX_REQUEST_BODY_BYTES", "16777216"),
            ("SHUTDOWN_TIMEOUT_SECONDS", "300"),
        ]);

        let config = Config::from_reader(|name| Ok(values.get(name).map(ToString::to_string)))
            .expect("maximum values must remain in the valid range");

        assert_eq!(config.request_timeout, Duration::from_secs(3_600));
        assert_eq!(config.max_concurrent_requests, 10_000);
        assert_eq!(config.max_request_body_bytes, 16_777_216);
        assert_eq!(config.shutdown_timeout, Duration::from_secs(300));
    }

    #[test]
    fn zero_port_is_rejected_instead_of_selecting_an_ephemeral_port() {
        let result = Config::from_reader(|name| Ok((name == "PORT").then(|| "0".to_owned())));

        assert_eq!(result, Err(ConfigError::Zero { name: "PORT" }));
    }

    #[test]
    fn invalid_value_names_variable_without_echoing_value() {
        let result =
            Config::from_reader(|name| Ok((name == "PORT").then(|| "secret-value".to_owned())));

        assert_eq!(result, Err(ConfigError::Invalid { name: "PORT" }));
        assert!(
            !result
                .expect_err("value is invalid")
                .to_string()
                .contains("secret-value")
        );
    }

    #[test]
    fn non_unicode_environment_value_is_rejected_as_invalid() {
        let result = Config::from_reader(|name| {
            if name == "BIND_ADDRESS" {
                Err(ConfigError::Invalid { name })
            } else {
                Ok(None)
            }
        });

        assert_eq!(
            result,
            Err(ConfigError::Invalid {
                name: "BIND_ADDRESS"
            })
        );
    }
}
