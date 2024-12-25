#[macro_use]
pub mod macros;

pub mod annotations;
pub mod base64;
pub mod clipboard;
pub mod document;
pub mod editor;
pub mod events;
pub mod expansion;
pub mod graphics;
pub mod gutter;
pub mod handlers;
pub mod info;
pub mod input;
pub mod keyboard;
pub mod register;
pub mod theme;
pub mod tree;
pub mod view;

use std::{cell::RefCell, collections::HashMap, num::NonZeroUsize, path::PathBuf};

// uses NonZeroUsize so Option<DocumentId> use a byte rather than two
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct DocumentId(NonZeroUsize);

impl Default for DocumentId {
    fn default() -> DocumentId {
        // Safety: 1 is non-zero
        DocumentId(unsafe { NonZeroUsize::new_unchecked(1) })
    }
}

impl std::fmt::Display for DocumentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.0))
    }
}

slotmap::new_key_type! {
    pub struct ViewId;
}

pub enum Align {
    Top,
    Center,
    Bottom,
}

pub fn align_view(doc: &mut Document, view: &View, align: Align) {
    let doc_text = doc.text().slice(..);
    let cursor = doc.selection(view.id).primary().cursor(doc_text);
    let viewport = view.inner_area(doc);
    let last_line_height = viewport.height.saturating_sub(1);
    let mut view_offset = doc.view_offset(view.id);

    let relative = match align {
        Align::Center => last_line_height / 2,
        Align::Top => 0,
        Align::Bottom => last_line_height,
    };

    let text_fmt = doc.text_format(viewport.width, None);
    (view_offset.anchor, view_offset.vertical_offset) = char_idx_at_visual_offset(
        doc_text,
        cursor,
        -(relative as isize),
        0,
        &text_fmt,
        &view.text_annotations(doc, None),
    );
    doc.set_view_offset(view.id, view_offset);
}

pub use document::Document;
pub use editor::Editor;
use helix_core::{
    char_idx_at_visual_offset, diagnostic::DiagnosticProvider, uri::actualize_bookmarks,
    BookmarkUri, Diagnostic,
};
pub use theme::Theme;
pub use view::View;

type BookmarkCache = RefCell<Option<HashMap<PathBuf, Vec<BookmarkUri>>>>;

pub fn read_and_update_bookmarks_cache(
    bookmarks_cache: &BookmarkCache,
    doc: &Document,
) -> Vec<BookmarkUri> {
    let mut bookmark_file_path = helix_stdx::env::current_working_dir();
    bookmark_file_path.push(".bookmarks");
    let bookmark_file_path = bookmark_file_path.as_path().to_string_lossy().to_string();

    if bookmarks_cache.borrow().is_none() {
        // read bookmarks from file and update the cache
        log::info!("reading bookmark file from disk");

        let bookmarks_data = std::fs::read_to_string(bookmark_file_path).unwrap_or("".into());
        let bookmarks: Vec<BookmarkUri> = bookmarks_data
            .lines()
            .filter(|line| !line.is_empty())
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();

        let mut new_bookmarks_cache: HashMap<PathBuf, Vec<BookmarkUri>> = HashMap::new();

        for bookmark in bookmarks {
            new_bookmarks_cache
                .entry(bookmark.path.clone().into())
                .and_modify(|b| b.push(bookmark.clone()))
                .or_insert(vec![bookmark]);
        }

        *bookmarks_cache.borrow_mut() = Some(new_bookmarks_cache);
    }

    let bookmarks = doc
        .path()
        .and_then(|path| {
            bookmarks_cache
                .borrow()
                .as_ref()
                .and_then(|cache| cache.get(path).cloned())
        })
        .unwrap_or_default();
    actualize_bookmarks(bookmarks)
}

pub fn read_and_update_document_bookmarks_cache(doc: &Document) -> Vec<BookmarkUri> {
    let mut bookmark_file_path = helix_stdx::env::current_working_dir();
    bookmark_file_path.push(".bookmarks");
    let bookmark_file_path = bookmark_file_path.as_path().to_string_lossy().to_string();

    if let Some(doc_path) = doc
        .path()
        .map(|p| p.as_path().to_string_lossy().to_string())
    {
        if doc.bookmarks_cache.borrow().is_none() {
            // read bookmarks from file and update the cache
            log::info!("reading bookmark file from disk");

            let bookmarks_data = std::fs::read_to_string(bookmark_file_path).unwrap_or("".into());
            let bookmarks: Vec<BookmarkUri> = bookmarks_data
                .lines()
                .filter(|line| !line.is_empty())
                .map(|l| serde_json::from_str(l).unwrap())
                .collect();

            let mut new_bookmarks_cache: HashMap<usize, BookmarkUri> = HashMap::new();

            for bookmark in bookmarks {
                if bookmark.path != doc_path {
                    continue;
                }

                new_bookmarks_cache.insert(bookmark.line, bookmark);
            }

            *doc.bookmarks_cache.borrow_mut() = Some(new_bookmarks_cache);
        }

        doc.bookmarks_cache
            .borrow()
            .clone()
            .map(|cache| cache.values().cloned().collect())
            .unwrap_or_default()
    } else {
        vec![]
    }
}

pub fn convert_bookmarks_to_fake_diagnostics(
    doc: &Document,
    bookmarks: Vec<BookmarkUri>,
) -> Vec<Diagnostic> {
    let mut diagnostics = vec![];

    for bookmark in bookmarks {
        let fake_lsp_diagnostics = helix_lsp::lsp::Diagnostic::new_simple(
            helix_lsp::lsp::Range::new(
                helix_lsp::Position {
                    line: bookmark.line as u32,
                    character: 0,
                },
                helix_lsp::Position {
                    line: bookmark.line as u32,
                    character: 0,
                },
            ),
            bookmark.name,
        );
        let diagnostic = Document::lsp_diagnostic_to_diagnostic(
            doc.text(),
            None,
            &fake_lsp_diagnostics,
            DiagnosticProvider::Fake,
            helix_lsp::OffsetEncoding::default(),
        )
        .unwrap();
        diagnostics.push(diagnostic);
    }

    diagnostics
}
