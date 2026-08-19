use crate::{Error, resp::Command};
use std::{fmt, sync::Arc, time::Duration};

/// Sits between a caller and the wire, on every command the client sends.
///
/// This is the extension point for what the crate does not do itself:
/// per-command metrics, a request identifier, structured logging of a slow
/// command, a rate limiter counting what it lets through, an audit trail.
/// [`ClientStats`](crate::client::ClientStats) reports the connection as a
/// whole; this reports commands one by one.
///
/// It is installed with
/// [`Config::interceptor`](crate::client::Config::interceptor):
///
/// ```
/// use rustis::client::{Config, CustomInterceptor, CommandInterceptor};
/// use rustis::resp::Command;
/// use std::{sync::atomic::{AtomicUsize, Ordering}, time::Duration};
///
/// struct CountCommands(AtomicUsize);
///
/// impl CommandInterceptor for CountCommands {
///     fn on_command(&self, _command: &mut Command) {
///         self.0.fetch_add(1, Ordering::Relaxed);
///     }
/// }
///
/// let mut config = Config::default();
/// config.interceptor = Some(CustomInterceptor::new(CountCommands(AtomicUsize::new(0))));
/// ```
///
/// # What it sees
///
/// [`on_command`](Self::on_command) runs on the **caller's** task, once per
/// command, just before the command is handed to the network task — including
/// every command of a pipeline or a transaction, and the subscribe/monitor
/// commands the client sends on its own behalf. It may rewrite the command; the
/// rewritten one is what goes out.
///
/// [`on_complete`](Self::on_complete) runs when the **caller's** future
/// resolves, with the elapsed time and the error the caller sees — a server
/// error and a decode mismatch included, both of which are born after the reply
/// arrives. What has no caller waiting is announced and never concluded:
/// [`send_and_forget`](crate::client::Client::send_and_forget), the
/// subscribe/monitor commands, and the client-side cache's own reads.
///
/// Neither is a place to block or to send a command: both run on a task that is
/// waiting on this one, and a nested send would deadlock a caller waiting for
/// the reply.
///
/// A replay after a reconnection announces the command again — it really is
/// sent again — so a counter here counts wire traffic, not caller intent.
pub trait CommandInterceptor: Send + Sync + 'static {
    /// Called just before `command` is handed to the network task.
    ///
    /// The default does nothing.
    fn on_command(&self, command: &mut Command) {
        let _ = command;
    }

    /// Called when the command named `command_name` resolved, successfully or
    /// not, after `elapsed`.
    ///
    /// `command_name` is the RESP name (`b"GET"`), or empty for a batch, which
    /// resolves as a whole. The default does nothing.
    fn on_complete(&self, command_name: &[u8], elapsed: Duration, error: Option<&Error>) {
        let _ = (command_name, elapsed, error);
    }
}

/// A [`CommandInterceptor`] as held by
/// [`Config::interceptor`](crate::client::Config::interceptor).
///
/// The wrapper exists so a [`Config`](crate::client::Config) stays `Clone` and
/// `Debug`: an interceptor is neither, and its `Debug` says only that one is
/// installed.
#[derive(Clone)]
pub struct CustomInterceptor(Arc<dyn CommandInterceptor>);

impl CustomInterceptor {
    /// Wraps `interceptor`.
    pub fn new(interceptor: impl CommandInterceptor) -> Self {
        Self(Arc::new(interceptor))
    }

    pub(crate) fn get(&self) -> &Arc<dyn CommandInterceptor> {
        &self.0
    }
}

impl fmt::Debug for CustomInterceptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CustomInterceptor")
    }
}
