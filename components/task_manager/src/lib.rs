use languageserver_types::Position;

use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

use mir::DefId;

type TaskId = usize;

enum MsgFromManager<T> {
    Shutdown,
    Message(T),
}

#[derive(Debug)]
pub enum TypeMessage {
    DefIdForPos(TaskId, Position),
    TypeForDefId(TaskId, DefId),
    CompletionsForDefId(TaskId, DefId),
}

pub enum TypeResponse {
    DefId(TaskId, DefId),
    Type(TaskId, String),
    Completions(TaskId, Vec<(String, String)>),
}

pub enum LspRequest {
    TypeForPos(TaskId, Position),
    Completion(TaskId, Position),
    Initialize(TaskId),
}

pub enum LspResponse {
    Type(TaskId, String),
    Completions(TaskId, Vec<(String, String)>),
    Initialized(TaskId),
}

pub enum MsgToManager {
    TypeResponse(TypeResponse),
    LspRequest(LspRequest),
    Shutdown,
}

enum RecipeStep {
    GetDefIdForPosition,

    GetTypeForDefId,
    GetCompletionsForDefId,

    RespondWithType,
    RespondWithCompletions,
    RespondWithInitialized,
}

pub trait Actor {
    type InMessage: Send + Sync + 'static;
    type OutMessage: Send + Sync + 'static;

    fn startup(&mut self, send_channel: Box<dyn Fn(Self::OutMessage) -> () + Send>);
    fn receive_message(&mut self, message: Self::InMessage);
    fn shutdown(&mut self);
}

pub struct ActorControl<MessageType: Send + Sync + 'static> {
    pub channel: Sender<MessageType>,
    pub join_handle: std::thread::JoinHandle<()>,
}

pub struct FakeTypeChecker {
    send_channel: Option<Box<dyn Fn(TypeResponse) -> () + Send>>,
}

impl Actor for FakeTypeChecker {
    type InMessage = TypeMessage;
    type OutMessage = TypeResponse;

    fn startup(&mut self, send_channel: Box<dyn Fn(Self::OutMessage) -> () + Send>) {
        self.send_channel = Some(send_channel);
    }

    fn shutdown(&mut self) {}

    fn receive_message(&mut self, message: Self::InMessage) {
        match message {
            TypeMessage::DefIdForPos(task_id, pos) => match self.send_channel {
                Some(ref c) => c(TypeResponse::DefId(task_id, pos.line as usize * 100)),
                None => {}
            },
            TypeMessage::TypeForDefId(task_id, def_id) => match self.send_channel {
                Some(ref c) => c(TypeResponse::Type(task_id, format!("<type:{}>", def_id))),
                None => {}
            },
            TypeMessage::CompletionsForDefId(task_id, def_id) => match self.send_channel {
                Some(ref c) => c(TypeResponse::Completions(
                    task_id,
                    vec![
                        ("bar".into(), format!("First option for {}", def_id)),
                        ("foo".into(), format!("Second option for {}", def_id)),
                    ],
                )),
                None => {}
            },
        }
    }
}

impl FakeTypeChecker {
    pub fn new() -> FakeTypeChecker {
        FakeTypeChecker { send_channel: None }
    }
}

pub struct TaskManager {
    live_recipes: HashMap<TaskId, Vec<RecipeStep>>,
    receive_channel: Receiver<MsgToManager>,

    /// Control points to communicate with other subsystems
    type_checker: ActorControl<MsgFromManager<TypeMessage>>,
    lsp_responder: ActorControl<MsgFromManager<LspResponse>>,
}

