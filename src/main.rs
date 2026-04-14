//! html2md - Convert web pages to clean Markdown
//!
//! A production-grade CLI tool that scrapes URLs and converts their content
//! to well-structured Markdown with full browser impersonation and JS rendering support.

mod browser;
mod cli;
mod converter;
mod error;
mod scraper;

use crate::browser::BrowserController;
use crate::cli::Args;
use crate::converter::Converter;
use crate::error::Error;
use crate::scraper::Scraper;
use clap::Parser;
use std::io::{self, Write};
use std::path::Path;
use std::time::Instant;

#[tokio::main]
async fn main() {
    // Parse CLI arguments
    let args = Args::parse();

    // Run the main async function
    if let Err(e) = run(args).await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

/// Main execution function
async fn run(args: Args) -> Result<(), Error> {
    let start = Instant::now();

    if args.verbose {
        eprintln!("[html2md] Starting...");
        eprintln!("[html2md] URL: {}", args.url);
        eprintln!("[html2md] Timeout: {}s", args.timeout);
        eprintln!("[html2md] JS rendering: {}", !args.no_js);
    }

    // Validate URL
    let mut url = validate_url(&args.url)?;

    // Reddit blocks all programmatic access (TLS/HTTP2 fingerprinting via Cloudflare).
    // Requires their official OAuth API — not currently supported.
    if is_reddit_url(&url) {
        eprintln!("Reddit is not supported: their Cloudflare bot protection blocks all automated access.");
        eprintln!("Use Reddit's official API (https://www.reddit.com/prefs/apps) for programmatic access.");
        std::process::exit(1);
    }

    // X.com / Twitter: use oEmbed API — Chrome is always blocked there
    if is_twitter_url(&url) {
        if args.verbose {
            eprintln!("[html2md] Detected X/Twitter URL, using oEmbed API...");
        }
        let scraper = Scraper::with_user_agent(args.timeout_duration(), args.verbose, args.user_agent())?;
        let markdown = fetch_tweet_oembed(&scraper, &url, args.verbose).await?;
        write_output(&markdown, &args.output)?;
        return Ok(());
    }

    // Always try HTTP first — fast path for static pages
    if args.verbose {
        eprintln!("[html2md] Fetching via HTTP...");
    }

    let scraper = Scraper::with_user_agent(
        args.timeout_duration(),
        args.verbose,
        args.user_agent(),
    )?;

    let fetch_result = scraper.fetch(&url).await?;

    let (final_url, final_html) = if !args.no_js && should_use_js(&fetch_result.body, args.min_content_tokens) {
        // HTTP content is thin or JS-heavy — fall back to Chrome
        if args.verbose {
            eprintln!("[html2md] Content looks JS-rendered, switching to headless browser...");
        }

        match render_with_browser(&url, args.js_wait_ms, args.timeout, args.verbose).await {
            Ok(render_result) => {
                if args.verbose {
                    eprintln!("[html2md] Browser rendering successful!");
                }
                (render_result.url, render_result.html)
            }
            Err(e) => {
                if args.verbose {
                    eprintln!("[html2md] Browser failed ({}), using HTTP result", e);
                }
                (fetch_result.url, fetch_result.body)
            }
        }
    } else {
        // HTTP content is good — use it directly
        if args.verbose {
            eprintln!("[html2md] Using HTTP result ({} bytes)", fetch_result.body.len());
        }
        (fetch_result.url, fetch_result.body)
    };

    // Convert to Markdown
    let markdown = {
        let converter = Converter::with_verbose(&final_url, args.verbose)?;
        converter.convert(&final_html)?
    };

    // Output result
    let elapsed = start.elapsed();
    if args.verbose {
        eprintln!("[html2md] Conversion complete in {:?}", elapsed);
        eprintln!("[html2md] Output size: {} bytes", markdown.len());
    }

    if markdown.len() < 100 {
        eprintln!("Warning: very little content extracted ({} bytes) — page may be paywalled, bot-blocked, or require authentication", markdown.len());
    }

    write_output(&markdown, &args.output)?;

    Ok(())
}

/// Validate and normalize the URL
fn validate_url(url: &str) -> Result<String, Error> {
    // Add https:// if no scheme
    let url = if !url.starts_with("http://") && !url.starts_with("https://") {
        format!("https://{}", url)
    } else {
        url.to_string()
    };

    // Validate URL format
    let parsed = url::Url::parse(&url)?;
    
    // Ensure it has a host
    if parsed.host_str().is_none() {
        return Err(Error::InvalidUrl(
            url.clone(),
            url::ParseError::EmptyHost,
        ));
    }

    Ok(url)
}

/// Extract clean tweet text from oEmbed blockquote HTML
fn extract_tweet_text(html: &str) -> String {
    use ::scraper::{Html, Selector};
    let doc = Html::parse_fragment(html);
    // The first <p> inside the blockquote contains the tweet text
    if let Ok(sel) = Selector::parse("blockquote p") {
        let parts: Vec<String> = doc.select(&sel)
            .take(1) // only first <p> — second is the author/date line
            .map(|el| -> String { el.text().collect::<Vec<_>>().join("") })
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !parts.is_empty() {
            return parts.join("\n\n");
        }
    }
    html.to_string()
}

fn is_reddit_url(url: &str) -> bool {
    url.contains("://reddit.com/") || url.contains("://www.reddit.com/")
}

/// Check if a URL is an X.com or Twitter URL
fn is_twitter_url(url: &str) -> bool {
    url.contains("://x.com/") || url.contains("://twitter.com/")
}

/// Fetch a tweet via the public oEmbed API and return Markdown
async fn fetch_tweet_oembed(scraper: &Scraper, url: &str, verbose: bool) -> Result<String, Error> {
    let oembed_url = format!(
        "https://publish.twitter.com/oembed?url={}&omit_script=true",
        url::form_urlencoded::byte_serialize(url.as_bytes()).collect::<String>()
    );

    if verbose {
        eprintln!("[html2md] oEmbed URL: {}", oembed_url);
    }

    let result = scraper.fetch(&oembed_url).await?;

    // Parse the JSON response
    let json: serde_json::Value = serde_json::from_str(&result.body)
        .map_err(|e| Error::ParseError(format!("oEmbed JSON parse error: {}", e)))?;

    let author = json["author_name"].as_str().unwrap_or("Unknown");
    let author_url = json["author_url"].as_str().unwrap_or("");
    let html = json["html"].as_str().unwrap_or("");

    // oEmbed returns a <blockquote> with the tweet text — extract just the <p> content
    let tweet_text = extract_tweet_text(html);

    Ok(format!("**[{}]({})** on X.com\n\n{}\n\n> Source: {}", author, author_url, tweet_text.trim(), url))
}

/// Determine if JS rendering should be used based on content
fn should_use_js(html: &str, min_tokens: usize) -> bool {
    // Check visible text density — JS-rendered pages have lots of HTML but little visible text
    let visible_chars = {
        let mut in_tag = false;
        let mut count = 0usize;
        for c in html.chars() {
            match c {
                '<' => in_tag = true,
                '>' => in_tag = false,
                c if !in_tag && !c.is_whitespace() => count += 1,
                _ => {}
            }
        }
        count
    };

    let density = visible_chars as f64 / html.len().max(1) as f64;

    // Low text density = JS-rendered shell (Medium, Reddit, SPAs)
    if density < 0.08 {
        return true;
    }

    // Thin content overall
    if html.len() < min_tokens * 4 {
        return true;
    }

    // Strong SPA markers
    let content = html.to_lowercase();
    content.contains("data-reactroot")
        || content.contains("ng-app=")
        || content.contains("ember-cli-")
        || (content.contains("id=\"root\"") && content.contains("<script"))
        || (content.contains("id=\"app\"") && content.contains("<script"))
}

/// Render page with headless browser
async fn render_with_browser(
    url: &str,
    wait_ms: u64,
    timeout_secs: u64,
    verbose: bool,
) -> Result<browser::RenderResult, Error> {
    if verbose {
        eprintln!("[html2md] Initializing headless browser...");
    }
    
    // Create browser controller with stealth settings
    let controller = BrowserController::new(verbose)
        .map_err(|e| Error::BrowserError(format!("Failed to launch browser: {}", e)))?;
    
    if verbose {
        eprintln!("[html2md] Browser ready, fetching URL...");
    }
    
    // Fetch and render with extended timeout
    controller
        .fetch_and_render(url, wait_ms, timeout_secs)
        .await
        .map_err(|e| Error::BrowserError(format!("Failed to fetch and render: {}", e)))
}

/// Write output to file or stdout
fn write_output(markdown: &str, output_path: &Option<impl AsRef<Path>>) -> Result<(), Error> {
    match output_path {
        Some(path) => {
            let path = path.as_ref();
            let mut file = std::fs::File::create(path)
                .map_err(|e| Error::OutputError(format!("Failed to create file '{}': {}", path.display(), e)))?;
            
            file.write_all(markdown.as_bytes())
                .map_err(|e| Error::OutputError(format!("Failed to write to file '{}': {}", path.display(), e)))?;
        }
        None => {
            // Write to stdout
            io::stdout()
                .write_all(markdown.as_bytes())
                .map_err(|e| Error::OutputError(format!("Failed to write to stdout: {}", e)))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_url_with_https() {
        let result = validate_url("https://example.com");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "https://example.com");
    }

    #[test]
    fn test_validate_url_without_scheme() {
        let result = validate_url("example.com");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "https://example.com");
    }

    #[test]
    fn test_validate_url_with_path() {
        let result = validate_url("example.com/path/to/page");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "https://example.com/path/to/page");
    }

    #[test]
    fn test_validate_url_invalid() {
        let result = validate_url("not a url");
        assert!(result.is_err());
    }

    #[test]
    fn test_should_use_js_thin_content() {
        let thin_html = "<html><head></head><body><div id=\"app\"></div></body></html>";
        assert!(should_use_js(thin_html, 500));
    }

    #[test]
    fn test_should_use_js_react_app() {
        let react_html = "<html><body><div id=\"root\" data-reactroot></div><script src=\"react.js\"></script></body></html>";
        assert!(should_use_js(react_html, 100));
    }

    #[test]
    fn test_should_not_use_js_normal_content() {
        let normal_html = r#"
            <html>
            <body>
                <h1>Welcome to my page</h1>
                <p>This is a lot of content that would indicate this is a regular HTML page without JavaScript rendering requirements.</p>
                <p>More content here with plenty of text to make it appear as if this page has substantial content.</p>
            </body>
            </html>
        "#;
        assert!(!should_use_js(normal_html, 100));
    }
}
