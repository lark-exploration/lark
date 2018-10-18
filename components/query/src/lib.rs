use std::sync::Arc;

use ast::{
    AstDatabase, AstOfFile, AstOfItem, HasParserState, InputFiles, InputText, ItemIdTables,
    ItemsInFile, ParserState,
};
use intern::Has;
use salsa::Database;
use task_manager::{Actor, QueryRequest, QueryResponse};

#[derive(Default)]
struct LarkDatabase {
    runtime: salsa::Runtime<LarkDatabase>,
    parser_state: ParserState,
    item_id_tables: ItemIdTables,
}

impl salsa::Database for LarkDatabase {
    fn salsa_runtime(&self) -> &salsa::Runtime<LarkDatabase> {
        &self.runtime
    }
}

salsa::database_storage! {
    struct LarkDatabaseStorage for LarkDatabase {
        impl AstDatabase {
            fn input_files() for InputFiles;
            fn input_text() for InputText;
            fn ast_of_file() for AstOfFile;
            fn items_in_file() for ItemsInFile;
            fn ast_of_item() for AstOfItem;
        }
    }
}

impl parser::LookupStringId for LarkDatabase {
    fn lookup(&self, id: parser::StringId) -> Arc<String> {
        self.untern_string(id)
    }
}

impl Has<ItemIdTables> for LarkDatabase {
    fn intern_tables(&self) -> &ItemIdTables {
        &self.item_id_tables
    }
}

impl HasParserState for LarkDatabase {
    fn parser_state(&self) -> &ParserState {
        &self.parser_state
    }
}

pub struct QuerySystem {
    send_channel: Option<Box<dyn Fn(QueryResponse) -> () + Send>>,
    lark_db: LarkDatabase,
}

impl QuerySystem {
    pub fn new() -> QuerySystem {
        QuerySystem {
            send_channel: None,
            lark_db: LarkDatabase::default(),
        }
    }
}

impl Actor for QuerySystem {
    type InMessage = QueryRequest;
    type OutMessage = QueryResponse;

    fn startup(&mut self, send_channel: Box<dyn Fn(Self::OutMessage) -> () + Send>) {
        self.send_channel = Some(send_channel);
    }

    fn shutdown(&mut self) {}

    fn receive_message(&mut self, message: Self::InMessage) {
        match message {
            QueryRequest::OpenFile(url, contents) => {
                let interned_path = self.lark_db.intern_string(url.as_str());
                let interned_contents = self.lark_db.intern_string(contents.as_str());
                self.lark_db
                    .query(InputFiles)
                    .set((), Arc::new(vec![interned_path]));
                self.lark_db
                    .query(InputText)
                    .set(interned_path, Some(interned_contents));
            }
            QueryRequest::EditFile(_) => {}
            QueryRequest::TypeAtPosition(task_id, url, _position) => {
                let interned_path = self.lark_db.intern_string(url.as_str());
                let result = self.lark_db.query(InputText).get(interned_path);

                let contents = self.lark_db.untern_string(result.unwrap());
                match self.send_channel {
                    Some(ref c) => c(QueryResponse::Type(task_id, contents.to_string())),
                    None => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
