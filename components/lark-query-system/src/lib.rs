use language_reporting as l_r;
use lark_entity::EntityTables;
use lark_intern::{Intern, Untern};
use lark_parser::{ParserDatabase, ParserDatabaseExt};
use lark_pretty_print::PrettyPrintDatabase;
use lark_span::{ByteIndex, FileName, Span};
use lark_string::{GlobalIdentifier, GlobalIdentifierTables, Text};
use lark_task_manager::{Actor, NoopSendChannel, QueryRequest, QueryResponse, SendChannel};
use salsa::{Database, ParallelDatabase, Snapshot};
use std::collections::VecDeque;
use std::sync::Arc;
use url::Url;

pub mod ls_ops;
use self::ls_ops::{Cancelled, LsDatabase};

pub struct LarkDatabase {
    runtime: salsa::Runtime<LarkDatabase>,
    item_id_tables: Arc<EntityTables>,
    global_id_tables: Arc<GlobalIdentifierTables>,
    declaration_tables: Arc<lark_ty::declaration::DeclarationTables>,
    base_inferred_tables: Arc<lark_ty::base_inferred::BaseInferredTables>,
    full_inferred_tables: Arc<lark_ty::full_inferred::FullInferredTables>,
}

impl std::fmt::Debug for LarkDatabase {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.debug_struct("LarkDatabase").finish()
    }
}

impl ParserDatabaseExt for LarkDatabase {}

impl LarkDatabase {
    pub fn intern_string(&self, s: &str) -> GlobalIdentifier {
        s.intern(self)
    }

    pub fn untern_string(&self, id: GlobalIdentifier) -> Text {
        id.untern(self)
    }
}

impl Default for LarkDatabase {
    fn default() -> Self {
        let mut db = LarkDatabase {
            runtime: Default::default(),
            item_id_tables: Default::default(),
            global_id_tables: Default::default(),
            declaration_tables: Default::default(),
            base_inferred_tables: Default::default(),
            full_inferred_tables: Default::default(),
        };
        db.init_parser_db();
        db
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
            runtime: self.runtime.snapshot(self),
            item_id_tables: self.item_id_tables.clone(),
            global_id_tables: self.global_id_tables.clone(),
            declaration_tables: self.declaration_tables.clone(),
            base_inferred_tables: self.base_inferred_tables.clone(),
            full_inferred_tables: self.full_inferred_tables.clone(),
        })
    }
}

impl PrettyPrintDatabase for LarkDatabase {}

impl LsDatabase for LarkDatabase {}

salsa::database_storage! {
    pub struct LarkDatabaseStorage for LarkDatabase {
        impl lark_parser::ParserDatabase {
            fn file_names() for lark_parser::FileNamesQuery;
            fn file_text() for lark_parser::FileTextQuery;
            fn line_offsets() for lark_parser::LineOffsetsQuery;
            fn location() for lark_parser::LocationQuery;
            fn byte_index() for lark_parser::ByteIndexQuery;
            fn file_tokens() for lark_parser::FileTokensQuery;
            fn parsed_file() for lark_parser::ParsedFileQuery;
            fn child_parsed_entities() for lark_parser::ChildParsedEntitiesQuery;
            fn parsed_entity() for lark_parser::ParsedEntityQuery;
            fn child_entities() for lark_parser::ChildEntitiesQuery;
            fn fn_body() for lark_parser::FnBodyQuery;
            fn hover_targets() for lark_parser::HoverTargetsQuery;
            fn members() for lark_parser::MembersQuery;
            fn member_entity() for lark_parser::MemberEntityQuery;
            fn descendant_entities() for lark_parser::DescendantEntitiesQuery;
            fn entity_span() for lark_parser::EntitySpanQuery;
            fn ty() for lark_parser::TyQuery;
            fn signature() for lark_parser::SignatureQuery;
            fn generic_declarations() for lark_parser::GenericDeclarationsQuery;
            fn resolve_name() for lark_parser::ResolveNameQuery;
        }
        impl lark_type_check::TypeCheckDatabase {
            fn base_type_check() for lark_type_check::BaseTypeCheckQuery;
            fn full_type_check() for lark_type_check::FullTypeCheckQuery;
        }
    }
}

impl AsRef<EntityTables> for LarkDatabase {
    fn as_ref(&self) -> &EntityTables {
        &self.item_id_tables
    }
}

