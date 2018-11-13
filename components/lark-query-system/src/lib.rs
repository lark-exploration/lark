use codespan::{CodeMap, FileMap};
use lark_entity::EntityTables;
use lark_hir as hir;
use lark_mir2 as mir;
use lark_string::global::GlobalIdentifierTables;
use lark_task_manager::{Actor, NoopSendChannel, QueryRequest, QueryResponse, SendChannel};
use map::FxIndexMap;
use parking_lot::RwLock;
use parser::{HasParserState, HasReaderState, ParserState, ReaderDatabase, ReaderState};
use salsa::{Database, ParallelDatabase, Snapshot};
use std::collections::VecDeque;
use std::sync::Arc;
use url::Url;

pub mod ls_ops;
use self::ls_ops::{Cancelled, LsDatabase};

#[derive(Default)]
pub struct LarkDatabase {
    runtime: salsa::Runtime<LarkDatabase>,
    code_map: Arc<RwLock<CodeMap>>,
    file_maps: Arc<RwLock<FxIndexMap<String, Arc<FileMap>>>>,
    parser_state: Arc<ParserState>,
    reader_state: ReaderState,
    item_id_tables: Arc<EntityTables>,
    declaration_tables: Arc<lark_ty::declaration::DeclarationTables>,
    base_inferred_tables: Arc<lark_ty::base_inferred::BaseInferredTables>,
}

impl LarkDatabase {
    pub fn code_map(&self) -> &RwLock<CodeMap> {
        &self.code_map
    }
}

impl Database for LarkDatabase {
    fn salsa_runtime(&self) -> &salsa::Runtime<LarkDatabase> {
        &self.runtime
    }
}

impl ParallelDatabase for LarkDatabase {
    fn snapshot(&self) -> Snapshot<Self> {
        Snapshot::new(LarkDatabase {
            code_map: self.code_map.clone(),
            file_maps: self.file_maps.clone(),
            runtime: self.runtime.snapshot(self),
            parser_state: self.parser_state.clone(),
            reader_state: self.reader_state.clone(),
            item_id_tables: self.item_id_tables.clone(),
            declaration_tables: self.declaration_tables.clone(),
            base_inferred_tables: self.base_inferred_tables.clone(),
        })
    }
}

impl LsDatabase for LarkDatabase {}

salsa::database_storage! {
    pub struct LarkDatabaseStorage for LarkDatabase {
        impl lark_parser::ParserDatabase {
            fn file_names() for lark_parser::FileNamesQuery;
            fn file_text() for lark_parser::FileTextQuery;
            fn child_parsed_entities() for lark_parser::ChildParsedEntitiesQuery;
            fn parsed_entity() for lark_parser::ParsedEntityQuery;
            fn child_entities() for lark_parser::ChildEntitiesQuery;
        }
        impl parser::ReaderDatabase {
            fn paths() for parser::Paths;
            fn paths_trigger() for parser::PathsTrigger;
            fn source() for parser::Source;
        }
        impl ast::AstDatabase {
            fn ast_of_file() for ast::AstOfFileQuery;
            fn items_in_file() for ast::ItemsInFileQuery;
            fn ast_of_item() for ast::AstOfItemQuery;
            fn ast_of_field() for ast::AstOfFieldQuery;
            fn entity_span() for ast::EntitySpanQuery;
        }
        impl hir::HirDatabase {
            fn fn_body() for hir::FnBodyQuery;
            fn members() for hir::MembersQuery;
            fn member_entity() for hir::MemberEntityQuery;
            fn subentities() for hir::SubentitiesQuery;
            fn ty() for hir::TyQuery;
            fn signature() for hir::SignatureQuery;
            fn generic_declarations() for hir::GenericDeclarationsQuery;
            fn resolve_name() for hir::ResolveNameQuery;
        }
        impl lark_type_check::TypeCheckDatabase {
            fn base_type_check() for lark_type_check::BaseTypeCheckQuery;
        }
        impl mir::MirDatabase {
            fn fn_bytecode() for mir::FnBytecodeQuery;
        }
    }
}

impl AsRef<EntityTables> for LarkDatabase {
    fn as_ref(&self) -> &EntityTables {
        &self.item_id_tables
    }
}

impl AsRef<lark_ty::declaration::DeclarationTables> for LarkDatabase {
    fn as_ref(&self) -> &lark_ty::declaration::DeclarationTables {
        &self.declaration_tables
    }
}

impl AsRef<lark_ty::base_inferred::BaseInferredTables> for LarkDatabase {
    fn as_ref(&self) -> &lark_ty::base_inferred::BaseInferredTables {
        &self.base_inferred_tables
    }
}

