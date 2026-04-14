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
            .or_else(|| self.select_one(&document, "#mw-content-text"))   // Wikipedia
            .or_else(|| self.select_one(&document, ".mw-parser-output"))  // Wikipedia
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
            // Wikipedia-specific
            ".mw-editsection", ".mw-jump-link", ".catlinks", ".navbox",
            ".reflist", ".refbegin", "#toc", ".toc", ".infobox",
            ".sidebar", ".hatnote", ".mw-references-wrap",
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
                let text = self.get_heading_text(element).trim().to_string();
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
                // Collect direct rows only (through tbody/thead/tfoot but not nested tables)
                let mut rows: Vec<scraper::ElementRef> = Vec::new();
                for child in element.child_elements() {
                    match child.value().name() {
                        "tbody" | "thead" | "tfoot" => {
                            for row in child.child_elements() {
                                if row.value().name() == "tr" {
                                    rows.push(row);
                                }
                            }
                        }
                        "tr" => rows.push(child),
                        _ => {}
                    }
                }
                for row in rows {
                    let cells: Vec<scraper::ElementRef> = row.child_elements()
                        .filter(|e| matches!(e.value().name(), "td" | "th"))
                        .collect();

                    // If any cell contains a nested table, this is a layout table — recurse into it
                    let is_layout = cells.iter().any(|cell| {
                        cell.child_elements().any(|c| c.value().name() == "table"
                            || matches!(c.value().name(), "div" | "section" | "article" | "main"))
                    });

                    if is_layout {
                        for cell in cells {
                            self.extract_recursive(&cell, items);
                        }
                    } else {
                        let texts: Vec<String> = cells.iter()
                            .map(|e| self.get_text_no_tables(e).trim().to_string())
                            .filter(|t| !t.is_empty())
                            .collect();
                        if !texts.is_empty() {
                            items.push(texts.join(" | "));
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
            // Table structure elements — always recurse, never extract as text
            "tbody" | "thead" | "tfoot" | "tr" => {
                for child in element.child_elements() {
                    self.extract_recursive(&child, items);
                }
            }
            _ => {
                // If element contains block-level children, recurse rather than
                // flattening all descendant text into one blob (e.g. custom elements
                // like <markdown-accessiblity-table> that wrap tables/headings).
                // Recurse if ANY child is not a purely inline element.
                // Whitelist inline elements; everything else (div, center, table, etc.) triggers recursion.
                let is_inline = |name: &str| matches!(name,
                    "a" | "span" | "strong" | "b" | "em" | "i" | "code" | "small"
                    | "sup" | "sub" | "abbr" | "cite" | "time" | "mark" | "kbd"
                    | "br" | "img" | "button" | "label" | "input" | "select" | "wbr"
                );
                let has_block_children = element.child_elements()
                    .any(|e| !is_inline(e.value().name()));
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

    /// Like get_text but skips nested table elements (used for table cell extraction)
    fn get_text_no_tables(&self, element: &scraper::ElementRef) -> String {
        let mut parts = Vec::new();
        for node in element.descendants() {
            if let Some(elem) = scraper::ElementRef::wrap(node.clone()) {
                match elem.value().name() {
                    "table" | "tbody" | "thead" | "tfoot" | "tr" | "td" | "th" => continue,
                    "script" | "style" | "noscript" => continue,
                    _ => {}
                }
            } else if let Some(text) = node.value().as_text() {
                // Skip if any ancestor is a nested table
                let in_nested_table = node.ancestors().skip(1).any(|a| {
                    a.value().as_element()
                        .map(|e| matches!(e.name(), "table" | "tbody" | "thead" | "tr" | "td" | "th"))
                        .unwrap_or(false)
                        && a.id() != element.id()
                });
                if !in_nested_table {
                    let t = text.trim();
                    if !t.is_empty() {
                        parts.push(t.to_string());
                    }
                }
            }
        }
        let text = parts.join(" ");
        let re = Regex::new(r"\s+").unwrap();
        // Strip citation refs
        let text = Regex::new(r"\[\s*(?:note\s+)?[\w\d]+\s*\]|\[\s*\]")
            .map(|re| re.replace_all(&text, "").to_string())
            .unwrap_or(text);
        re.replace_all(text.trim(), " ").to_string()
    }

    /// Get heading text, stripping [edit] links and similar noise
    fn get_heading_text(&self, element: &scraper::ElementRef) -> String {
        // Collect text from all non-edit children
        let mut parts = Vec::new();
        for node in element.descendants() {
            if let Some(text_node) = node.value().as_text() {
                let t = text_node.trim();
                if !t.is_empty() && t != "edit" && t != "[edit]" {
                    parts.push(t.to_string());
                }
            } else if let Some(elem) = scraper::ElementRef::wrap(node) {
                // Skip <a> tags that are edit links
                if elem.value().name() == "a" {
                    let href = elem.value().attr("href").unwrap_or("");
                    let text = elem.text().collect::<String>();
                    if href.contains("action=edit") || text.trim() == "edit" {
                        continue;
                    }
                }
            }
        }
        // Deduplicate consecutive identical words (heading text can be doubled in some parsers)
        let text = parts.join(" ");
        let re = Regex::new(r"\s+").unwrap();
        re.replace_all(text.trim(), " ").to_string()
    }

    /// Get text from element — proper child-by-child traversal to avoid double-counting.
    /// Parent elements and their text children are both visited by `descendants()`, so
    /// using it causes text inside <a>/<strong>/etc to appear twice.
    fn get_text(&self, element: &scraper::ElementRef) -> String {
        let mut result = String::new();
        self.collect_text(element, &mut result);
        // Normalize whitespace
        let re = Regex::new(r"\s+").unwrap();
        let result = re.replace_all(result.trim(), " ").to_string();
        // Strip citation refs: [1], [16], [note 4], [ ]
        Regex::new(r"\[\s*(?:note\s+)?[\w\d]+\s*\]|\[\s*\]")
            .map(|re| re.replace_all(&result, "").to_string())
            .unwrap_or(result)
    }

    /// Recursive helper — iterates direct children only, then recurses.
    fn collect_text(&self, element: &scraper::ElementRef, out: &mut String) {
        for node in element.children() {
            if let Some(text) = node.value().as_text() {
                out.push_str(text);
            } else if let Some(elem) = scraper::ElementRef::wrap(node) {
                match elem.value().name() {
                    "script" | "style" | "noscript" | "iframe" | "svg" | "canvas" => {}
                    "br" => out.push(' '),
                    "strong" | "b" => {
                        let mut inner = String::new();
                        self.collect_text(&elem, &mut inner);
                        let inner = inner.trim().to_string();
                        if !inner.is_empty() { out.push_str(&format!("**{}**", inner)); }
                    }
                    "em" | "i" => {
                        let mut inner = String::new();
                        self.collect_text(&elem, &mut inner);
                        let inner = inner.trim().to_string();
                        if !inner.is_empty() { out.push_str(&format!("*{}*", inner)); }
                    }
                    "code" => {
                        let is_pre = elem.parent()
                            .and_then(|p| p.value().as_element())
                            .map(|e| e.name() == "pre")
                            .unwrap_or(false);
                        if !is_pre {
                            let text: String = elem.text().collect();
                            let text = text.trim().to_string();
                            if !text.is_empty() { out.push_str(&format!("`{}`", text)); }
                        }
                    }
                    _ => self.collect_text(&elem, out),
                }
            }
        }
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
               lower.contains("you must be signed in") ||
               lower.contains("action=edit") || lower == "edit" || lower == "[edit]" {
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

        // Word overlap: if 80%+ of the shorter string's words appear in the longer, it's a dupe
        let a_words: std::collections::HashSet<&str> = a.split_whitespace().collect();
        let b_words: std::collections::HashSet<&str> = b.split_whitespace().collect();
        let min_words = a_words.len().min(b_words.len());
        if min_words >= 5 {
            let overlap = a_words.intersection(&b_words).count();
            if overlap as f64 / min_words as f64 >= 0.8 {
                return true;
            }
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
        let mut last_was_code = false;

        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let is_heading = trimmed.starts_with('#');
            let is_code = trimmed.starts_with("```");
            let is_list = trimmed.starts_with('-');

            if !result.is_empty() {
                if is_heading || is_code || last_was_code {
                    result.push_str("\n\n");
                } else if is_list {
                    result.push('\n');
                } else {
                    result.push_str("\n\n");
                }
            }

            result.push_str(trimmed);
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
