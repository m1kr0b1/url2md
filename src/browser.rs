//! Headless browser control for JavaScript-rendered content with anti-detection

use crate::error::{Error, Result};
use headless_chrome::{Browser, LaunchOptions};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex as AsyncMutex;

/// Headless browser controller with stealth capabilities
pub struct BrowserController {
    browser: Arc<AsyncMutex<Browser>>,
    verbose: bool,
}

impl BrowserController {
    /// Launch a new headless browser instance with stealth settings
    pub fn new(verbose: bool) -> Result<Self> {
        let browser = launch_stealth_browser(verbose)?;
        Ok(Self {
            browser: Arc::new(AsyncMutex::new(browser)),
            verbose,
        })
    }

    /// Navigate to a URL and wait for content to render
    pub async fn fetch_and_render(
        &self,
        url: &str,
        wait_ms: u64,
        timeout_secs: u64,
    ) -> Result<RenderResult> {
        let start = Instant::now();

        if self.verbose {
            eprintln!("[html2md] Browser: Launching stealth Chrome...");
        }

        let browser = self.browser.lock().await;
        
        // Create new incognito-style tab
        let tab = browser.new_tab().map_err(|e| Error::BrowserError(e.to_string()))?;

        if self.verbose {
            eprintln!("[html2md] Browser: Navigating to {}", url);
        }

        // Inject anti-detection JavaScript BEFORE navigation
        let stealth_script = r#"
            Object.defineProperty(navigator, 'webdriver', { get: () => undefined });
            window.navigator.chrome = { runtime: {} };
            Object.defineProperty(navigator, 'plugins', { get: () => [1, 2, 3, 4, 5] });
            Object.defineProperty(navigator, 'languages', { get: () => ['en-US', 'en'] });
            window.chrome = { runtime: {} };
            if (window.navigator.mediaDevices === undefined) {
                window.navigator.mediaDevices = {};
            }
            if (window.navigator.mediaDevices.getUserMedia === undefined) {
                window.navigator.mediaDevices.getUserMedia = () => Promise.resolve({});
            }
            const originalQuery = window.navigator.permissions.query;
            window.navigator.permissions.query = (parameters) => (
                parameters.name === 'notifications' ?
                    Promise.resolve({ state: Notification.permission }) :
                    originalQuery(parameters)
            );
        "#;

        // Enable debugging features first
        let _ = tab.enable_runtime();
        let _ = tab.enable_debugger();

        // Navigate to page
        tab.navigate_to(url)
            .map_err(|e: anyhow::Error| Error::BrowserError(e.to_string()))?;

        // Inject anti-detection script after navigation
        if let Err(e) = tab.evaluate(stealth_script, false) {
            if self.verbose {
                eprintln!("[html2md] Browser: Could not inject stealth script: {}", e);
            }
        }

        if self.verbose {
            eprintln!("[html2md] Browser: Page loading, waiting for content...");
        }

        // Wait for page to stabilize - critical for X.com
        tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
        
        // Wait for specific content indicators
        let mut attempts = 0;
        let max_attempts = (timeout_secs / 2) as usize;
        
        while attempts < max_attempts {
            attempts += 1;
            
            // Try to get page title to verify content loaded
            match tab.get_title() {
                Ok(title) if !title.is_empty() => {
                    if self.verbose {
                        eprintln!("[html2md] Browser: Page loaded, title: {}", title);
                    }
                    break;
                }
                _ => {
                    if self.verbose {
                        eprintln!("[html2md] Browser: Waiting for content... (attempt {})", attempts);
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                }
            }
        }

        // Additional wait for JS to finish rendering
        tokio::time::sleep(std::time::Duration::from_millis(wait_ms)).await;

        if self.verbose {
            eprintln!("[html2md] Browser: Extracting HTML...");
        }

        // Get the rendered HTML
        let html = tab.get_content()
            .map_err(|e: anyhow::Error| Error::BrowserError(e.to_string()))?;

        let elapsed = start.elapsed();
        let final_url = tab.get_url();

        // Check content quality
        let is_empty = html.trim().is_empty() 
            || html.len() < 500
            || html.contains("<html><head></head><body></body></html>")
            || !html.contains("<article") && !html.contains("<p") && !html.contains("data-tweet-id");

        if self.verbose {
            eprintln!("[html2md] Browser: Got {} bytes in {:?}", html.len(), elapsed);
            eprintln!("[html2md] Browser: Final URL: {}", final_url);
            if is_empty {
                eprintln!("[html2md] Browser: WARNING - content appears thin");
            }
        }

        // Close tab
        let _ = tab.close(true);

        if is_empty {
            return Err(Error::EmptyPageAfterJs);
        }

        Ok(RenderResult {
            url: final_url,
            html,
            elapsed,
        })
    }
}

/// Launch browser with maximum stealth/anti-detection settings
fn launch_stealth_browser(verbose: bool) -> Result<Browser> {
    let chrome_path = find_chrome_path();

    // Comprehensive anti-detection Chrome flags
    let chrome_args = vec![
        // Disable automation detection
        std::ffi::OsStr::new("--disable-blink-features=AutomationControlled"),
        std::ffi::OsStr::new("--disable-blink-features=AutomationBenchmark"),
        std::ffi::OsStr::new("--disable-blink-security-features"),
        
        // Sandbox settings (required for Chrome on macOS)
        std::ffi::OsStr::new("--no-sandbox"),
        std::ffi::OsStr::new("--disable-setuid-sandbox"),
        
        // Disable features that expose automation
        std::ffi::OsStr::new("--disable-dev-shm-usage"),
        std::ffi::OsStr::new("--disable-gpu"),
        std::ffi::OsStr::new("--disable-software-rasterizer"),
        
        // Disable web security features that detect bots
        std::ffi::OsStr::new("--disable-web-security"),
        std::ffi::OsStr::new("--allow-running-insecure-content"),
        
        // Disable features that track automation
        std::ffi::OsStr::new("--disable-features=IsolateOrigins,site-per-process,Chrome杠杆21"),
        std::ffi::OsStr::new("--disable-site-isolation-trials"),
        
        // User agent and platform
        std::ffi::OsStr::new("--user-agent=Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36"),
        
        // Language and locale
        std::ffi::OsStr::new("--accept-lang=en-US,en;q=0.9"),
        
        // Window and viewport
        std::ffi::OsStr::new("--window-size=1920,1080"),
        std::ffi::OsStr::new("--screen-width=1920"),
        std::ffi::OsStr::new("--screen-height=1080"),
        std::ffi::OsStr::new("--screen-avail-width=1920"),
        std::ffi::OsStr::new("--screen-avail-height=1040"),
        
        // Disable notifications and permissions prompts
        std::ffi::OsStr::new("--disable-notifications"),
        std::ffi::OsStr::new("--disable-permissions-api"),
        
        // Disable background networking
        std::ffi::OsStr::new("--disable-background-networking"),
        std::ffi::OsStr::new("--disable-default-apps"),
        std::ffi::OsStr::new("--disable-sync"),
        std::ffi::OsStr::new("--disable-translate"),
        
        // Disable extensions
        std::ffi::OsStr::new("--disable-extensions"),
        
        // Performance optimizations
        std::ffi::OsStr::new("--disable-background-timer-throttling"),
        std::ffi::OsStr::new("--disable-backgrounding-occluded-windows"),
        std::ffi::OsStr::new("--disable-renderer-backgrounding"),
        
        // Logging and crash reporting
        std::ffi::OsStr::new("--disable-logging"),
        std::ffi::OsStr::new("--disable-crash-reporter"),
        std::ffi::OsStr::new("--no-first-run"),
        std::ffi::OsStr::new("--no-default-browser-check"),
        
        // Memory optimization
        std::ffi::OsStr::new("--disable-hang-monitor"),
        std::ffi::OsStr::new("--disable-prompt-on-repost"),
        std::ffi::OsStr::new("--disable-popup-blocking"),
        
        // Content settings
        std::ffi::OsStr::new("--disable-speech-api"),
        std::ffi::OsStr::new("--disable-speech-input"),
        std::ffi::OsStr::new("--disable-web-security-user-level"),
        
        // Font settings
        std::ffi::OsStr::new("--force-color-profile=srgb"),
        
        // Touch settings
        std::ffi::OsStr::new("--disable-touch-editing"),
        std::ffi::OsStr::new("--enable-vertical-scroll"),
        std::ffi::OsStr::new("--enable-horizontal-scroll"),
        
        // WebGL settings (anti-fingerprinting)
        std::ffi::OsStr::new("--enable-webgl"),
        std::ffi::OsStr::new("--use-gl=swiftshader"),
        
        // Misc
        std::ffi::OsStr::new("--hide-scrollbars"),
        std::ffi::OsStr::new("--metrics-recording-only"),
        std::ffi::OsStr::new("--mute-audio"),
        std::ffi::OsStr::new("--no-service-autorun"),
        std::ffi::OsStr::new("--password-store=basic"),
        std::ffi::OsStr::new("--use-mock-keychain"),
        
        // Disable automation detection via CDP
        std::ffi::OsStr::new("--disable-automation"),
        
        // Enable features that make browser look more normal
        std::ffi::OsStr::new("--allow-insecure-localhost"),
        std::ffi::OsStr::new("--ignore-certificate-errors"),
        
        // Additional stealth
        std::ffi::OsStr::new("--disable-ipc-flooding-protection"),
        std::ffi::OsStr::new("--disable-new-content-adr"),
        std::ffi::OsStr::new("--disable-profile-header"),
        std::ffi::OsStr::new("--disable-renderer-accessibility"),
        std::ffi::OsStr::new("--disable-search-geolocation-disclosure"),
        std::ffi::OsStr::new("--disable-third-party-keyboard-workaround"),
        std::ffi::OsStr::new("--disable-trace-bg-info"),
        std::ffi::OsStr::new("--enable-features=NetworkService,NetworkServiceInProcess2"),
        std::ffi::OsStr::new("--metrics-defaults"),
        
        // Simulate real browser timing
        std::ffi::OsStr::new("--disable-back-forward-cache"),
        std::ffi::OsStr::new("--disable-breakpad"),
        std::ffi::OsStr::new("--disable-component-extensions-with-background-pages"),
        std::ffi::OsStr::new("--disable-component-update"),
        std::ffi::OsStr::new("--disable-domain-reliability"),
        
        // More stealth features
        std::ffi::OsStr::new("--disable-features=AudioServiceAec,AudioServiceNoiseCancellation"),
        std::ffi::OsStr::new("--disable-hid-detection"),
    ];

    let launch_options = LaunchOptions {
        headless: true,
        path: chrome_path,
        window_size: Some((1920, 1080)),
        args: chrome_args,
        enable_logging: false,
        ..Default::default()
    };

    if verbose {
        eprintln!("[html2md] Launching stealth Chrome...");
    }

    let browser = Browser::new(launch_options)
        .map_err(|e| Error::ChromeLaunchFailure(e.to_string()))?;

    if verbose {
        eprintln!("[html2md] Chrome launched successfully");
    }

    Ok(browser)
}

/// Result of browser rendering
#[derive(Debug, Clone)]
pub struct RenderResult {
    pub url: String,
    pub html: String,
    pub elapsed: std::time::Duration,
}

/// Find Chrome/Chromium executable path
fn find_chrome_path() -> Option<PathBuf> {
    // Common Chrome installation paths on macOS
    let mac_paths = vec![
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Chromium.app/Contents/MacOS/Chromium",
        "/Applications/Chrome.app/Contents/MacOS/Chrome",
    ];

    for path in mac_paths {
        if std::path::Path::new(path).exists() {
            return Some(PathBuf::from(path));
        }
    }

    // Try to find via mdfind on macOS
    if std::env::consts::OS == "macos" {
        if let Ok(output) = std::process::Command::new("mdfind")
            .args(["kMDItemDisplayName == 'Google Chrome.app'"])
            .output()
        {
            let path = String::from_utf8_lossy(&output.stdout);
            for line in path.lines() {
                let chrome_path = format!("{}/Contents/MacOS/Google Chrome", line.trim());
                if std::path::Path::new(&chrome_path).exists() {
                    return Some(PathBuf::from(chrome_path));
                }
            }
        }
    }

    // Try default_executable
    headless_chrome::browser::default_executable().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires Chrome to be installed
    fn test_stealth_browser_fetch() {
        let browser = BrowserController::new(false);
        if browser.is_err() {
            panic!("Failed to create browser: {:?}", browser.err());
        }

        let browser = browser.unwrap();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(browser.fetch_and_render("https://example.com", 2000, 30));

        assert!(result.is_ok(), "Failed: {:?}", result.err());
        let result = result.unwrap();
        assert!(result.html.len() > 100, "HTML too short");
    }
}
