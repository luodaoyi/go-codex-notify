use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

pub fn render(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);

    let mut renderer = Renderer::default();
    for event in Parser::new_ext(markdown, options) {
        renderer.render_event(event);
    }
    renderer.finish()
}

#[derive(Default)]
struct Renderer {
    output: String,
    quote_buffers: Vec<String>,
    link_targets: Vec<String>,
    list_stack: Vec<Option<u64>>,
    strong_markers: Vec<bool>,
    heading_depth: usize,
    item_depth: usize,
    code_block_depth: usize,
    table_first_cells: Vec<bool>,
}

impl Renderer {
    fn render_event(&mut self, event: Event<'_>) {
        match event {
            Event::Start(tag) => self.start_tag(tag),
            Event::End(tag) => self.end_tag(tag),
            Event::Text(text) => {
                if self.code_block_depth > 0 {
                    self.push(&escape_code(&text));
                } else {
                    self.push(&escape_text(&text));
                }
            }
            Event::Code(code) => {
                self.push("`");
                self.push(&escape_code(&code));
                self.push("`");
            }
            Event::Html(html) | Event::InlineHtml(html) => self.push(&escape_text(&html)),
            Event::FootnoteReference(reference) => {
                self.push(&escape_text(&format!("[{reference}]")));
            }
            Event::SoftBreak | Event::HardBreak => self.ensure_newlines(1),
            Event::Rule => {
                self.ensure_newlines(2);
                self.push("────────");
                self.ensure_newlines(2);
            }
            Event::TaskListMarker(checked) => {
                self.push(if checked { "☑ " } else { "☐ " });
            }
            _ => {}
        }
    }

