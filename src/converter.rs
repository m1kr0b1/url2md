//! HTML to Markdown converter with aggressive deduplication

use crate::error::Result;
use regex::Regex;
use scraper::{Html, Selector};
use url::Url;

/// Convert HTML to Markdown
pub struct Converter {
    base_url: Url,
    verbose: bool,
}

impl Converter {
    pub fn new(base_url: &str) -> Result<Self> {
        let base_url = Url::parse(base_url)?;
        Ok(Self { base_url, verbose: false })
    }

    pub fn with_verbose(base_url: &str, verbose: bool) -> Result<Self> {
        let base_url = Url::parse(base_url)?;
        Ok(Self { base_url, verbose })
    }

    pub fn convert(&self, html: &str) -> Result<String> {
        if self.verbose {
            eprintln!("[html2md] Converting HTML to Markdown...");
        }

        let document = Html::parse_document(html);
        let cleaned = self.preprocess(&document);
        let document = Html::parse_document(&cleaned);

        // Prefer specific content areas before broad containers
        let root = self.select_one(&document, "article.markdown-body")
            .or_else(|| self.select_one(&document, "#readme"))
            .or_else(|| self.select_one(&document, ".markdown-body"))
            .or_else(|| self.select_one(&document, "article"))
            .or_else(|| self.select_one(&document, "#main-content"))
            .or_else(|| self.select_one(&document, "#content"))
            .or_else(|| self.select_one(&document, "[role='main']"))
            .or_else(|| self.select_one(&document, "main"))
            .or_else(|| self.select_one(&document, "body"));

        let root = match root {
            Some(r) => r,
            None => return Ok(String::new()),
        };

        // Get all meaningful content
        let content = self.extract_content(&root);

        // Aggressive deduplication
        let deduped = self.deduplicate_content(&content);

        Ok(deduped)
    }

    fn preprocess(&self, document: &Html) -> String {
        let mut html = document.root_element().html();

        // Remove scripts, styles, etc.
        let patterns = [
            r"<script[^>]*>[\s\S]*?</script>",
            r"<style[^>]*>[\s\S]*?</style>",
            r"<noscript[^>]*>[\s\S]*?</noscript>",
            r"<iframe[^>]*>[\s\S]*?</iframe>",
            r"<svg[^>]*>[\s\S]*?</svg>",
            r"<!--[\s\S]*?-->",
        ];

        for p in &patterns {
            if let Ok(re) = Regex::new(p) {
                html = re.replace_all(&html, "").to_string();
            }
        }

        // Remove nav, header, footer tags
        for tag in &["nav", "header", "footer", "aside", "menu"] {
            if let Ok(sel) = Selector::parse(tag) {
                let frag = Html::parse_fragment(&html);
                for elem in frag.select(&sel) {
                    html = html.replace(&elem.html(), "");
                }
            }
        }

        // Remove elements with navigation/UI class names
        let nav_classes = [
            "[class*='nav']", "[class*='header']", "[class*='footer']",
            "[class*='sidebar']", "[class*='toolbar']", "[class*='breadcrumb']",
            "[class*='cookie']", "[class*='banner']", "[class*='alert']",
            "[class*='notification']", "[class*='modal']", "[class*='overlay']",
            "[class*='popup']", "[class*='toast']", "[class*='skip']",
            "[role='navigation']", "[role='banner']", "[role='contentinfo']",
        ];
        for sel_str in &nav_classes {
            if let Ok(sel) = Selector::parse(sel_str) {
                let frag = Html::parse_fragment(&html);
                for elem in frag.select(&sel) {
                    html = html.replace(&elem.html(), "");
                }
            }
        }

        html
    }

    /// Extract content from body element
    fn extract_content(&self, element: &scraper::ElementRef) -> Vec<String> {
        let mut items = Vec::new();
        self.extract_recursive(element, &mut items);
        items
    }

