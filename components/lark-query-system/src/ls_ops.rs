//! Contains "pseudo-queries" for language-server interaction. These
//! aren't *actual* queries, they are just functions, so they are not
//! memoized.  This also means they can take arbitrary parameters
//! (e.g. `&uri`) that wouldn't be possible otherwise, which is
//! convenient.

use languageserver_types::Position;

pub(crate) struct Cancelled;

pub(crate) type Cancelable<T> = Result<T, Cancelled>;

pub(crate) trait LsDatabase: type_check::TypeCheckDatabase {
    fn check_for_cancellation(&self) -> Cancelable<()> {
        if self.salsa_runtime().is_current_revision_canceled() {
            Err(Cancelled)
        } else {
            Ok(())
        }
    }

    fn type_at_position(&self, url: &str, _position: Position) -> Cancelable<String> {
        let interned_path = self.intern_string(url);
        let result = self.input_text(interned_path);
        let contents = self.untern_string(result.unwrap().text);
        self.check_for_cancellation()?;
        Ok(contents.to_string())
    }
}
