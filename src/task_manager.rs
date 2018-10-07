use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::{self, JoinHandle};

use crate::ide::{LspResponder, Position};
use crate::ir::DefId;

type TaskId = usize;

enum MsgFromManager<T> {
    Shutdown,
    Message(T),
}

#[derive(Debug)]
enum TypeMessage {
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

struct FakeTypeChecker {
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
                Some(ref c) => c(TypeResponse::DefId(task_id, pos.line as usize * 10)),
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
    fn new() -> FakeTypeChecker {
        FakeTypeChecker { send_channel: None }
    }
}

pub struct TaskManager {
    live_recipes: HashMap<TaskId, Vec<RecipeStep>>,
    receive_channel: Receiver<MsgToManager>,

    //Channel to send to this task manager
    pub send_to_manager: Sender<MsgToManager>,

    //Channels to communicate with other subsystems
    send_to_type_checker: Option<Sender<MsgFromManager<TypeMessage>>>,
    send_to_lsp_responder: Option<Sender<MsgFromManager<LspResponse>>>,

    //Handles when we shutdown threads,
    handle_for_type_checker: Option<std::thread::JoinHandle<()>>,
}

impl TaskManager {
    pub fn new() -> TaskManager {
        let (host_tx, host_rx) = channel();

        TaskManager {
            live_recipes: HashMap::new(),
            receive_channel: host_rx,
            send_to_manager: host_tx,
            send_to_type_checker: None,
            send_to_lsp_responder: None,
            handle_for_type_checker: None,
        }
    }

    fn send_next_step(&mut self, task_id: TaskId, argument: Box<dyn std::any::Any>) {
        match self.live_recipes.get_mut(&task_id) {
            Some(x) => {
                if x.len() > 0 {
                    let next_step = x.remove(0);

                    match next_step {
                        RecipeStep::GetDefIdForPosition => {
                            if let Ok(position) = argument.downcast::<Position>() {
                                //let position = (*position).clone();
                                match &mut self.send_to_type_checker {
                                    Some(x) => x
                                        .send(MsgFromManager::Message(TypeMessage::DefIdForPos(
                                            task_id, *position,
                                        )))
                                        .unwrap(),
                                    None => {}
                                }
                            } else {
                                unimplemented!("Internal error: malformed GetDefIdForPosition");
                            }
                        }
                        RecipeStep::GetTypeForDefId => {
                            if let Ok(def_id) = argument.downcast::<DefId>() {
                                match &mut self.send_to_type_checker {
                                    Some(x) => x
                                        .send(MsgFromManager::Message(TypeMessage::TypeForDefId(
                                            task_id, *def_id,
                                        )))
                                        .unwrap(),
                                    None => {}
                                }
                            } else {
                                unimplemented!("Internal error: malformed GetDefIdForPosition");
                            }
                        }
                        RecipeStep::GetCompletionsForDefId => {
                            if let Ok(def_id) = argument.downcast::<DefId>() {
                                match &mut self.send_to_type_checker {
                                    Some(x) => x
                                        .send(MsgFromManager::Message(
                                            TypeMessage::CompletionsForDefId(task_id, *def_id),
                                        ))
                                        .unwrap(),
                                    None => {}
                                }
                            } else {
                                unimplemented!("Internal error: malformed GetCompletionsForDefId");
                            }
                        }
                        RecipeStep::RespondWithType => {
                            if let Ok(ty) = argument.downcast::<String>() {
                                match &mut self.send_to_lsp_responder {
                                    Some(x) => x
                                        .send(MsgFromManager::Message(LspResponse::Type(
                                            task_id, *ty,
                                        )))
                                        .unwrap(),
                                    None => {}
                                }
                            } else {
                                unimplemented!("Internal error: malformed RespondWithCompletion");
                            }
                        }
                        RecipeStep::RespondWithCompletions => {
                            if let Ok(completions) = argument.downcast::<Vec<(String, String)>>() {
                                match &mut self.send_to_lsp_responder {
                                    Some(x) => x
                                        .send(MsgFromManager::Message(LspResponse::Completions(
                                            task_id,
                                            *completions,
                                        )))
                                        .unwrap(),
                                    None => {}
                                }
                            } else {
                                unimplemented!("Internal error: malformed RespondWithCompletion");
                            }
                        }
                        RecipeStep::RespondWithInitialized => {
                            match &mut self.send_to_lsp_responder {
                                Some(x) => x
                                    .send(MsgFromManager::Message(LspResponse::Initialized(
                                        task_id,
                                    )))
                                    .unwrap(),
                                None => {}
                            }
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
                    break;
                }
                Err(_) => {
                    eprintln!("Error during host receive");
                }
            }
        }

        self.stop();
    }

    pub fn start(self) -> std::thread::JoinHandle<()> {
        thread::spawn(move || {
            self.message_loop();
        })
    }

    fn stop(self) {
        // Join all the threads
        let _ = self.handle_for_type_checker.unwrap().join();
    }

    pub fn start_type_checker(&mut self) {
        let tx_for_type: Sender<MsgToManager> = self.send_to_manager.clone();

        let mut type_checker = FakeTypeChecker::new();
        type_checker.startup(Box::new(move |x| {
            tx_for_type.send(MsgToManager::TypeResponse(x)).unwrap()
        }));

        let (actor_tx, handle) = Self::spawn_actor(type_checker);
        self.send_to_type_checker = Some(actor_tx);
        self.handle_for_type_checker = Some(handle);
    }

    pub fn start_lsp_server(&mut self) {
        let mut lsp_responder = LspResponder;

        lsp_responder.startup(Box::new(move |_| {}));

        let (actor_tx, handle) = Self::spawn_actor(lsp_responder);
        self.send_to_lsp_responder = Some(actor_tx);
        self.handle_for_type_checker = Some(handle);
    }

    fn spawn_actor<T: Actor + Send + 'static>(
        mut actor: T,
    ) -> (Sender<MsgFromManager<T::InMessage>>, JoinHandle<()>) {
        let (actor_tx, actor_rx) = channel();

        let handle = thread::spawn(move || loop {
            match actor_rx.recv() {
                Ok(MsgFromManager::Message(message)) => actor.receive_message(message),
                Ok(MsgFromManager::Shutdown) => break,
                Err(_) => {
                    eprintln!("Failure during toplevel Message receive");
                    break;
                }
            }
        });

        (actor_tx, handle)
    }
}