impl AsRef<GlobalIdentifierTables> for LarkDatabase {
    fn as_ref(&self) -> &GlobalIdentifierTables {
        <ParserState as AsRef<GlobalIdentifierTables>>::as_ref(&self.parser_state)
    }
}

impl HasParserState for LarkDatabase {
    fn parser_state(&self) -> &ParserState {
        &self.parser_state
    }
}

impl HasReaderState for LarkDatabase {
    fn reader_state(&self) -> &ReaderState {
        &self.reader_state
    }
}

pub struct QuerySystem {
    send_channel: Box<dyn SendChannel<QueryResponse>>,
    lark_db: LarkDatabase,
    needs_error_check: bool,
}

impl QuerySystem {
    pub fn new() -> QuerySystem {
        QuerySystem {
            send_channel: Box::new(NoopSendChannel),
            lark_db: LarkDatabase::default(),
            needs_error_check: false,
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

    fn receive_messages(&mut self, messages: &mut VecDeque<Self::InMessage>) {
        log::info!("receive_messages({} messages pending)", messages.len());

        // Find the last mutation in our list. Up until that point, we need to process *only*
        // mutations.
        if let Some(last_mutation) = messages.iter().rposition(|message| message.is_mutation()) {
            for message in messages.drain(0..=last_mutation) {
                if message.is_mutation() {
                    self.process_message(message);
                }
            }

            // After each mutation, we need to perform an error-check at some point.
            self.needs_error_check = true;
        }

        // OK, all mutations are processed. Now we can process the next non-mutation (if any).
        if let Some(message) = messages.pop_front() {
            assert!(!message.is_mutation());
            self.process_message(message);
        }

        // If there are no more pending messages, we can go ahead and
        // start checking for errors.  Otherwise, return, and we'll be
        // called again.
        if messages.is_empty() && self.needs_error_check {
            self.check_for_errors_and_report();
        }
    }
}

impl QuerySystem {
    pub fn check_for_errors_and_report(&mut self) {
        self.needs_error_check = false;
        std::thread::spawn({
            let db = self.lark_db.snapshot();
            let send_channel = self.send_channel.clone_send_channel();
            move || {
                match db.errors_for_project() {
                    Ok(errors) => {
                        // loop over hashmap and send messages
                        for (key, value) in errors {
                            let url = Url::parse(&key).unwrap();
                            let ranges_with_default =
                                value.iter().map(|x| (x.range, x.label.clone())).collect();
                            send_channel.send(QueryResponse::Diagnostics(url, ranges_with_default));
                        }
                    }
                    Err(Cancelled) => {
                        // Ignore
                    }
                }
            }
        });
    }

    fn process_message(&mut self, message: QueryRequest) {
        log::info!("process_message(message={:#?})", message);

        match message {
            QueryRequest::OpenFile(url, contents) => {
                // Process sets on the same thread -- this not only gives them priority,
                // it ensures an overall ordering to edits.
                self.lark_db.add_file(url.as_str(), contents.as_str());
            }

            QueryRequest::EditFile(url, changes) => {
                // Process sets on the same thread -- this not only gives them priority,
                // it ensures an overall ordering to edits.
                let path_id = self.lark_db.intern_string(url.as_str());

                let file_maps = self.lark_db.source(path_id);

                let mut current_contents = file_maps.source().to_string();

                let origin_byte_offset = file_maps.byte_index(0, 0).unwrap();

                for change in changes {
                    let start_position = change.0.start;
                    let start_byte_offset = file_maps
                        .byte_index(start_position.line, start_position.character)
                        .unwrap();

                    let end_position = change.0.end;
                    let end_byte_offset = file_maps
                        .byte_index(end_position.line, end_position.character)
                        .unwrap();

                    unsafe {
                        let vec = current_contents.as_mut_vec();
                        vec.drain(
                            (start_byte_offset - origin_byte_offset).to_usize()
                                ..(end_byte_offset - origin_byte_offset).to_usize(),
                        );
                    }

                    current_contents.insert_str(
                        (start_byte_offset - origin_byte_offset).to_usize(),
                        &change.1,
                    );
                }

                self.lark_db
                    .add_file(url.as_str(), current_contents.to_string());
            }
            QueryRequest::TypeAtPosition(task_id, url, position) => {
                std::thread::spawn({
                    let db = self.lark_db.snapshot();
                    let send_channel = self.send_channel.clone_send_channel();
                    move || {
                        match db.hover_text_at_position(url.as_str(), position) {
                            Ok(Some(v)) => {
                                send_channel.send(QueryResponse::Type(task_id, v.to_string()));
                            }
                            Ok(None) => {
                                // FIXME what to send here to indicate "no hover"?
                                send_channel.send(QueryResponse::Type(task_id, "".to_string()));
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

        log::info!("receive_message: awaiting next message");
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
