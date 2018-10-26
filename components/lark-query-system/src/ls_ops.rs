//! Contains "pseudo-queries" for language-server interaction. These
//! aren't *actual* queries, they are just functions, so they are not
//! memoized.  This also means they can take arbitrary parameters
//! (e.g. `&uri`) that wouldn't be possible otherwise, which is
//! convenient.

use languageserver_types::Position;

pub(crate) struct Cancelled;

pub(crate) trait LsDatabase: type_check::TypeCheckDatabase {
    fn type_at_position(&self, url: &str, _position: Position) -> Result<String, Cancelled> {
        let interned_path = self.intern_string(url);
        let result = self.input_text(interned_path);
        let contents = self.untern_string(result.unwrap());
        if self.salsa_runtime().is_current_revision_canceled() {
            return Err(Cancelled);
        }
        Ok(contents.to_string())
    }
}