    fn extract_recursive(&self, element: &scraper::ElementRef, items: &mut Vec<String>) {
        let tag = element.value().name();

        // Skip these entirely
        match tag {
            "script" | "style" | "noscript" | "iframe" | "svg" | "canvas" | "nav" | "header" | "footer" => return,
            _ => {}
        }

        match tag {
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                let level = tag[1..].parse().unwrap_or(1);
                let text = self.get_text(element).trim().to_string();
                if !text.is_empty() {
                    items.push(format!("{} {}", "#".repeat(level), text));
                }
            }
            "p" => {
                let text = self.get_text(element).trim().to_string();
                if !text.is_empty() && text.len() > 1 {
                    items.push(text);
                }
            }
            "blockquote" => {
                let text = self.get_text(element).trim().to_string();
                if !text.is_empty() {
                    for line in text.lines() {
                        items.push(format!("> {}", line));
                    }
                }
            }
            "pre" => {
                let code = self.get_text(element).trim().to_string();
                if !code.is_empty() {
                    let lang = self.get_lang(element);
                    items.push(format!("```{}\n{}\n```", lang, code));
                }
            }
            "ul" | "ol" => {
                for child in element.child_elements() {
                    if child.value().name() == "li" {
                        let text = self.get_text(&child).trim().to_string();
                        if !text.is_empty() {
                            items.push(format!("- {}", text));
                        }
                    }
                }
            }
            "table" => {
                // Extract rows directly — avoid triple-counting via tbody/tr/td recursion
                if let Ok(tr_sel) = Selector::parse("tr") {
                    for row in element.select(&tr_sel) {
                        let cells: Vec<String> = row.child_elements()
                            .filter(|e| matches!(e.value().name(), "td" | "th"))
                            .map(|e| self.get_text(&e).trim().to_string())
                            .filter(|t| !t.is_empty())
                            .collect();
                        if !cells.is_empty() {
                            items.push(cells.join(" | "));
                        }
                    }
                }
            }
            "a" => {
                if let Some(href) = element.value().attr("href") {
                    let text = self.get_text(element).trim().to_string();
                    let url = self.resolve_url(href);
                    if !text.is_empty() && text != url {
                        items.push(format!("[{}]({})", text, url));
                    } else if url.starts_with("http") {
                        items.push(url);
                    }
                }
            }
            "img" => {
                if let Some(src) = element.value().attr("src") {
                    let alt = element.value().attr("alt").unwrap_or("image");
                    let url = self.resolve_url(src);
                    items.push(format!("![{}]({})", alt, url));
                }
            }
            "br" => {
                items.push("".to_string());
            }
            "hr" => {
                items.push("---".to_string());
            }
            "div" | "section" | "article" | "main" | "span" | "body"
            | "tbody" | "thead" | "tfoot" | "tr" => {
                for child in element.child_elements() {
                    self.extract_recursive(&child, items);
                }
            }
            _ => {
                // If element contains block-level children, recurse rather than
                // flattening all descendant text into one blob (e.g. custom elements
                // like <markdown-accessiblity-table> that wrap tables/headings).
                let has_block_children = element.child_elements().any(|e| {
                    matches!(e.value().name(),
                        "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
                        | "ul" | "ol" | "table" | "pre" | "blockquote"
                        | "div" | "section" | "article" | "main"
                    )
                });
                if has_block_children {
                    for child in element.child_elements() {
                        self.extract_recursive(&child, items);
                    }
                } else {
                    let text = self.get_text(element).trim().to_string();
                    if !text.is_empty() && text.len() > 2 {
                        items.push(text);
                    }
                }
            }
        }
    }

    /// Get text from element, handling nested formatting
    fn get_text(&self, element: &scraper::ElementRef) -> String {
        let mut result = String::new();
        let mut last_was_text = false;

        for node in element.descendants() {
            if let Some(elem) = scraper::ElementRef::wrap(node.clone()) {
                let tag = elem.value().name();
                
                match tag {
                    "script" | "style" | "noscript" | "iframe" | "svg" | "canvas" => continue,
                    "br" => {
                        result.push(' ');
                        last_was_text = false;
                    }
                    "a" => {
                        let text = elem.text().collect::<String>().trim().to_string();
                        if !text.is_empty() {
                            if last_was_text {
                                result.push(' ');
                            }
                            result.push_str(&text);
                            last_was_text = true;
                        }
                    }
                    "strong" | "b" => {
                        let text = elem.text().collect::<String>().trim().to_string();
                        if !text.is_empty() {
                            result.push_str(&format!("**{}**", text));
                            last_was_text = true;
                        }
                    }
                    "em" | "i" => {
                        let text = elem.text().collect::<String>().trim().to_string();
                        if !text.is_empty() {
                            result.push_str(&format!("*{}*", text));
                            last_was_text = true;
                        }
                    }
                    "code" => {
                        // Check if parent is pre
                        let is_pre = elem.parent()
                            .and_then(|p| p.value().as_element())
                            .map(|e| e.name() == "pre")
                            .unwrap_or(false);
                        if !is_pre {
                            let text = elem.text().collect::<String>().trim().to_string();
                            if !text.is_empty() {
                                result.push_str(&format!("`{}`", text));
                                last_was_text = true;
                            }
                        }
                    }
                    _ => {
                        let text = elem.text().collect::<String>();
                        if !text.trim().is_empty() {
                            if last_was_text && !result.ends_with(' ') && !result.ends_with('\n') {
                                result.push(' ');
                            }
                            result.push_str(&text);
                            last_was_text = true;
                        }
                    }
                }
            }
        }

        // Normalize whitespace
        let re = Regex::new(r"\s+").unwrap();
        re.replace_all(&result, " ").to_string()
    }

    fn get_lang(&self, element: &scraper::ElementRef) -> String {
        // Check code element inside
        if let Ok(sel) = Selector::parse("code") {
            if let Some(code) = element.select(&sel).next() {
                if let Some(class) = code.value().attr("class") {
                    for pat in &["language-", "lang-", "prism-language-", "hljs-"] {
                        if let Some(pos) = class.find(pat) {
                            let start = pos + pat.len();
                            let end = class[start..].find(|c: char| !c.is_alphanumeric())
                                .map(|i| start + i)
                                .unwrap_or(class.len());
                            return class[start..end].to_lowercase();
                        }
                    }
                }
            }
        }
        // Check pre class
        if let Some(class) = element.value().attr("class") {
            for pat in &["language-", "lang-"] {
                if let Some(pos) = class.find(pat) {
                    let start = pos + pat.len();
                    let end = class[start..].find(|c: char| !c.is_alphanumeric())
                        .map(|i| start + i)
                        .unwrap_or(class.len());
                    return class[start..end].to_lowercase();
                }
            }
        }
        String::new()
    }

    fn resolve_url(&self, href: &str) -> String {
        if href.starts_with("javascript:") || href.starts_with("mailto:") {
            return String::new();
        }
        if href.starts_with('#') {
            return href.to_string();
        }
        match self.base_url.join(href) {
            Ok(u) => u.to_string(),
            Err(_) => href.to_string(),
        }
    }

    /// Normalize text for deduplication comparison (strip markdown, lowercase, collapse spaces)
    fn normalize_for_dedup(&self, s: &str) -> String {
        let s = Regex::new(r"\*\*([^*]+)\*\*").map(|re| re.replace_all(s, "$1").to_string()).unwrap_or_else(|_| s.to_string());
        let s = Regex::new(r"`([^`]+)`").map(|re| re.replace_all(&s, "$1").to_string()).unwrap_or_else(|_| s.clone());
        let s = Regex::new(r"\[([^\]]+)\]\([^)]+\)").map(|re| re.replace_all(&s, "$1").to_string()).unwrap_or_else(|_| s.clone());
        s.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
    }

    /// Aggressive deduplication
    fn deduplicate_content(&self, content: &[String]) -> String {
        let mut result: Vec<String> = Vec::new();
        let mut seen_normalized: std::collections::HashSet<String> = std::collections::HashSet::new();

        for item in content {
            let item = item.trim();
            if item.is_empty() {
                continue;
            }

            // Skip common navigation/boilerplate phrases
            let lower = item.to_lowercase();
            if lower.contains("log in") || lower.contains("sign up") ||
               lower.contains("terms of service") || lower.contains("privacy policy") ||
               lower.contains("cookie use") || lower.contains("ads info") ||
               lower.contains("create account") || lower.contains("upgrade to premium") ||
               lower.contains("new to x") || lower.contains("skip to content") ||
               lower.contains("you must be signed in") {
                continue;
            }

            let normalized = self.normalize_for_dedup(item);

            // Skip if we've seen this normalized form before
            if seen_normalized.contains(&normalized) {
                continue;
            }

            // Skip if very similar to any recent item
            let too_similar = result.iter()
                .rev()
                .take(10)
                .any(|prev: &String| self.strings_similar(prev, item));

            if too_similar {
                continue;
            }

            seen_normalized.insert(normalized);
            result.push(item.to_string());
        }

        self.format_output(&result)
    }

    /// Check if two strings are suspiciously similar
    fn strings_similar(&self, a: &str, b: &str) -> bool {
        // If one contains the other and they're close in length
        let len_diff = if a.len() > b.len() { a.len() - b.len() } else { b.len() - a.len() };
        if len_diff < 5 && (a.contains(b) || b.contains(a)) {
            return true;
        }

        // If both are very short and equal
        if a.len() < 10 && a == b {
            return true;
        }

        // Check for repeated content pattern (e.g., "texttexttext" vs "text")
        let a_base: String = a.chars().filter(|c| !c.is_whitespace()).collect();
        let b_base: String = b.chars().filter(|c| !c.is_whitespace()).collect();
        if (is_repeated_pattern(&a_base) || is_repeated_pattern(&b_base)) && a_base.len() > 5 {
            return true;
        }

        false
    }

    /// Format output with proper spacing
    fn format_output(&self, lines: &[String]) -> String {
        let mut result = String::new();
        let mut last_was_heading = false;
        let mut last_was_code = false;

        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let is_heading = trimmed.starts_with('#');
            let is_code = trimmed.starts_with("```");
            let is_blockquote = trimmed.starts_with('>');
            let is_list = trimmed.starts_with('-');

            // Add spacing before certain elements
            if !result.is_empty() {
                if is_heading || (last_was_code && !is_code) {
                    result.push_str("\n\n");
                } else if is_list && !result.ends_with('\n') && !result.ends_with("\n\n") {
                    result.push('\n');
                } else if !result.ends_with(' ') && !result.ends_with('\n') {
                    result.push(' ');
                }
            }

            result.push_str(trimmed);

            last_was_heading = is_heading;
            last_was_code = is_code;
        }

        result.trim().to_string()
    }

    fn select_one<'a>(&self, document: &'a Html, selector: &str) -> Option<scraper::ElementRef<'a>> {
        Selector::parse(selector).ok().and_then(|s| document.select(&s).next())
    }
}

/// Returns true if `s` consists of a short substring repeated 3+ times.
fn is_repeated_pattern(s: &str) -> bool {
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    for unit in 1..=(len / 3) {
        if len % unit == 0 {
            let pattern: String = chars[..unit].iter().collect();
            if pattern.repeat(len / unit) == s {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        let c = Converter::new("https://example.com").unwrap();
        assert!(c.convert("<h1>Title</h1><p>Hello</p>").is_ok());
    }
}
