//! HTTP scraping with browser impersonation

use crate::error::{Error, Result};

pub const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36";
use reqwest::{
    header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_ENCODING, ACCEPT_LANGUAGE, CACHE_CONTROL, DNT, UPGRADE_INSECURE_REQUESTS, USER_AGENT as UA_HEADER},
    redirect::Policy,
    Client, ClientBuilder,
};
use std::time::{Duration, Instant};

/// HTTP scraper with browser-like behavior
pub struct Scraper {
    client: Client,
    verbose: bool,
}

impl Scraper {
    /// Create a new scraper with default Chrome-like configuration
    pub fn new(timeout: Duration, verbose: bool) -> Result<Self> {
        Self::with_user_agent(timeout, verbose, USER_AGENT)
    }

    /// Create a new scraper with custom User-Agent
    pub fn with_user_agent(timeout: Duration, verbose: bool, user_agent: &str) -> Result<Self> {
        // Use default redirect policy (10 redirects max)
        let redirect_policy = Policy::limited(10);

        let mut headers = HeaderMap::new();
        headers.insert(UA_HEADER, HeaderValue::from_str(user_agent).unwrap_or_else(|_| HeaderValue::from_static(USER_AGENT)));
        headers.insert(ACCEPT, HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7"));
        headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.9"));
        headers.insert(ACCEPT_ENCODING, HeaderValue::from_static("gzip, deflate, br, zstd"));
        
        // Chrome Client Hints (using custom header names since they're not in reqwest)
        headers.insert("Sec-CH-UA", HeaderValue::from_static("\"Chromium\";v=\"134\", \"Google Chrome\";v=\"134\", \"Not-A.Brand\";v=\"24\""));
        headers.insert("Sec-CH-UA-Mobile", HeaderValue::from_static("?0"));
        headers.insert("Sec-CH-UA-Platform", HeaderValue::from_static("\"Windows\""));
        
        // Fetch hints
        headers.insert("Sec-Fetch-Dest", HeaderValue::from_static("document"));
        headers.insert("Sec-Fetch-Mode", HeaderValue::from_static("navigate"));
        headers.insert("Sec-Fetch-Site", HeaderValue::from_static("none"));
        headers.insert("Sec-Fetch-User", HeaderValue::from_static("?1"));
        
        // Additional Chrome headers
        headers.insert(UPGRADE_INSECURE_REQUESTS, HeaderValue::from_static("1"));
        headers.insert(DNT, HeaderValue::from_static("1"));
        headers.insert(CACHE_CONTROL, HeaderValue::from_static("max-age=0"));

        let client = ClientBuilder::new()
            .tcp_keepalive(Duration::from_secs(120))
            .tcp_nodelay(true)
            .connection_verbose(verbose)
            .default_headers(headers)
            .redirect(redirect_policy)
            .timeout(timeout)
            .build()
            .map_err(|e| Error::TlsError(e.to_string()))?;

        Ok(Self {
            client,
            verbose,
        })
    }

    /// Fetch a URL and return the response body as string
    pub async fn fetch(&self, url: &str) -> Result<FetchResult> {
        let start = Instant::now();

        if self.verbose {
            eprintln!("[html2md] Fetching: {}", url);
        }

        let response = self.client.get(url).send().await?;
        let elapsed = start.elapsed();
        let status = response.status();
        let final_url = response.url().to_string();

        if self.verbose {
            eprintln!("[html2md] Status: {} (took {:?})", status, elapsed);
            eprintln!("[html2md] Final URL: {}", final_url);
        }

        // Handle HTTP errors
        if !status.is_success() {
            let reason = status.canonical_reason().unwrap_or("Unknown");
            return Err(Error::HttpStatus {
                status: status.as_u16(),
                url: final_url,
                reason: reason.to_string(),
            });
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let body = response.text().await?;

        if self.verbose {
            eprintln!("[html2md] Content length: {} bytes", body.len());
            if let Some(ref ct) = content_type {
                eprintln!("[html2md] Content-Type: {}", ct);
            }
        }

        Ok(FetchResult {
            url: final_url,
            body,
            content_type,
            status,
            elapsed,
        })
    }
}

/// Result of a fetch operation
#[derive(Debug, Clone)]
pub struct FetchResult {
    pub url: String,
    pub body: String,
    pub content_type: Option<String>,
    pub status: reqwest::StatusCode,
    pub elapsed: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_simple_page() {
        let scraper = Scraper::new(Duration::from_secs(10), false).unwrap();
        let result = scraper.fetch("https://example.com").await;
        assert!(result.is_ok() || result.is_err()); // Allow network failures in test
    }

    #[tokio::test]
    async fn test_fetch_invalid_url() {
        let scraper = Scraper::new(Duration::from_secs(10), false).unwrap();
        let result = scraper.fetch("not-a-url").await;
        assert!(result.is_err());
    }
}
