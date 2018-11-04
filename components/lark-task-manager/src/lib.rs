use map::FxIndexMap;
use std::collections::VecDeque;
use std::sync::mpsc::{channel, Receiver, RecvError, Sender, TryRecvError};
use std::thread;
use url::Url;

use languageserver_types::{Position, Range};

pub type TaskId = usize;

/// A message the manager sends to the subsystem
/// This enables it both to control the subsystem and
/// transmit data.
enum MsgFromManager<T> {
    Shutdown,
    Message(T),
}

/// Message that the manager can understand from
/// the subsystems.
pub enum MsgToManager {
    QueryResponse(QueryResponse),
    LspRequest(LspRequest),
    Cancel(TaskId),
    Shutdown,
}

/// Tasks that the LSP service can request
/// from the manager.
pub enum LspRequest {
    TypeForPos(TaskId, Url, Position),
    OpenFile(Url, String),
    EditFile(Url, Vec<(Range, String)>),
    Initialize(TaskId),
}

/// Responses back to the LSP services from
/// the manager.
pub enum LspResponse {
    Type(TaskId, String),
    Completions(TaskId, Vec<(String, String)>),
    Initialized(TaskId),
    Diagnostics(Url, Vec<(Range, String)>),
}

/// Requests from the manager to the query
/// system
#[derive(Debug)]
pub enum QueryRequest {
    /// URI followed by contents
    OpenFile(Url, String),
    EditFile(Url, Vec<(Range, String)>),
    TypeAtPosition(TaskId, Url, Position),
}

impl QueryRequest {
    /// True if this query will cause us to mutate the state of the
    /// program.
    pub fn is_mutation(&self) -> bool {
        match self {
            QueryRequest::OpenFile(..) | QueryRequest::EditFile(..) => true,
            QueryRequest::TypeAtPosition(..) => false,
        }
    }
}

/// Responses from the query system back to the
/// manager
pub enum QueryResponse {
    Type(TaskId, String),
    Diagnostics(Url, Vec<(Range, String)>),
}

/// Requests are broken into a series of steps called a recipe, each
/// composed of finer-grained steps. This stepping allows a bit more
/// control over when tasks are cancelled, their priority, and how
/// they become parallel.
enum RecipeStep {
    GetTextForFile,

    RespondWithType,
    RespondWithInitialized,
}

/// An actor in the task system. This gives a uniform way to
/// create, control, message, and shutdown concurrent workers.
pub trait Actor {
    type InMessage: Send + Sync + 'static;
    type OutMessage: Send + Sync + 'static;

    fn startup(&mut self, send_channel: &dyn SendChannel<Self::OutMessage>);

    /// Invoked when new message(s) arrive. Contains all the messages
    /// that can be pulled at this time. The actor is free to process
    /// as many as they like. So long as messages remain in the
    /// dequeue, we'll just keep calling back (possibly appending more
    /// messages to the back). Once the queue is empty, we'll block
    /// until we can fetch more.
    ///
    /// The intended workflow is as follows:
    ///
    /// - If desired, inspect `messages` and prune messages that become outdated
    ///   due to later messages in the queue.
    /// - Invoke `messages.pop_front().unwrap()` and process that message,
    ///   then return.
    ///   - In particular, it is probably better to return than to eagerly process
    ///     all messages in the queue, as it gives the actor a chance to add more
    ///     messages if they have arrived in the meantime.
    ///     - This is only important if you are trying to remove outdated messages.
    fn receive_messages(&mut self, messages: &mut VecDeque<Self::InMessage>);

    fn shutdown(&mut self);
}

pub trait SendChannel<T: Send + 'static>: Send + 'static {
    fn send(&self, value: T);
    fn clone_send_channel(&self) -> Box<dyn SendChannel<T>>;
}

impl SendChannel<QueryResponse> for Sender<MsgToManager> {
    fn send(&self, value: QueryResponse) {
        match self.send(MsgToManager::QueryResponse(value)) {
            Ok(()) => {}
            Err(_) => panic!("manager no longer listening"),
        }
    }

