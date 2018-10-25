use std::sync::Arc;

use ast::{
    AstDatabase, AstOfFile, AstOfItem, HasParserState, InputFiles, InputText, ItemsInFile,
    ParserState,
};
use languageserver_types::Position;
use lark_entity::EntityTables;
use lark_task_manager::{Actor, NoopSendChannel, QueryRequest, QueryResponse, SendChannel};
use salsa::{Database, ParallelDatabase};
use ty::interners::TyInternTables;

#[derive(Default)]
struct LarkDatabase {
    runtime: salsa::Runtime<LarkDatabase>,
    parser_state: Arc<ParserState>,
    item_id_tables: Arc<EntityTables>,
    ty_intern_tables: Arc<TyInternTables>,
}

impl Database for LarkDatabase {
    fn salsa_runtime(&self) -> &salsa::Runtime<LarkDatabase> {
        &self.runtime
    }
}

impl ParallelDatabase for LarkDatabase {
    fn fork(&self) -> Self {
        LarkDatabase {
            runtime: self.runtime.fork(),
            parser_state: self.parser_state.clone(),
            item_id_tables: self.item_id_tables.clone(),
            ty_intern_tables: self.ty_intern_tables.clone(),
        }
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
        impl hir::HirDatabase {
            fn boolean_entity() for hir::BooleanEntityQuery;
            fn fn_body() for hir::FnBodyQuery;
            fn members() for hir::MembersQuery;
            fn member_entity() for hir::MemberEntityQuery;
            fn ty() for hir::TyQuery;
            fn signature() for hir::SignatureQuery;
            fn generic_declarations() for hir::GenericDeclarationsQuery;
            fn resolve_name() for hir::ResolveNameQuery;
        }
        impl type_check::TypeCheckDatabase {
            fn base_type_check() for type_check::BaseTypeCheckQuery;
        }
    }
}

impl parser::LookupStringId for LarkDatabase {
    fn lookup(&self, id: parser::StringId) -> Arc<String> {
        self.untern_string(id)
    }
}

impl AsRef<EntityTables> for LarkDatabase {
    fn as_ref(&self) -> &EntityTables {
        &self.item_id_tables
    }
}

impl AsRef<TyInternTables> for LarkDatabase {
    fn as_ref(&self) -> &TyInternTables {
        &self.ty_intern_tables
    }
}

impl HasParserState for LarkDatabase {
    fn parser_state(&self) -> &ParserState {
        &self.parser_state
    }
}

pub struct QuerySystem {
    send_channel: Box<dyn SendChannel<QueryResponse>>,
    lark_db: LarkDatabase,
}

impl QuerySystem {
    pub fn new() -> QuerySystem {
        QuerySystem {
            send_channel: Box::new(NoopSendChannel),
            lark_db: LarkDatabase::default(),
        }
    }
}

impl Actor for QuerySystem {
    type InMessage = QueryRequest;
    type OutMessage = QueryResponse;

    fn startup(&mut self, send_channel: &dyn SendChannel<QueryResponse>) {
        self.send_channel = send_channel.clone_send_channel();
    }

    fn shutdown(&mut self) {}

    fn receive_message(&mut self, message: Self::InMessage) {
        match message {
            QueryRequest::OpenFile(url, contents) => {
                // Process sets on the same thread -- this not only gives them priority,
                // it ensures an overall ordering to edits.
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
            QueryRequest::TypeAtPosition(task_id, url, position) => {
                std::thread::spawn({
                    let db = self.lark_db.fork();
                    let send_channel = self.send_channel.clone_send_channel();
                    move || {
                        match type_at_position(&db, url.as_str(), position) {
                            Ok(v) => {
                                send_channel.send(QueryResponse::Type(task_id, v.to_string()));
                            }
                            Err(Cancelled) => {
                                // Not sure what to send here, if anything.
                                send_channel
                                    .send(QueryResponse::Type(task_id, format!("<cancelled>")));
                            }
                        }
                    }
                });
            }
        }
    }
}

struct Cancelled;

fn type_at_position(
    db: &impl AstDatabase,
    url: &str,
    _position: Position,
) -> Result<String, Cancelled> {
    let interned_path = db.intern_string(url);
    let result = db.input_text(interned_path);
    let contents = db.untern_string(result.unwrap());
    if db.salsa_runtime().is_current_revision_canceled() {
        return Err(Cancelled);
    }
    Ok(contents.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
