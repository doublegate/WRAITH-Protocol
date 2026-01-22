//! Markdown Processing
//!
//! Parses and renders markdown content with sanitization.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd, html};
use serde::{Deserialize, Serialize};

/// Markdown rendering options
#[derive(Debug, Clone, Default)]
pub struct RenderOptions {
    /// Enable strikethrough
    pub strikethrough: bool,
    /// Enable tables
    pub tables: bool,
    /// Enable footnotes
    pub footnotes: bool,
    /// Enable task lists
    pub task_lists: bool,
}

/// Markdown processor
pub struct MarkdownProcessor {
    options: RenderOptions,
}

impl Default for MarkdownProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownProcessor {
    /// Create a new markdown processor with default options
    pub fn new() -> Self {
        Self {
            options: RenderOptions {
                strikethrough: true,
                tables: true,
                footnotes: true,
                task_lists: true,
            },
        }
    }

    /// Create with custom options
    pub fn with_options(options: RenderOptions) -> Self {
        Self { options }
    }

    /// Convert markdown to HTML
    pub fn to_html(&self, markdown: &str) -> String {
        let mut pulldown_options = Options::empty();

        if self.options.strikethrough {
            pulldown_options.insert(Options::ENABLE_STRIKETHROUGH);
        }
        if self.options.tables {
            pulldown_options.insert(Options::ENABLE_TABLES);
        }
        if self.options.footnotes {
            pulldown_options.insert(Options::ENABLE_FOOTNOTES);
        }
        if self.options.task_lists {
            pulldown_options.insert(Options::ENABLE_TASKLISTS);
        }

        let parser = Parser::new_ext(markdown, pulldown_options);
        let mut html_output = String::new();
        html::push_html(&mut html_output, parser);

        html_output
    }

    /// Extract title from markdown (first H1 heading)
    pub fn extract_title(&self, markdown: &str) -> Option<String> {
        let parser = Parser::new(markdown);
        let mut in_heading = false;
        let mut title = String::new();

        for event in parser {
            match event {
                Event::Start(Tag::Heading {
                    level: pulldown_cmark::HeadingLevel::H1,
                    ..
                }) => {
                    in_heading = true;
                }
                Event::End(TagEnd::Heading(pulldown_cmark::HeadingLevel::H1)) => {
                    if !title.is_empty() {
                        return Some(title);
                    }
                    in_heading = false;
                }
                Event::Text(text) if in_heading => {
                    title.push_str(&text);
                }
                _ => {}
            }
        }

        if !title.is_empty() { Some(title) } else { None }
    }

    /// Extract excerpt (first paragraph, truncated)
    pub fn extract_excerpt(&self, markdown: &str, max_length: usize) -> String {
        let parser = Parser::new(markdown);
        let mut in_paragraph = false;
        let mut excerpt = String::new();

        for event in parser {
            match event {
                Event::Start(Tag::Paragraph) => {
                    in_paragraph = true;
                }
                Event::End(TagEnd::Paragraph) => {
                    if !excerpt.is_empty() {
                        break;
                    }
                    in_paragraph = false;
                }
                Event::Text(text) if in_paragraph => {
                    excerpt.push_str(&text);
                }
                Event::SoftBreak | Event::HardBreak if in_paragraph => {
                    excerpt.push(' ');
                }
                _ => {}
            }
        }

        // Truncate if necessary
        if excerpt.len() > max_length {
            let truncated = &excerpt[..max_length];
            // Find last space to avoid cutting words
            if let Some(last_space) = truncated.rfind(' ') {
                format!("{}...", &truncated[..last_space])
            } else {
                format!("{}...", truncated)
            }
        } else {
            excerpt
        }
    }

    /// Count words in markdown content
    pub fn word_count(&self, markdown: &str) -> usize {
        let parser = Parser::new(markdown);
        let mut count = 0;

        for event in parser {
            if let Event::Text(text) = event {
                count += text.split_whitespace().count();
            }
        }

        count
    }

    /// Estimate reading time in minutes
    pub fn reading_time(&self, markdown: &str) -> u32 {
        let words = self.word_count(markdown);
        // Average reading speed: 200 words per minute
        let minutes = (words as f64 / 200.0).ceil() as u32;
        std::cmp::max(1, minutes)
    }

