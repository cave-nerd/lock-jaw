use egui_commonmark::{CommonMarkCache, CommonMarkViewer};

/// Stateful cache for rendered markdown — reuse across frames.
pub struct MarkdownCache {
    inner: CommonMarkCache,
}

impl Default for MarkdownCache {
    fn default() -> Self {
        Self {
            inner: CommonMarkCache::default(),
        }
    }
}

impl MarkdownCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Render markdown `text` into the given egui `Ui` inside a scroll area.
    /// `note_id` must be unique per note (use the file path string).
    pub fn render(&mut self, ui: &mut egui::Ui, note_id: &str, text: &str) {
        CommonMarkViewer::new().show_scrollable(note_id, ui, &mut self.inner, text);
    }
}

/// Returns a word count for the given text.
pub fn word_count(text: &str) -> usize {
    text.split_whitespace().count()
}
