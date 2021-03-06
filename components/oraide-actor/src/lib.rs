// This file is part of oraide.  See <https://github.com/Phrohdoh/oraide>.
// 
// oraide is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License version 3
// as published by the Free Software Foundation.
// 
// oraide is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
// 
// You should have received a copy of the GNU Affero General Public License
// along with oraide.  If not, see <https://www.gnu.org/licenses/>.

use std::{
    thread,
    collections::{
        VecDeque,
    },
    sync::mpsc::{
        Sender,
        Receiver,
        RecvError,
        TryRecvError,
        channel,
    },
};

use url::Url;

use languageserver_types::{
    Position as LsPos,
    Range as LsRange,
};

mod ls_types;
pub use ls_types::{
    Position,
    Range,
    RangedFilePosition,
    Symbol,
};

pub type TaskId = usize;

#[derive(Debug)]
pub enum QueryRequest {
    Initialize {
        task_id: TaskId,
        workspace_root_url: Option<Url>,
    },
    HoverAtPosition {
        task_id: TaskId,
        file_url: Url,
        file_pos: LsPos,
    },
    GoToDefinition {
        task_id: TaskId,
        file_url: Url,
        file_pos: LsPos,
    },
    FileOpened {
        file_url: Url,
        file_text: String,
    },
    FileChanged {
        file_url: Url,
        changes: Vec<(LsRange, String)>,
    },
    FileSymbols {
        task_id: TaskId,
        file_url: Url,
    },
}

impl QueryRequest {
    pub fn will_mutate_server_state(&self) -> bool {
        match self {
            QueryRequest::Initialize { .. }
            | QueryRequest::FileOpened { .. }
            | QueryRequest::FileChanged { .. }
                => true,
            QueryRequest::HoverAtPosition { .. }
            | QueryRequest::GoToDefinition { .. }
            | QueryRequest::FileSymbols { .. }
                => false,
        }
    }
}

pub enum QueryResponse {
    Nothing {
        task_id: TaskId,
    },
    AckInitialize {
        task_id: TaskId,
    },
    HoverData {
        task_id: TaskId,
        data: String,
    },
    Definition {
        task_id: TaskId,
        ranged_file_position: Option<RangedFilePosition>,
    },
    DocumentSymbols {
        task_id: TaskId,
        symbols: Vec<Symbol>,
    },
}

/// An actor in the task system.  This gives us a uniform way to
/// create, control, message, and shut down concurrent workers.
pub trait Actor {
    type Input: Send + Sync + 'static;

    /// Invoked when new message(s) arrive.  Contains all of the messages that
    /// can be pulled at this time.  The actor is free to process as many as
    /// they like.  So long as messages remain in the queue, we'll just keep
    /// invoking this function (possibly appending more messages to the back).
    /// Once the queue is empty, we'll block until we can fetch more.
    ///
    /// The intended workflow is as follows:
    ///
    /// - If desired, inspect `messages` and prune messages that become
    ///   outdated due to later messages in the queue
    /// - Invoke `messages.pop_front().unwrap()` and process that message
    ///   - In particular, it is probably better to return than to eagerly
    ///     process all messages in the queue, as it gives the actor a chance
    ///     to add more messages if they have arrived in the meantime
    ///     - This is only important if you are trying to remove
    ///       outdated messages
    fn on_new_messages(&mut self, messages: &mut VecDeque<Self::Input>);
}

/// # Type Parameters
/// - `M`: The message type to be sent over `channel`
pub struct ActorControl<M: Send + Sync + 'static> {
    pub channel: Sender<M>,
    pub join_handle: thread::JoinHandle<()>,
}

pub fn spawn_actor<T: Actor + Send + 'static>(mut actor: T) -> ActorControl<T::Input> {
    let (actor_tx, actor_rx) = channel();
    let mut queue = VecDeque::default();

    let join_handle = thread::spawn(move || loop {
        match push_all_pending(&actor_rx, &mut queue) {
            Ok(()) => actor.on_new_messages(&mut queue),
            Err(PushAllPendingError::Disconnected) => {
                eprintln!("Failure during top-level message receive");

                break;
            },
        }
    });

    ActorControl {
        channel: actor_tx,
        join_handle,
    }
}

enum PushAllPendingError {
    Disconnected,
}

fn push_all_pending<T>(rx: &Receiver<T>, queue: &mut VecDeque<T>) -> Result<(), PushAllPendingError> {
    // If the queue is currently empty, block until we get at least one message
    if queue.is_empty() {
        match rx.recv() {
            Ok(m) => queue.push_back(m),
            Err(RecvError) => return Err(PushAllPendingError::Disconnected),
        }
    }

    // Once the queue is non-empty, opportunistically poll for more
    loop {
        match rx.try_recv() {
            Ok(m) => queue.push_back(m),
            Err(TryRecvError::Empty) => break Ok(()),
            Err(TryRecvError::Disconnected) => break Err(PushAllPendingError::Disconnected),
        }
    }
}