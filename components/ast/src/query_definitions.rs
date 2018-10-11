use crate::item_id::ItemId;
use crate::item_id::ItemIdData;
use crate::AstDatabase;
use intern::Intern;
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

crate fn items_in_file(db: &impl AstDatabase, input_file: StringId) -> Arc<Vec<ItemId>> {
    let ast_of_file = match db.ast_of_file(input_file) {
        Ok(module) => module,
        Err(_) => return Arc::new(vec![]),
    };

    let items: Vec<_> = ast_of_file
        .items
        .iter()
        .map(|item| {
            ItemIdData {
                input_file,
                path: Arc::new(vec![item.name()]),
            }
            .intern(db)
        })
        .collect();
    Arc::new(items)
}

crate fn ast_of_item(db: &impl AstDatabase, item_id: ItemId) -> Result<Arc<ast::Item>, ParseError> {
    let ItemIdData { input_file, path } = item_id.untern(db);
    let module = db.ast_of_file(input_file)?;

    // have to follow `path` through `module`; for now we'll just support lenth-1 paths
    // (no nested items)
    if path.len() != 1 {
        unimplemented!();
    }
    let path_id = path[0];

    for item in &module.items {
        if item.name() == path_id {
            return Ok(item.clone());
        }
    }

    panic!("no such item")
}