    fn start_tag(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Paragraph => {}
            Tag::Heading { .. } => {
                self.ensure_block_boundary();
                self.heading_depth += 1;
                self.push("*");
            }
            Tag::BlockQuote(_) => {
                self.ensure_block_boundary();
                self.quote_buffers.push(String::new());
            }
            Tag::CodeBlock(_) => {
                self.ensure_block_boundary();
                self.push("```");
                // Telegram accepts a language after the fence, but omitting arbitrary CommonMark
                // info strings avoids introducing unescaped syntax into the fence header.
                self.push("\n");
                self.code_block_depth += 1;
            }
            Tag::List(start) => {
                self.ensure_block_boundary();
                self.list_stack.push(start);
            }
            Tag::Item => {
                self.ensure_newlines(1);
                let marker = match self.list_stack.last_mut() {
                    Some(Some(next)) => {
                        let marker = format!("{next}\\. ");
                        *next += 1;
                        marker
                    }
                    _ => "• ".to_owned(),
                };
                self.push(&marker);
                self.item_depth += 1;
            }
            Tag::Emphasis => self.push("_"),
            Tag::Strong => {
                let emit = self.heading_depth == 0 && !self.strong_markers.contains(&true);
                self.strong_markers.push(emit);
                if emit {
                    self.push("*");
                }
            }
            Tag::Strikethrough => self.push("~"),
            Tag::Link { dest_url, .. } | Tag::Image { dest_url, .. } => {
                self.push("[");
                self.link_targets.push(dest_url.into_string());
            }
            Tag::Table(_) => self.ensure_block_boundary(),
            Tag::TableRow => {
                self.ensure_newlines(1);
                self.table_first_cells.push(true);
            }
            Tag::TableCell => {
                if self.table_first_cells.last().is_some_and(|first| !first) {
                    self.push(" │ ");
                }
                if let Some(first) = self.table_first_cells.last_mut() {
                    *first = false;
                }
            }
            _ => {}
        }
    }

    fn end_tag(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph => self.end_paragraph(),
            TagEnd::Heading(_) => {
                self.push("*");
                self.heading_depth = self.heading_depth.saturating_sub(1);
                self.ensure_newlines(2);
            }
            TagEnd::BlockQuote(_) => self.end_block_quote(),
            TagEnd::CodeBlock => {
                self.ensure_newlines(1);
                self.push("```");
                self.code_block_depth = self.code_block_depth.saturating_sub(1);
                self.ensure_newlines(2);
            }
            TagEnd::List(_) => {
                self.list_stack.pop();
                self.ensure_block_boundary();
            }
            TagEnd::Item => {
                self.item_depth = self.item_depth.saturating_sub(1);
                self.ensure_newlines(1);
            }
            TagEnd::Emphasis => self.push("_"),
            TagEnd::Strong => self.end_strong(),
            TagEnd::Strikethrough => self.push("~"),
            TagEnd::Link | TagEnd::Image => {
                let target = self.link_targets.pop().unwrap_or_default();
                self.push("](");
                self.push(&escape_link_target(&target));
                self.push(")");
            }
            TagEnd::Table => self.ensure_block_boundary(),
            TagEnd::TableRow => {
                self.table_first_cells.pop();
                self.ensure_newlines(1);
            }
            _ => {}
        }
    }

    fn end_block_quote(&mut self) {
        let Some(content) = self.quote_buffers.pop() else {
            return;
        };
        let content = content.trim_matches('\n');
        let quoted = content
            .split('\n')
            .map(|line| format!(">{line}"))
            .collect::<Vec<_>>()
            .join("\n");
        self.push(&quoted);
        self.ensure_newlines(2);
    }

    fn end_paragraph(&mut self) {
        if self.item_depth == 0 {
            self.ensure_newlines(2);
        }
    }

    fn end_strong(&mut self) {
        if self.strong_markers.pop().unwrap_or(false) {
            self.push("*");
        }
    }

    fn ensure_block_boundary(&mut self) {
        self.ensure_newlines(if self.item_depth == 0 { 2 } else { 1 });
    }

    fn ensure_newlines(&mut self, count: usize) {
        if self.target().is_empty() {
            return;
        }
        let existing = self
            .target()
            .chars()
            .rev()
            .take_while(|character| *character == '\n')
            .count();
        for _ in existing..count {
            self.target().push('\n');
        }
    }

    fn push(&mut self, value: &str) {
        self.target().push_str(value);
    }

    fn target(&mut self) -> &mut String {
        if let Some(buffer) = self.quote_buffers.last_mut() {
            buffer
        } else {
            &mut self.output
        }
    }

    fn finish(self) -> String {
        self.output.trim_matches('\n').to_owned()
    }
}

fn escape_text(value: &str) -> String {
    escape_characters(value, "\\_*[]()~`>#+-=|{}.!")
}

fn escape_code(value: &str) -> String {
    escape_characters(value, "\\`")
}

fn escape_link_target(value: &str) -> String {
    escape_characters(value, "\\)")
}

fn escape_characters(value: &str, reserved: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        if reserved.contains(character) {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_headings_lists_links_and_inline_styles() {
        let input =
            "# 结果\n\n- **成功**：见 [文档](https://example.com/a_b).\n- *强调*、~~删除~~和 `x_y`";

        assert_eq!(
            render(input),
            "*结果*\n\n• *成功*：见 [文档](https://example.com/a_b)\\.\n• _强调_、~删除~和 `x_y`"
        );
    }

    #[test]
    fn renders_code_blocks_without_escaping_code_punctuation() {
        let input = "```rust\nlet x = 1; // !\nprintln!(\"`{x}`\");\n```";

        assert_eq!(
            render(input),
            "```\nlet x = 1; // !\nprintln!(\"\\`{x}\\`\");\n```"
        );
    }

    #[test]
    fn escapes_plain_text_and_preserves_block_quotes() {
        let input = "普通：(ok) [raw] {x} #tag +a=b | c ~ d > e - f _ g . ! \\\n\n> 引用 **重点**";

        assert_eq!(
            render(input),
            "普通：\\(ok\\) \\[raw\\] \\{x\\} \\#tag \\+a\\=b \\| c \\~ d \\> e \\- f \\_ g \\. \\! \\\\\n\n>引用 *重点*"
        );
    }
}
