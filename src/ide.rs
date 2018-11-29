use lark_language_server::{lsp_serve, LspResponder};
use lark_query_system::QuerySystem;
use lark_task_manager::Actor;

pub fn ide() {
    let query_system = QuerySystem::new();
    let lsp_responder = LspResponder;

    let task_manager = lark_task_manager::TaskManager::spawn(query_system, lsp_responder);

    lsp_serve(task_manager.channel);
    let _ = task_manager.join_handle.join();
}