    /// Extract all headings from markdown
    pub fn extract_headings(&self, markdown: &str) -> Vec<Heading> {
        let parser = Parser::new(markdown);
        let mut headings = Vec::new();
        let mut current_level = 0u8;
        let mut current_text = String::new();

        for event in parser {
            match event {
                Event::Start(Tag::Heading { level, .. }) => {
                    current_level = match level {
                        pulldown_cmark::HeadingLevel::H1 => 1,
                        pulldown_cmark::HeadingLevel::H2 => 2,
                        pulldown_cmark::HeadingLevel::H3 => 3,
                        pulldown_cmark::HeadingLevel::H4 => 4,
                        pulldown_cmark::HeadingLevel::H5 => 5,
                        pulldown_cmark::HeadingLevel::H6 => 6,
                    };
                }
                Event::End(TagEnd::Heading(_)) => {
                    if !current_text.is_empty() {
                        headings.push(Heading {
                            level: current_level,
                            text: std::mem::take(&mut current_text),
                        });
                    }
                    current_level = 0;
                }
                Event::Text(text) if current_level > 0 => {
                    current_text.push_str(&text);
                }
                _ => {}
            }
        }

        headings
    }
}

/// A heading extracted from markdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heading {
    pub level: u8,
    pub text: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_to_html() {
        let processor = MarkdownProcessor::new();

        let markdown = "# Hello World\n\nThis is a **test**.";
        let html = processor.to_html(markdown);

        assert!(html.contains("<h1>Hello World</h1>"));
        assert!(html.contains("<strong>test</strong>"));
    }

    #[test]
    fn test_extract_title() {
        let processor = MarkdownProcessor::new();

        let markdown = "# My Article Title\n\nSome content here.";
        let title = processor.extract_title(markdown);

        assert_eq!(title, Some("My Article Title".to_string()));
    }

    #[test]
    fn test_extract_title_no_heading() {
        let processor = MarkdownProcessor::new();

        let markdown = "Just some content without a heading.";
        let title = processor.extract_title(markdown);

        assert_eq!(title, None);
    }

    #[test]
    fn test_extract_excerpt() {
        let processor = MarkdownProcessor::new();

        let markdown =
            "# Title\n\nThis is the first paragraph with some content.\n\nSecond paragraph.";
        let excerpt = processor.extract_excerpt(markdown, 30);

        assert!(excerpt.contains("This is the first"));
        assert!(excerpt.ends_with("..."));
    }

    #[test]
    fn test_word_count() {
        let processor = MarkdownProcessor::new();

        let markdown = "# Title\n\nOne two three four five.\n\n## Another\n\nSix seven eight.";
        let count = processor.word_count(markdown);

        assert_eq!(count, 10); // Title + One two three four five + Another + Six seven eight
    }

    #[test]
    fn test_reading_time() {
        let processor = MarkdownProcessor::new();

        // 400 words should be ~2 minutes
        let markdown = "word ".repeat(400);
        let time = processor.reading_time(&markdown);

        assert_eq!(time, 2);
    }

    #[test]
    fn test_extract_headings() {
        let processor = MarkdownProcessor::new();

        let markdown = "# Title\n\n## Section 1\n\n### Subsection\n\n## Section 2";
        let headings = processor.extract_headings(markdown);

        assert_eq!(headings.len(), 4);
        assert_eq!(headings[0].level, 1);
        assert_eq!(headings[0].text, "Title");
        assert_eq!(headings[1].level, 2);
        assert_eq!(headings[1].text, "Section 1");
        assert_eq!(headings[2].level, 3);
        assert_eq!(headings[2].text, "Subsection");
    }

    #[test]
    fn test_tables() {
        let processor = MarkdownProcessor::new();

        let markdown = "| Header 1 | Header 2 |\n|----------|----------|\n| Cell 1   | Cell 2   |";
        let html = processor.to_html(markdown);

        assert!(html.contains("<table>"));
        assert!(html.contains("<th>Header 1</th>"));
        assert!(html.contains("<td>Cell 1</td>"));
    }

    #[test]
    fn test_strikethrough() {
        let processor = MarkdownProcessor::new();

        let markdown = "This is ~~deleted~~ text.";
        let html = processor.to_html(markdown);

        assert!(html.contains("<del>deleted</del>"));
    }

    #[test]
    fn test_task_lists() {
        let processor = MarkdownProcessor::new();

        let markdown = "- [ ] Todo item\n- [x] Done item";
        let html = processor.to_html(markdown);

        assert!(html.contains("type=\"checkbox\""));
        assert!(html.contains("checked"));
    }
}