impl AsRef<GlobalIdentifierTables> for LarkDatabase {
    fn as_ref(&self) -> &GlobalIdentifierTables {
        &self.global_id_tables
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

impl AsRef<lark_ty::full_inferred::FullInferredTables> for LarkDatabase {
    fn as_ref(&self) -> &lark_ty::full_inferred::FullInferredTables {
        &self.full_inferred_tables
    }
}

impl l_r::ReportingFiles for &LarkDatabase {
    type Span = Span<FileName>;
    type FileId = FileName;

    fn byte_span(
        &self,
        file: Self::FileId,
        from_index: usize,
        to_index: usize,
    ) -> Option<Self::Span> {
        Some(Span::new(file, from_index, to_index))
    }

    fn file_id(&self, span: Self::Span) -> Self::FileId {
        span.file()
    }

    fn file_name(&self, file: Self::FileId) -> l_r::FileName {
        let text = file.id.untern(self);
        l_r::FileName::Verbatim(text.to_string())
    }

    fn byte_index(&self, file: Self::FileId, line: usize, column: usize) -> Option<usize> {
        let b_i = ParserDatabase::byte_index(*self, file, line as u64, column as u64);
        Some(b_i.to_usize())
    }

    fn location(&self, file: Self::FileId, byte_index: usize) -> Option<l_r::Location> {
        let location = ParserDatabase::location(*self, file, ByteIndex::from(byte_index));
        Some(l_r::Location {
            line: location.line,
            column: location.column,
        })
    }

    fn line_span(&self, file: Self::FileId, lineno: usize) -> Option<Self::Span> {
        let line_offsets = self.line_offsets(file);
        let line_start = line_offsets[lineno];
        let next_line_start = line_offsets[lineno + 1];

        // This includes the `\n` from `lineno`, is that ok?
        Some(Span::new(file, line_start, next_line_start))
    }

    fn source(&self, span: Self::Span) -> Option<String> {
        let file = span.file();
        Some(self.file_text(file)[span].to_string())
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
        let _killme = KillTheProcess;
        log::info!("process_message(message={:#?})", message);

        match message {
            QueryRequest::OpenFile(url, contents) => {
                let text = contents.intern(&self.lark_db).untern(&self.lark_db);

                // Process sets on the same thread -- this not only gives them priority,
                // it ensures an overall ordering to edits.
                self.lark_db.add_file(url.as_str(), text);
            }

            QueryRequest::EditFile(url, changes) => {
                // Process sets on the same thread -- this not only gives them priority,
                // it ensures an overall ordering to edits.
                let path_id = self.lark_db.intern_string(url.as_str());
                let file_name = FileName { id: path_id };

                let text = self.lark_db.file_text(file_name);
                let mut current_contents = text.to_string();

                for change in changes {
                    let start_position = change.0.start;
                    let start_offset = self.lark_db.byte_index(
                        file_name,
                        start_position.line,
                        start_position.character,
                    );

                    let end_position = change.0.end;
                    let end_offset = self.lark_db.byte_index(
                        file_name,
                        end_position.line,
                        end_position.character,
                    );

                    unsafe {
                        let vec = current_contents.as_mut_vec();
                        vec.drain(start_offset.to_usize()..end_offset.to_usize());
                    }

                    current_contents.insert_str(start_offset.to_usize(), &change.1);
                }

                let text = Text::from(current_contents);
                self.lark_db.add_file(url.as_str(), text);
            }
            QueryRequest::DefinitionAtPosition(task_id, url, position) => {
                std::thread::spawn({
                    let db = self.lark_db.snapshot();
                    let send_channel = self.send_channel.clone_send_channel();
                    move || {
                        let _killme = KillTheProcess;

                        match db.definition_range_at_position(url.as_str(), position) {
                            Ok(Some(v)) => {
                                send_channel.send(QueryResponse::Range(
                                    task_id,
                                    Url::parse(&v.0).unwrap(),
                                    v.1,
                                ));
                            }
                            _ => {
                                send_channel.send(QueryResponse::Nothing(task_id));
                            }
                        }
                    }
                });
            }
            QueryRequest::TypeAtPosition(task_id, url, position) => {
                std::thread::spawn({
                    let db = self.lark_db.snapshot();
                    let send_channel = self.send_channel.clone_send_channel();
                    move || {
                        let _killme = KillTheProcess;

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

/// A little struct which -- when dropped -- will abort the process if
/// we have panicked. This is a temporary band-aid to have us die on
/// panic and prevent tests from hanging.
struct KillTheProcess;

impl Drop for KillTheProcess {
    fn drop(&mut self) {
        if std::thread::panicking() {
            std::process::abort();
        }
    }
}
