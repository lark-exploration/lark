use lark_string::Text;

/// Trait used for contexts that know the current file. This is used
/// in `DebugWith` implementations to handle local strings.
pub trait HasCurrentFileText {
    fn current_file_text(&self) -> Text;
}

impl HasCurrentFileText for Text {
    fn current_file_text(&self) -> Text {
        self.clone()
    }
}
