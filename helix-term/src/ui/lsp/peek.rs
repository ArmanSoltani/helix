use std::path::PathBuf;
use std::sync::Arc;

use crate::compositor::{Component, Context, EventResult};
use crate::key;
use crate::ui::Markdown;
use arc_swap::ArcSwap;
use helix_core::syntax;
use helix_lsp::lsp;
use helix_view::editor::Action;
use helix_view::graphics::{Margin, Rect};
use helix_view::input::Event;
use tokio::time::Instant;
use tui::buffer::Buffer;
use tui::text::{Span, Text};
use tui::widgets::{Paragraph, Widget, Wrap};

pub struct PeekDefinition {
    document_position: lsp::Position,
    document_path: PathBuf,
    offset_encoding: helix_lsp::OffsetEncoding,
    markdown_content: Markdown,
    file_path: String,
}

impl PeekDefinition {
    pub const ID: &'static str = "peek-definition";

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        document_position: lsp::Position,
        document_path: PathBuf,
        offset_encoding: helix_lsp::OffsetEncoding,
        lines: Vec<String>,
        language: String,
        file_path: String,
        config_loader: Arc<ArcSwap<syntax::Loader>>,
    ) -> Self {
        let markdown = Markdown::new(
            format!("```{}\n{}```", language, lines.join("")),
            config_loader,
        );

        Self {
            document_position,
            document_path,
            offset_encoding,
            markdown_content: markdown,
            file_path,
        }
    }

    pub fn jump_to_definition(&self, cx: &mut Context) {
        let range = lsp::Range::new(self.document_position, self.document_position);
        crate::commands::lsp::jump_to_position(
            cx.editor,
            &self.document_path,
            range,
            self.offset_encoding,
            Action::Replace,
        );
    }
}

// Constants for padding and layout, matching the hover component style
const PADDING_HORIZONTAL: u16 = 2;
const PADDING_TOP: u16 = 1;
const PADDING_BOTTOM: u16 = 1;
const HEADER_HEIGHT: u16 = 1;
const SEPARATOR_HEIGHT: u16 = 1;

impl Component for PeekDefinition {
    fn render(&mut self, area: Rect, surface: &mut Buffer, cx: &mut Context) {
        let start = Instant::now();

        let margin = Margin::all(1);
        let inner_area = area.inner(margin);

        // Create header
        let header_style = cx.editor.theme.get("ui.text.info");
        let header = Text::from(Span::styled(&self.file_path, header_style));
        let header_para = Paragraph::new(&header);
        header_para.render(inner_area.with_height(HEADER_HEIGHT), surface);

        // Set up content area
        let content_area = inner_area.clip_top(HEADER_HEIGHT + SEPARATOR_HEIGHT);

        // Parse and render the Markdown content
        let contents = self.markdown_content.parse(Some(&cx.editor.theme));

        let contents_para = Paragraph::new(&contents)
            .wrap(Wrap { trim: false })
            .scroll((cx.scroll.unwrap_or_default() as u16, 0));
        contents_para.render(content_area, surface);

        let duration = start.elapsed();
        log::info!("PEEK PERF rendering: {:?}ms", duration.as_millis());
    }

    fn handle_event(&mut self, event: &Event, cx: &mut Context) -> EventResult {
        let Event::Key(event) = event else {
            return EventResult::Ignored(None);
        };

        if let key!(Enter) = event {
            self.jump_to_definition(cx);
        }
        EventResult::Ignored(None)
    }

    fn id(&self) -> Option<&'static str> {
        Some(Self::ID)
    }

    fn required_size(&mut self, viewport: (u16, u16)) -> Option<(u16, u16)> {
        let max_text_width = viewport.0.saturating_sub(PADDING_HORIZONTAL).clamp(10, 120);

        // Parse the markdown content to calculate its size
        let contents = self.markdown_content.parse(None);
        let (content_width, content_height) =
            crate::ui::text::required_size(&contents, max_text_width);

        // We always have a header with path
        let width = PADDING_HORIZONTAL + content_width;
        let height =
            PADDING_TOP + HEADER_HEIGHT + SEPARATOR_HEIGHT + content_height + PADDING_BOTTOM;

        Some((width.min(120), height.min(30)))
    }
}
