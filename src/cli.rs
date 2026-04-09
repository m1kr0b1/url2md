//! CLI argument parsing for html2md

use crate::scraper::USER_AGENT;
use clap::{Parser, ValueHint};
use std::path::PathBuf;

/// Convert web pages to clean Markdown
#[derive(Parser, Debug)]
#[command(
    name = "html2md",
    version = "0.1.0",
    about = "Convert web pages to clean, well-structured Markdown",
    long_about = None,
    author = "Developer"
)]
pub struct Args {
    /// URL to convert to Markdown
    #[arg(value_hint = ValueHint::Url)]
    pub url: String,

    /// Request timeout in seconds (default: 60)
    #[arg(short, long, env = "HTML2MD_TIMEOUT", default_value = "60")]
    pub timeout: u64,

    /// Disable JavaScript rendering (use plain HTTP only)
    #[arg(short = 'n', long = "no-js")]
    pub no_js: bool,

    /// Override the default User-Agent string
    #[arg(short, long, env = "HTML2MD_USER_AGENT")]
    pub user_agent: Option<String>,

    /// Output file path (default: stdout)
    #[arg(short, long, value_hint = ValueHint::FilePath)]
    pub output: Option<PathBuf>,

    /// Enable verbose output (print request headers and timing)
    #[arg(short, long)]
    pub verbose: bool,

    /// Minimum content length (in tokens) to trigger JS rendering fallback
    #[arg(long, default_value = "500")]
    pub min_content_tokens: usize,

    /// Additional delay before JS rendering wait (milliseconds)
    #[arg(long, default_value = "3000")]
    pub js_wait_ms: u64,
}

impl Args {
    /// Get the User-Agent string
    pub fn user_agent(&self) -> &str {
        self.user_agent.as_deref().unwrap_or(USER_AGENT)
    }

    /// Get the timeout duration
    pub fn timeout_duration(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.timeout)
    }
}

/// Chrome browser identification
pub const CHROME_VERSION: &str = "Chrome/134.0.0.0";

/// Platform identifier
pub const PLATFORM: &str = "\"Windows\"";

/// Mobile indicator
pub const IS_MOBILE: &str = "?0";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_user_agent() {
        let args = Args::parse_from(["html2md", "https://example.com"]);
        assert_eq!(args.user_agent(), USER_AGENT);
    }

    #[test]
    fn test_custom_user_agent() {
        let args = Args::parse_from(["html2md", "-u", "CustomAgent/1.0", "https://example.com"]);
        assert_eq!(args.user_agent(), "CustomAgent/1.0");
    }
}