    fn clone_send_channel(&self) -> Box<dyn SendChannel<QueryResponse>> {
        Box::new(self.clone())
    }
}

pub struct NoopSendChannel;

impl<T: Send + 'static> SendChannel<T> for NoopSendChannel {
    fn send(&self, _value: T) {}

    fn clone_send_channel(&self) -> Box<dyn SendChannel<T>> {
        Box::new(NoopSendChannel)
    }
}

pub struct ActorControl<MessageType: Send + Sync + 'static> {
    pub channel: Sender<MessageType>,
    pub join_handle: std::thread::JoinHandle<()>,
}

/// The coordinator of tasks coming in from the IDE services to the
/// parts of the system that will do the processing.
pub struct TaskManager {
    live_recipes: FxIndexMap<TaskId, Vec<RecipeStep>>,
    receive_channel: Receiver<MsgToManager>,

    /// Control points to communicate with other subsystems
    query_system: ActorControl<MsgFromManager<QueryRequest>>,
    lsp_responder: ActorControl<MsgFromManager<LspResponse>>,
}

impl TaskManager {
    pub fn spawn(
        mut query_system: impl Actor<InMessage = QueryRequest, OutMessage = QueryResponse>
            + Send
            + 'static,
        mut lsp_responder: impl Actor<InMessage = LspResponse> + Send + 'static,
    ) -> ActorControl<MsgToManager> {
        let (manager_tx, manager_rx) = channel();

        query_system.startup(&manager_tx);
        lsp_responder.startup(&NoopSendChannel);

        let query_system_actor = TaskManager::spawn_actor(query_system);
        let lsp_responder_actor = TaskManager::spawn_actor(lsp_responder);

        let task_manager = TaskManager {
            live_recipes: FxIndexMap::default(),
            receive_channel: manager_rx,

            query_system: query_system_actor,
            lsp_responder: lsp_responder_actor,
        };

        let join_handle = thread::spawn(move || {
            task_manager.message_loop();
        });

        ActorControl {
            channel: manager_tx,
            join_handle,
        }
    }

    fn join_worker_threads(self) {
        let _ = self.query_system.join_handle.join();
        let _ = self.lsp_responder.join_handle.join();
    }

    fn send_next_step(&mut self, task_id: TaskId, argument: Box<dyn std::any::Any>) {
        match self.live_recipes.get_mut(&task_id) {
            Some(x) => {
                if x.len() > 0 {
                    let next_step = x.remove(0);

                    match next_step {
                        RecipeStep::GetTextForFile => {
                            if let Ok(location) = argument.downcast::<(Url, Position)>() {
                                self.query_system
                                    .channel
                                    .send(MsgFromManager::Message(QueryRequest::TypeAtPosition(
                                        task_id, location.0, location.1,
                                    )))
                                    .unwrap();
                            }
                        }
                        RecipeStep::RespondWithType => {
                            if let Ok(ty) = argument.downcast::<String>() {
                                self.lsp_responder
                                    .channel
                                    .send(MsgFromManager::Message(LspResponse::Type(task_id, *ty)))
                                    .unwrap();
                            } else {
                                panic!("Internal error: malformed RespondWithType");
                            }
                        }
                        RecipeStep::RespondWithInitialized => {
                            self.lsp_responder
                                .channel
                                .send(MsgFromManager::Message(LspResponse::Initialized(task_id)))
                                .unwrap();
                        }
                    }
                }
            }
            None => {
                //Do nothing as task has completed or it has been cancelled
            }
        }
    }

