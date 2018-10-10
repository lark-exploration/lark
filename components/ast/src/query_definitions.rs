use crate::item_id::ItemId;
use crate::item_id::ItemIdData;
use crate::AstDatabase;
use intern::Untern;
use parser::ast;
use parser::ParseError;
use parser::StringId;
use std::sync::Arc;

crate fn ast_of_file(
    db: &impl AstDatabase,
    path: StringId,
) -> Result<Arc<ast::Module>, ParseError> {
    let input_text = db.input_text(path).unwrap_or_else(|| {
        panic!("no input text for path `{}`", db.untern_string(path));
    });

    let module = db.parser_state().parse(path, input_text)?;

    Ok(Arc::new(module))
}

crate fn ast_of_item(db: &impl AstDatabase, item_id: ItemId) -> Result<Arc<ast::Item>, ParseError> {
    let ItemIdData { input_file, path } = item_id.untern(db);
    let module = db.ast_of_file(input_file)?;

    // have to follow `path` through `module`
    std::mem::drop((path, module));

    unimplemented!()
}
