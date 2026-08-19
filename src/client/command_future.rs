use crate::{
    Error, ErrorKind, Result, TimeoutKind,
    client::{Client, CommandInterceptor},
    network::{ResultReceiver, TimeoutFuture},
    resp::{Command, RespResponse},
};
use bytes::Bytes;
use pin_project_lite::pin_project;
use serde::de::DeserializeOwned;
use std::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Instant,
};

/// What the probe needs to identify a command, outside a test build: nothing.
/// The label keeps a uniform shape through [`Client::finish_send`] so the two
/// builds share one code path instead of two `cfg` arms.
#[cfg(test)]
pub(crate) type ProbeLabel = String;
#[cfg(not(test))]
pub(crate) type ProbeLabel = ();

/// Hands one observation to the response-shape probe, and nothing at all
/// outside a test build.
#[cfg(test)]
#[inline]
pub(crate) fn record_probe<T>(probe_label: ProbeLabel, response: &RespResponse, decoded: bool) {
    crate::tests::response_probe::record(
        probe_label,
        std::any::type_name::<T>(),
        response,
        decoded,
    );
}

#[cfg(not(test))]
#[inline]
pub(crate) fn record_probe<T>(_probe_label: ProbeLabel, _response: &RespResponse, _decoded: bool) {}

pin_project! {
    /// The future `client.get("key").await` drives.
    ///
    /// Written by hand rather than produced by an `async` block, because the
    /// associated type of [`IntoFuture`](std::future::IntoFuture) must be
    /// named: an `async` block has no name, so the only way to hand one back is
    /// to box it — one heap allocation and one virtual call on the path every
    /// documented example takes. This state machine lives in the caller's frame
    /// instead.
    ///
    /// It runs the same three steps as [`Client::send`]: hand the command to
    /// the network task, wait for the reply on a oneshot — under
    /// `command_timeout` when one is configured — then deserialize it into `R`.
    /// Like any future, it does nothing until first polled: a `CommandFuture`
    /// built and dropped never reaches the server.
    pub struct CommandFuture<'a, R> {
        #[pin]
        state: State<'a>,
        // Name of the command being awaited, used to name the error a failure
        // produces, wherever it is born.
        command_name: Option<Bytes>,
        probe_label: Option<ProbeLabel>,
        // Both `None` unless an interceptor is installed, which is what keeps
        // the clock unread on the path that has nobody to report to. Taken at
        // the first poll rather than at construction: a future built and never
        // polled sends nothing, so it lasted no time.
        interceptor: Option<Arc<dyn CommandInterceptor>>,
        started_at: Option<Instant>,
        phantom: PhantomData<fn() -> R>,
    }
}

pin_project! {
    /// Not sent yet, waiting on the reply, or already holding the reason there
    /// will be none.
    #[project = StateProj]
    pub(crate) enum State<'a> {
        /// Built but never polled, so the command is still in hand.
        Unsent {
            client: &'a Client,
            command: Option<Command>,
            retry_on_error: Option<bool>,
        },
        /// The command never reached the network task: `send_message` failed,
        /// and the error waits here for the first poll.
        Failed { error: Option<Error> },
        /// `command_timeout` is disabled: the oneshot is awaited bare.
        Waiting { receiver: ResultReceiver },
        Timed {
            #[pin]
            receiver: TimeoutFuture<ResultReceiver>,
        },
    }
}

impl<'a, R> CommandFuture<'a, R> {
    pub(crate) fn new(
        client: &'a Client,
        command: Command,
        retry_on_error: Option<bool>,
    ) -> CommandFuture<'a, R> {
        CommandFuture {
            state: State::Unsent {
                client,
                command: Some(command),
                retry_on_error,
            },
            command_name: None,
            probe_label: None,
            interceptor: None,
            started_at: None,
            phantom: PhantomData,
        }
    }
}

impl<R: DeserializeOwned> Future for CommandFuture<'_, R> {
    type Output = Result<R>;

    #[expect(
        clippy::unreachable,
        reason = "a future polled after it returned `Ready` is a caller bug, and \
                  the alternative — returning `Pending` forever — hangs silently"
    )]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();

        // First poll: this is where the command reaches the network task, so
        // that a future built and dropped sends nothing.
        let unsent = match this.state.as_mut().project() {
            StateProj::Unsent {
                client,
                command,
                retry_on_error,
            } => command
                .take()
                .map(|command| (*client, command, *retry_on_error)),
            _ => None,
        };

        if let Some((client, command, retry_on_error)) = unsent {
            *this.interceptor = client.interceptor().cloned();
            *this.started_at = this.interceptor.as_ref().map(|_| Instant::now());
            let (state, command_name, probe_label) = client.start_send(command, retry_on_error);
            *this.command_name = command_name;
            *this.probe_label = Some(probe_label);
            this.state.as_mut().set(state);
        }

        let response: Result<RespResponse> = match this.state.project() {
            StateProj::Unsent { .. } => unreachable!("`CommandFuture` polled after it completed"),
            StateProj::Failed { error } => match error.take() {
                Some(error) => Err(error),
                None => unreachable!("`CommandFuture` polled after it completed"),
            },
            StateProj::Waiting { receiver } => match Pin::new(receiver).poll(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(result) => result.map_err(Error::from).and_then(|response| response),
            },
            StateProj::Timed { receiver } => match receiver.poll(cx) {
                Poll::Pending => return Poll::Pending,
                // Expiry is the one failure the network task never sees, so it
                // is also the one it never names: the name is applied here.
                Poll::Ready(Err(_)) => Client::name_command(
                    Err(Error::from(ErrorKind::Timeout(TimeoutKind::Command))),
                    this.command_name.clone(),
                ),
                Poll::Ready(Ok(result)) => {
                    result.map_err(Error::from).and_then(|response| response)
                }
            },
        };

        let Some(probe_label) = this.probe_label.take() else {
            unreachable!("`CommandFuture` polled after it completed")
        };

        let command_name = this.command_name.take();
        let result = match response {
            Err(e) => Err(e),
            Ok(response) => Client::finish_send::<R>(&response, command_name.clone(), probe_label),
        };

        // Announced here rather than at the reply: a server error and a decode
        // mismatch are both born above, and an interceptor that missed them
        // would report a command as successful that its caller sees fail.
        if let Some(interceptor) = this.interceptor.take()
            && let Some(started_at) = this.started_at.take()
        {
            interceptor.on_complete(
                command_name.as_ref().map_or(&[][..], Bytes::as_ref),
                started_at.elapsed(),
                result.as_ref().err(),
            );
        }

        Poll::Ready(result)
    }
}
