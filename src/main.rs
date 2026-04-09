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
    let url = validate_url(&args.url)?;

    // Determine if we need JS rendering
    let use_js_rendering = !args.no_js;

    let (final_url, final_html) = if use_js_rendering {
        // Go straight to browser - skip HTTP fetch for JS-heavy sites like X.com
        if args.verbose {
            eprintln!("[html2md] Using headless browser for JavaScript rendering...");
        }

        match render_with_browser(&url, args.js_wait_ms, args.timeout, args.verbose).await {
            Ok(render_result) => {
                if args.verbose {
                    eprintln!("[html2md] JavaScript rendering successful!");
                }
                (render_result.url, render_result.html)
            }
            Err(e) => {
                // Fallback to HTTP if browser fails
                if args.verbose {
                    eprintln!("[html2md] Browser failed: {}", e);
                    eprintln!("[html2md] Falling back to HTTP fetch...");
                }
                
                let scraper = Scraper::with_user_agent(
                    args.timeout_duration(),
                    args.verbose,
                    args.user_agent(),
                )?;
                
                let fetch_result = scraper.fetch(&url).await?;
                (fetch_result.url, fetch_result.body)
            }
        }
    } else {
        // HTTP only mode
        if args.verbose {
            eprintln!("[html2md] Fetching via HTTP (no JS)...");
        }
        
        let scraper = Scraper::with_user_agent(
            args.timeout_duration(),
            args.verbose,
            args.user_agent(),
        )?;
        
        let fetch_result = scraper.fetch(&url).await?;
        (fetch_result.url, fetch_result.body)
    };

    // Convert to Markdown
    let markdown = {
        let converter = Converter::with_verbose(&final_url, args.verbose)?;
        converter.convert(&final_html)?
    };

    // Output result
    if args.verbose {
        let elapsed = start.elapsed();
        eprintln!("[html2md] Conversion complete in {:?}", elapsed);
        eprintln!("[html2md] Output size: {} bytes", markdown.len());
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

/// Determine if JS rendering should be used based on content
fn should_use_js(html: &str, min_tokens: usize) -> bool {
    // Count rough tokens (words + visible elements)
    let content = html.to_lowercase();
    
    // Check for indicators of JavaScript-rendered content
    let js_indicators = [
        "ng-app",
        "ng-controller",
        "ng-init",
        "react",
        "vue",
        "angular",
        "ember",
        "backbone",
        "svelte",
        "data-v-",      // Vue
        "ember-cli-",   // Ember
    ];

    let has_js_indicators = js_indicators.iter().any(|ind| content.contains(ind));
    
    // Check for SPA indicators
    let spa_indicators = [
        "id=\"app\"",
        "id=\"root\"",
        "class=\"app\"",
        "class=\"root\"",
        "data-reactroot",
    ];

    let has_spa_indicators = spa_indicators.iter().any(|ind| content.contains(ind));

    // Check for thin content
    let has_thin_content = html.len() < min_tokens * 4; // Rough estimate: 4 chars per token

    // Use JS if content is thin or has JS indicators
    has_thin_content || has_js_indicators || has_spa_indicators
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
