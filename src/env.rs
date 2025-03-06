use crate::{db, expiration, highlight};
use axum_extra::extract::cookie::Key;
use std::env::VarError;
use std::net::SocketAddr;
use std::num::{NonZeroUsize, ParseIntError};
use std::path::PathBuf;
use std::time::Duration;

pub const DEFAULT_HTTP_TIMEOUT: Duration = Duration::from_secs(5);

const VAR_ADDRESS_PORT: &str = "WASTEBIN_ADDRESS_PORT";
const VAR_BASE_URL: &str = "WASTEBIN_BASE_URL";
const VAR_CACHE_SIZE: &str = "WASTEBIN_CACHE_SIZE";
const VAR_DATABASE_PATH: &str = "WASTEBIN_DATABASE_PATH";
const VAR_HTTP_TIMEOUT: &str = "WASTEBIN_HTTP_TIMEOUT";
const VAR_MAX_BODY_SIZE: &str = "WASTEBIN_MAX_BODY_SIZE";
const VAR_PASTE_EXPIRATIONS: &str = "WASTEBIN_PASTE_EXPIRATIONS";
const VAR_SIGNING_KEY: &str = "WASTEBIN_SIGNING_KEY";
const VAR_THEME: &str = "WASTEBIN_THEME";
const VAR_PASSWORD_SALT: &str = "WASTEBIN_PASSWORD_SALT";

#[derive(thiserror::Error, Debug)]
pub(crate) enum Error {
    #[error("failed to parse {VAR_CACHE_SIZE}, expected number of elements: {0}")]
    CacheSize(ParseIntError),
    #[error("failed to parse {VAR_DATABASE_PATH}, contains non-Unicode data")]
    DatabasePath,
    #[error("failed to parse {VAR_MAX_BODY_SIZE}, expected number of bytes: {0}")]
    MaxBodySize(ParseIntError),
    #[error("failed to parse {VAR_ADDRESS_PORT}, expected `host:port`")]
    AddressPort,
    #[error("failed to parse {VAR_BASE_URL}: {0}")]
    BaseUrl(String),
    #[error("failed to generate key from {VAR_SIGNING_KEY}: {0}")]
    SigningKey(String),
    #[error("failed to parse {VAR_HTTP_TIMEOUT}: {0}")]
    HttpTimeout(ParseIntError),
    #[error("failed to parse {VAR_PASTE_EXPIRATIONS}: {0}")]
    ParsePasteExpiration(#[from] expiration::Error),
    #[error("unknown theme {0}")]
    UnknownTheme(String),
}

pub fn title() -> String {
    std::env::var("WASTEBIN_TITLE").unwrap_or_else(|_| "wastebin".to_string())
}

pub fn theme() -> Result<highlight::Theme, Error> {
    std::env::var(VAR_THEME).map_or_else(
        |_| Ok(highlight::Theme::Ayu),
        |var| match var.as_str() {
            "ayu" => Ok(highlight::Theme::Ayu),
            "base16ocean" => Ok(highlight::Theme::Base16Ocean),
            "coldark" => Ok(highlight::Theme::Coldark),
            "gruvbox" => Ok(highlight::Theme::Gruvbox),
            "monokai" => Ok(highlight::Theme::Monokai),
            "onehalf" => Ok(highlight::Theme::Onehalf),
            "solarized" => Ok(highlight::Theme::Solarized),
            _ => Err(Error::UnknownTheme(var)),
        },
    )
}

pub fn cache_size() -> Result<NonZeroUsize, Error> {
    std::env::var(VAR_CACHE_SIZE)
        .map_or_else(
            |_| Ok(NonZeroUsize::new(128).expect("128 is non-zero")),
            |s| s.parse::<NonZeroUsize>(),
        )
        .map_err(Error::CacheSize)
}

pub fn database_method() -> Result<db::Open, Error> {
    match std::env::var(VAR_DATABASE_PATH) {
        Ok(path) => Ok(db::Open::Path(PathBuf::from(path))),
        Err(VarError::NotUnicode(_)) => Err(Error::DatabasePath),
        Err(VarError::NotPresent) => Ok(db::Open::Memory),
    }
}

pub fn signing_key() -> Result<Key, Error> {
    std::env::var(VAR_SIGNING_KEY).map_or_else(
        |_| Ok(Key::generate()),
        |s| Key::try_from(s.as_bytes()).map_err(|err| Error::SigningKey(err.to_string())),
    )
}

pub fn addr() -> Result<SocketAddr, Error> {
    std::env::var(VAR_ADDRESS_PORT)
        .as_ref()
        .map(String::as_str)
        .unwrap_or("0.0.0.0:8088")
        .parse()
        .map_err(|_| Error::AddressPort)
}

pub fn max_body_size() -> Result<usize, Error> {
    std::env::var(VAR_MAX_BODY_SIZE)
        .map_or_else(|_| Ok(1024 * 1024), |s| s.parse::<usize>())
        .map_err(Error::MaxBodySize)
}

/// Read base URL either from the environment variable or fallback to the hostname.
pub fn base_url() -> Result<url::Url, Error> {
    if let Some(base_url) = std::env::var(VAR_BASE_URL).map_or_else(
        |err| {
            if matches!(err, VarError::NotUnicode(_)) {
                Err(Error::BaseUrl(format!("{VAR_BASE_URL} is not unicode")))
            } else {
                Ok(None)
            }
        },
        |var| {
            Ok(Some(
                url::Url::parse(&var).map_err(|err| Error::BaseUrl(err.to_string()))?,
            ))
        },
    )? {
        return Ok(base_url);
    }

    let hostname =
        hostname::get().map_err(|err| Error::BaseUrl(format!("failed to get hostname: {err}")))?;

    url::Url::parse(&format!("https://{}", hostname.to_string_lossy()))
        .map_err(|err| Error::BaseUrl(err.to_string()))
}

pub fn password_hash_salt() -> String {
    std::env::var(VAR_PASSWORD_SALT).unwrap_or_else(|_| "somesalt".to_string())
}

pub fn http_timeout() -> Result<Duration, Error> {
    std::env::var(VAR_HTTP_TIMEOUT)
        .map_or_else(
            |_| Ok(DEFAULT_HTTP_TIMEOUT),
            |s| s.parse::<u64>().map(|v| Duration::new(v, 0)),
        )
        .map_err(Error::HttpTimeout)
}

/// Parse [`expiration::ExpirationSet`] from environment or return default.
pub fn expiration_set() -> Result<expiration::ExpirationSet, Error> {
    let set = std::env::var(VAR_PASTE_EXPIRATIONS).map_or_else(
        |_| "0,600,3600=d,86400,604800,2419200,29030400".parse::<expiration::ExpirationSet>(),
        |value| value.parse::<expiration::ExpirationSet>(),
    )?;

    Ok(set)
}