    fn do_recipe_for_lsp_request(&mut self, lsp_request: LspRequest) {
        match lsp_request {
            LspRequest::TypeForPos(task_id, url, position) => {
                let recipe = vec![RecipeStep::GetTextForFile, RecipeStep::RespondWithType];

                self.live_recipes.insert(task_id, recipe);
                self.send_next_step(task_id, Box::new((url, position)));
            }
            LspRequest::OpenFile(url, contents) => {
                self.query_system
                    .channel
                    .send(MsgFromManager::Message(QueryRequest::OpenFile(
                        url, contents,
                    )))
                    .unwrap();
            }
            LspRequest::EditFile(url, changes) => {
                self.query_system
                    .channel
                    .send(MsgFromManager::Message(QueryRequest::EditFile(
                        url, changes,
                    )))
                    .unwrap();
            }
            LspRequest::Initialize(task_id) => {
                let recipe = vec![RecipeStep::RespondWithInitialized];

                self.live_recipes.insert(task_id, recipe);
                self.send_next_step(task_id, Box::new(()));
            }
        }
    }

    fn message_loop(mut self) {
        loop {
            match self.receive_channel.recv() {
                Ok(MsgToManager::QueryResponse(QueryResponse::Type(task_id, contents))) => {
                    self.send_next_step(task_id, Box::new(contents));
                }
                Ok(MsgToManager::QueryResponse(QueryResponse::Diagnostics(url, errors))) => {
                    let _ = self.lsp_responder.channel.send(MsgFromManager::Message(
                        LspResponse::Diagnostics(url, errors),
                    ));
                }
                Ok(MsgToManager::LspRequest(lsp_request)) => {
                    self.do_recipe_for_lsp_request(lsp_request);
                }
                Ok(MsgToManager::Cancel(task_id)) => {
                    //Note: In the future we may have multiple steps to cancel a task
                    self.live_recipes.remove(&task_id);
                }
                Ok(MsgToManager::Shutdown) => {
                    let _ = self.lsp_responder.channel.send(MsgFromManager::Shutdown);
                    let _ = self.query_system.channel.send(MsgFromManager::Shutdown);
                    break;
                }
                Err(_) => {
                    eprintln!("Error during host receive");
                }
            }
        }

        self.join_worker_threads();
    }

    fn spawn_actor<T: Actor + Send + 'static>(
        mut actor: T,
    ) -> ActorControl<MsgFromManager<T::InMessage>> {
        let (actor_tx, actor_rx) = channel();
        let mut message_queue = VecDeque::default();

        let handle = thread::spawn(move || loop {
            match push_all_pending(&actor_rx, &mut message_queue) {
                Ok(()) => {
                    actor.receive_messages(&mut message_queue);
                }
                Err(error) => {
                    match error {
                        PushAllPendingError::Disconnected => {
                            eprintln!("Failure during top-level message receive");
                        }

                        PushAllPendingError::ControlledShutdown => {}
                    }

                    break;
                }
            }
        });

        ActorControl {
            channel: actor_tx,
            join_handle: handle,
        }
    }
}

enum PushAllPendingError {
    ControlledShutdown,
    Disconnected,
}

fn push_all_pending<T>(
    rx: &Receiver<MsgFromManager<T>>,
    vec: &mut VecDeque<T>,
) -> Result<(), PushAllPendingError> {
    // If the queue is currently empty, then block until we get at
    // least one message.
    if vec.is_empty() {
        match rx.recv() {
            Ok(MsgFromManager::Message(m)) => vec.push_back(m),
            Ok(MsgFromManager::Shutdown) => return Err(PushAllPendingError::ControlledShutdown),
            Err(RecvError) => return Err(PushAllPendingError::Disconnected),
        }
    }

    // Once the queue is non-empty, opportunistically poll for more.
    loop {
        match rx.try_recv() {
            Ok(MsgFromManager::Message(m)) => vec.push_back(m),
            Err(TryRecvError::Empty) => break Ok(()),
            Ok(MsgFromManager::Shutdown) => return Err(PushAllPendingError::ControlledShutdown),
            Err(TryRecvError::Disconnected) => break Err(PushAllPendingError::Disconnected),
        }
    }
}
