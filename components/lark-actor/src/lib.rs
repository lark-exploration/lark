use std::collections::VecDeque;
use std::sync::mpsc::{channel, Receiver, RecvError, Sender, TryRecvError};
use std::thread;
use url::Url;

use languageserver_types::{Position, Range};

pub type TaskId = usize;

/// Requests to the query system.
#[derive(Debug)]
pub enum QueryRequest {
    TypeAtPosition(TaskId, Url, Position),
    DefinitionAtPosition(TaskId, Url, Position),
    ReferencesAtPosition(TaskId, Url, Position, bool),
    OpenFile(Url, String),
    EditFile(Url, Vec<(Range, String)>),
    Initialize(TaskId),
}
impl QueryRequest {
    /// True if this query will cause us to mutate the state of the
    /// program.
    pub fn is_mutation(&self) -> bool {
        match self {
            QueryRequest::OpenFile(..)
            | QueryRequest::EditFile(..)
            | QueryRequest::Initialize(..) => true,
            QueryRequest::TypeAtPosition(..) => false,
            QueryRequest::DefinitionAtPosition(..) => false,
            QueryRequest::ReferencesAtPosition(..) => false,
        }
    }
}

/// Responses back to the LSP services from
/// the query system.
pub enum LspResponse {
    Type(TaskId, String),
    Range(TaskId, Url, Range),
    Ranges(TaskId, Vec<(Url, Range)>),
    Completions(TaskId, Vec<(String, String)>),
    Initialized(TaskId),
    Nothing(TaskId),
    Diagnostics(Url, Vec<(Range, String)>),
}

/// An actor in the task system. This gives a uniform way to
/// create, control, message, and shutdown concurrent workers.
pub trait Actor {
    type InMessage: Send + Sync + 'static;

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
}

pub struct ActorControl<MessageType: Send + Sync + 'static> {
    pub channel: Sender<MessageType>,
    pub join_handle: std::thread::JoinHandle<()>,
}

pub fn spawn_actor<T: Actor + Send + 'static>(mut actor: T) -> ActorControl<T::InMessage> {
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

enum PushAllPendingError {
    Disconnected,
}

fn push_all_pending<T>(rx: &Receiver<T>, vec: &mut VecDeque<T>) -> Result<(), PushAllPendingError> {
    // If the queue is currently empty, then block until we get at
    // least one message.
    if vec.is_empty() {
        match rx.recv() {
            Ok(m) => vec.push_back(m),
            Err(RecvError) => return Err(PushAllPendingError::Disconnected),
        }
    }

    // Once the queue is non-empty, opportunistically poll for more.
    loop {
        match rx.try_recv() {
            Ok(m) => vec.push_back(m),
            Err(TryRecvError::Empty) => break Ok(()),
            Err(TryRecvError::Disconnected) => break Err(PushAllPendingError::Disconnected),
        }
    }
}
