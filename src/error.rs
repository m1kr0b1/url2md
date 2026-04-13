//! Error types for html2md

use thiserror::Error;

/// Main error type for html2md operations
#[derive(Error, Debug)]
pub enum Error {
    #[error("DNS resolution failed for '{0}'")]
    DnsFailure(String),

    #[error("TCP connection timeout after {0}s for '{1}'")]
    TcpTimeout(u64, String),

    #[error("HTTP {status} response from '{url}': {reason}")]
    HttpStatus {
        status: u16,
        url: String,
        reason: String,
    },

    #[error("Too many redirects (limit: 10) from '{0}'")]
    TooManyRedirects(String),

    #[error("Redirect loop detected: {0}")]
    RedirectLoop(String),

    #[error("Invalid redirect from '{from}' to '{to}'")]
    InvalidRedirect { from: String, to: String },

    #[error("Failed to parse URL '{0}': {1}")]
    InvalidUrl(String, #[source] url::ParseError),

    #[error("Failed to fetch URL '{0}': {1}")]
    FetchFailure(String, String),

    #[error("Browser operation failed: {0}")]
    BrowserError(String),

    #[error("Chrome process error: {0}")]
    ChromeProcessError(#[source] std::io::Error),

    #[error("Failed to launch Chrome: {0}")]
    ChromeLaunchFailure(String),

    #[error("HTML parsing failed: {0}")]
    HtmlParseError(String),

    #[error("Page remained empty after JS rendering")]
    EmptyPageAfterJs,

    #[error("TLS configuration error: {0}")]
    TlsError(String),

    #[error("Timeout exceeded: {0}s")]
    Timeout(u64),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Output file error: {0}")]
    OutputError(String),

    #[error("{0}")]
    Custom(String),

    #[error("Parse error: {0}")]
    ParseError(String),
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        let url = err.url().map(|u| u.to_string()).unwrap_or_else(|| "unknown".to_string());
        
        if err.is_timeout() {
            return Error::TcpTimeout(10, url);
        }
        
        if err.is_redirect() {
            return Error::TooManyRedirects(url);
        }

        if let Some(status) = err.status() {
            let reason = status.canonical_reason().unwrap_or("Unknown").to_string();
            return Error::HttpStatus {
                status: status.as_u16(),
                url,
                reason,
            };
        }

        Error::FetchFailure(url, err.to_string())
    }
}

impl From<url::ParseError> for Error {
    fn from(err: url::ParseError) -> Self {
        Error::InvalidUrl(err.to_string(), err)
    }
}

impl From<scraper::error::SelectorErrorKind<'_>> for Error {
    fn from(err: scraper::error::SelectorErrorKind<'_>) -> Self {
        Error::HtmlParseError(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
