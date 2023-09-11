use crate::{
    client::{prepare_command, PreparedCommand},
    resp::cmd,
};
use std::time::Duration;

/// A group of Redis commands related to Debug functionality of redis
/// # See Also
/// [Redis Debug Commands](https://redis.io/commands/debug/)
/// The DEBUG command is an internal command. It is meant to be used
/// for developing and testing Redis and libraries.
pub trait DebugCommands<'a> {
    /// Stop the server for <seconds>. Decimals allowed.
    #[must_use]
    fn debug_sleep(self, duration: Duration) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("DEBUG").arg("SLEEP").arg(duration.as_secs_f32()))
    }

    /// Graceful restart: save config, db, restart after a <milliseconds> delay (default 0).
    #[must_use]
    fn debug_restart(self, delay: Option<Duration>) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
    {
        prepare_command(
            self,
            cmd("DEBUG")
                .arg("RESTART")
                .arg(delay.map(|d| u64::try_from(d.as_millis()).unwrap())),
        )
    }

    /// Hard crash and restart after a <milliseconds> delay (default 0).
    #[must_use]
    fn debug_crash_and_recover(self, delay: Option<Duration>) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
    {
        prepare_command(
            self,
            cmd("DEBUG")
                .arg("CRASH-AND-RECOVER")
                .arg(delay.map(|d| u64::try_from(d.as_millis()).unwrap())),
        )
    }

    /// Crash the server by assertion failed.
    #[must_use]
    fn debug_assert(self) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("DEBUG").arg("ASSERT"))
    }

    /// Crash the server simulating an out-of-memory error.
    #[must_use]
    fn debug_oom(self) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("DEBUG").arg("OOM"))
    }

    /// Crash the server simulating a panic.
    #[must_use]
    fn debug_panic(self) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("DEBUG").arg("PANIC"))
    }
}
