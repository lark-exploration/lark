use lark_actor::{spawn_actor, Actor, LspResponse, QueryRequest};
use lark_language_server::{lsp_serve, LspResponder};
use lark_query_system::QuerySystem;
use std::sync::mpsc::{channel, Receiver, RecvError, Sender, TryRecvError};

pub fn ide() {
    let lsp_responder = spawn_actor(LspResponder);
    let query_system = spawn_actor(QuerySystem::new(lsp_responder.channel));

    lsp_serve(query_system.channel);
}