impl TaskManager {
    pub fn spawn(
        mut type_checker: impl Actor<InMessage = TypeMessage, OutMessage = TypeResponse>
            + Send
            + 'static,
        mut lsp_responder: impl Actor<InMessage = LspResponse> + Send + 'static,
    ) -> ActorControl<MsgToManager> {
        let (manager_tx, manager_rx) = channel();

        let manager_tx_clone = manager_tx.clone();

        type_checker.startup(Box::new(move |x| {
            manager_tx_clone
                .send(MsgToManager::TypeResponse(x))
                .unwrap()
        }));
        lsp_responder.startup(Box::new(move |_| {}));

        let type_checker = TaskManager::spawn_actor(type_checker);
        let lsp_responder = TaskManager::spawn_actor(lsp_responder);

        let task_manager = TaskManager {
            live_recipes: HashMap::new(),
            receive_channel: manager_rx,

            type_checker,
            lsp_responder,
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
        let _ = self.type_checker.join_handle.join();
        let _ = self.lsp_responder.join_handle.join();
    }

    fn send_next_step(&mut self, task_id: TaskId, argument: Box<dyn std::any::Any>) {
        match self.live_recipes.get_mut(&task_id) {
            Some(x) => {
                if x.len() > 0 {
                    let next_step = x.remove(0);

                    match next_step {
                        RecipeStep::GetDefIdForPosition => {
                            if let Ok(position) = argument.downcast::<Position>() {
                                self.type_checker
                                    .channel
                                    .send(MsgFromManager::Message(TypeMessage::DefIdForPos(
                                        task_id, *position,
                                    )))
                                    .unwrap();
                            } else {
                                panic!("Internal error: malformed GetDefIdForPosition");
                            }
                        }
                        RecipeStep::GetTypeForDefId => {
                            if let Ok(def_id) = argument.downcast::<DefId>() {
                                self.type_checker
                                    .channel
                                    .send(MsgFromManager::Message(TypeMessage::TypeForDefId(
                                        task_id, *def_id,
                                    )))
                                    .unwrap();
                            } else {
                                panic!("Internal error: malformed GetTypeForDefId");
                            }
                        }
                        RecipeStep::GetCompletionsForDefId => {
                            if let Ok(def_id) = argument.downcast::<DefId>() {
                                self.type_checker
                                    .channel
                                    .send(MsgFromManager::Message(
                                        TypeMessage::CompletionsForDefId(task_id, *def_id),
                                    ))
                                    .unwrap();
                            } else {
                                panic!("Internal error: malformed GetCompletionsForDefId");
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
                        RecipeStep::RespondWithCompletions => {
                            if let Ok(completions) = argument.downcast::<Vec<(String, String)>>() {
                                self.lsp_responder
                                    .channel
                                    .send(MsgFromManager::Message(LspResponse::Completions(
                                        task_id,
                                        *completions,
                                    )))
                                    .unwrap();
                            } else {
                                panic!("Internal error: malformed RespondWithCompletion");
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
            LspRequest::TypeForPos(task_id, position) => {
                let recipe = vec![
                    RecipeStep::GetDefIdForPosition,
                    RecipeStep::GetTypeForDefId,
                    RecipeStep::RespondWithType,
                ];

                self.live_recipes.insert(task_id, recipe);
                self.send_next_step(task_id, Box::new(position));
            }

            LspRequest::Completion(task_id, position) => {
                let recipe = vec![
                    RecipeStep::GetDefIdForPosition,
                    RecipeStep::GetCompletionsForDefId,
                    RecipeStep::RespondWithCompletions,
                ];

                self.live_recipes.insert(task_id, recipe);
                self.send_next_step(task_id, Box::new(position));
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
                Ok(MsgToManager::TypeResponse(TypeResponse::DefId(task_id, def_id))) => {
                    self.send_next_step(task_id, Box::new(def_id));
                }
                Ok(MsgToManager::TypeResponse(TypeResponse::Type(task_id, type_id))) => {
                    self.send_next_step(task_id, Box::new(type_id));
                }
                Ok(MsgToManager::TypeResponse(TypeResponse::Completions(task_id, completions))) => {
                    self.send_next_step(task_id, Box::new(completions));
                }
                Ok(MsgToManager::LspRequest(lsp_request)) => {
                    self.do_recipe_for_lsp_request(lsp_request);
                }
                Ok(MsgToManager::Shutdown) => {
                    let _ = self.lsp_responder.channel.send(MsgFromManager::Shutdown);
                    let _ = self.type_checker.channel.send(MsgFromManager::Shutdown);
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

        let handle = thread::spawn(move || loop {
            match actor_rx.recv() {
                Ok(MsgFromManager::Message(message)) => actor.receive_message(message),
                Ok(MsgFromManager::Shutdown) => break,
                Err(_) => {
                    eprintln!("Failure during top-level message receive");
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
