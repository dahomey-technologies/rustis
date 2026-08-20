use crate::{
    Result,
    client::{MonitorStream, PreparedCommand, prepare_command},
    commands::{LMoveWhere, ZMPopResult, ZWhere},
    resp::{Response, cmd, deserialize_vec_of_triplets},
};
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{DeserializeOwned, Visitor},
};
use std::{fmt, marker::PhantomData};

/// Result for the [`bzpopmin`](BlockingCommands::bzpopmin)
/// and [`bzpopmax`](BlockingCommands::bzpopmax) commands
#[derive(Deserialize)]
pub struct BZpopMinMaxResult<K, E>(
    #[serde(deserialize_with = "deserialize_bzop_min_max_result")] pub Option<Vec<(K, E, f64)>>,
)
where
    K: DeserializeOwned,
    E: DeserializeOwned;

#[allow(clippy::complexity)]
pub(crate) fn deserialize_bzop_min_max_result<'de, D, K, V>(
    deserializer: D,
) -> std::result::Result<Option<Vec<(K, V, f64)>>, D::Error>
where
    D: Deserializer<'de>,
    K: DeserializeOwned,
    V: DeserializeOwned,
{
    struct OptionVisitor<K, V> {
        phantom: PhantomData<(K, V)>,
    }

    impl<'de, K, V> Visitor<'de> for OptionVisitor<K, V>
    where
        K: DeserializeOwned,
        V: DeserializeOwned,
    {
        type Value = Option<Vec<(K, V, f64)>>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("Option<Vec<(K, V, f64)>>")
        }

        fn visit_none<E>(self) -> std::result::Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }

        fn visit_some<D>(self, deserializer: D) -> std::result::Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserialize_vec_of_triplets(deserializer).map(Some)
        }
    }

    deserializer.deserialize_option(OptionVisitor {
        phantom: PhantomData,
    })
}

/// A group of blocking commands
///
/// # ⚠ These commands require a dedicated connection
///
/// A blocking command occupies its connection until it returns. On a shared
/// connection every other caller queued behind it waits for the block to end,
/// however unrelated their commands are: a `BLPOP` with a 30-second timeout
/// stalls everyone for 30 seconds.
///
/// [`command_timeout`](crate::client::Config::command_timeout) does not rescue
/// this. It bounds how long *the caller* waits for a reply, and then returns
/// [`ErrorKind::Timeout`](crate::ErrorKind::Timeout) — the server keeps blocking the
/// connection until the command's own `timeout` argument expires, so the
/// commands queued behind it are still stuck.
///
/// This is why the trait is implemented for
/// [`ExclusiveClient`](crate::client::ExclusiveClient) alone, and not for the
/// clonable [`Client`](crate::client::Client): the dedicated connection is a
/// requirement the type carries, so calling one of these commands on a
/// multiplexed client does not compile.
///
/// ```
/// use rustis::{client::ExclusiveClient, commands::BlockingCommands, Result};
///
/// # async fn example() -> Result<()> {
/// let blocking_client = ExclusiveClient::connect("127.0.0.1:6379").await?;
/// let result: Option<(String, String)> = blocking_client.blpop("key", 30.).await?;
/// # Ok(())
/// # }
/// ```
///
/// A [`PooledClientManager`](crate::client::PooledClientManager) hands out an
/// exclusive client too: the borrowed connection returns to the pool only once
/// the command completes, so the block stays confined to it.
pub trait BlockingCommands<'a>: Sized {
    /// This command is the blocking variant of [`lmove`](crate::commands::ListCommands::lmove).
    ///
    /// **Blocks its connection.** Call it on a client dedicated to blocking
    /// commands — see [the trait documentation](BlockingCommands).
    ///
    /// # Return
    /// the element being popped from `source` and pushed to `destination`.
    /// If timeout is reached, a None reply is returned.
    ///
    /// # See Also
    /// [<https://redis.io/commands/blmove/>](https://redis.io/commands/blmove/)
    #[must_use]
    fn blmove<R: Response>(
        self,
        source: impl Serialize,
        destination: impl Serialize,
        where_from: LMoveWhere,
        where_to: LMoveWhere,
        timeout: f64,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("BLMOVE")
                .key(source)
                .key(destination)
                .arg(where_from)
                .arg(where_to)
                .arg(timeout),
        )
    }

    /// This command is the blocking variant of [`lmpop`](crate::commands::ListCommands::lmpop).
    ///
    /// **Blocks its connection.** Call it on a client dedicated to blocking
    /// commands — see [the trait documentation](BlockingCommands).
    ///
    /// # Return
    /// - None when no element could be popped, and timeout is reached.
    /// - Tuple composed by the name of the key from which elements were popped and the list of popped element
    ///
    /// # See Also
    /// [<https://redis.io/commands/blmpop/>](https://redis.io/commands/blmpop/)
    #[must_use]
    fn blmpop<R: Response + DeserializeOwned>(
        self,
        timeout: f64,
        keys: impl Serialize,
        where_: LMoveWhere,
        count: usize,
    ) -> PreparedCommand<'a, Self, Option<(String, R)>> {
        prepare_command(
            self,
            cmd("BLMPOP")
                .arg(timeout)
                .key_with_count(keys)
                .arg(where_)
                .arg("COUNT")
                .arg(count),
        )
    }

    /// This command is a blocking list pop primitive.
    ///
    /// It is the blocking version of [`lpop`](crate::commands::ListCommands::lpop) because it
    /// blocks the connection when there are no elements to pop from any of the given lists.
    ///
    /// **Blocks its connection.** Call it on a client dedicated to blocking
    /// commands — see [the trait documentation](BlockingCommands).
    ///
    /// An element is popped from the head of the first list that is non-empty,
    /// with the given keys being checked in the order that they are given.
    ///
    /// # Return
    /// - `None` when no element could be popped and the timeout expired
    /// - a tuple with the first element being the name of the key where an element was popped
    ///   and the second element being the value of the popped element.
    ///
    /// # See Also
    /// [<https://redis.io/commands/blpop/>](https://redis.io/commands/blpop/)
    #[must_use]
    fn blpop<R1: Response + DeserializeOwned, R2: Response + DeserializeOwned>(
        self,
        keys: impl Serialize,
        timeout: f64,
    ) -> PreparedCommand<'a, Self, Option<(R1, R2)>> {
        prepare_command(self, cmd("BLPOP").keys(keys).arg(timeout))
    }

    /// This command is a blocking list pop primitive.
    ///
    /// It is the blocking version of [`rpop`](crate::commands::ListCommands::rpop) because it
    /// blocks the connection when there are no elements to pop from any of the given lists.
    ///
    /// **Blocks its connection.** Call it on a client dedicated to blocking
    /// commands — see [the trait documentation](BlockingCommands).
    ///
    /// An element is popped from the tail of the first list that is non-empty,
    /// with the given keys being checked in the order that they are given.
    ///
    /// # Return
    /// - `None` when no element could be popped and the timeout expired
    /// - a tuple with the first element being the name of the key where an element was popped
    ///   and the second element being the value of the popped element.
    ///
    /// # See Also
    /// [<https://redis.io/commands/brpop/>](https://redis.io/commands/brpop/)
    #[must_use]
    fn brpop<R1: Response + DeserializeOwned, R2: Response + DeserializeOwned>(
        self,
        keys: impl Serialize,
        timeout: f64,
    ) -> PreparedCommand<'a, Self, Option<(R1, R2)>> {
        prepare_command(self, cmd("BRPOP").keys(keys).arg(timeout))
    }

    /// This command is the blocking variant of [`zmpop`](crate::commands::SortedSetCommands::zmpop).
    ///
    /// **Blocks its connection.** Call it on a client dedicated to blocking
    /// commands — see [the trait documentation](BlockingCommands).
    ///
    /// # Return
    /// * `None` if no element could be popped
    /// * A tuple made up of
    ///     * The name of the key from which elements were popped
    ///     * An array of tuples with all the popped members and their scores
    ///
    /// # See Also
    /// [<https://redis.io/commands/bzmpop/>](https://redis.io/commands/bzmpop/)
    #[must_use]
    fn bzmpop<R: Response + DeserializeOwned>(
        self,
        timeout: f64,
        keys: impl Serialize,
        where_: ZWhere,
        count: usize,
    ) -> PreparedCommand<'a, Self, Option<ZMPopResult<R>>> {
        prepare_command(
            self,
            cmd("BZMPOP")
                .arg(timeout)
                .key_with_count(keys)
                .arg(where_)
                .arg("COUNT")
                .arg(count),
        )
    }

    /// This command is the blocking variant of [`zpopmax`](crate::commands::SortedSetCommands::zpopmax).
    ///
    /// **Blocks its connection.** Call it on a client dedicated to blocking
    /// commands — see [the trait documentation](BlockingCommands).
    ///
    /// # Return
    /// * `None` when no element could be popped and the timeout expired.
    /// * The list of tuple with
    ///     * the first element being the name of the key where a member was popped,
    ///     * the second element is the popped member itself,
    ///     * and the third element is the score of the popped element.
    ///
    /// # See Also
    /// [<https://redis.io/commands/bzpopmax/>](https://redis.io/commands/bzpopmax/)
    #[must_use]
    fn bzpopmax<R1: Response + DeserializeOwned, R2: Response + DeserializeOwned>(
        self,
        keys: impl Serialize,
        timeout: f64,
    ) -> PreparedCommand<'a, Self, BZpopMinMaxResult<R1, R2>> {
        prepare_command(self, cmd("BZPOPMAX").keys(keys).arg(timeout))
    }

    /// This command is the blocking variant of [`zpopmin`](crate::commands::SortedSetCommands::zpopmin).
    ///
    /// **Blocks its connection.** Call it on a client dedicated to blocking
    /// commands — see [the trait documentation](BlockingCommands).
    ///
    /// # Return
    /// * `None` when no element could be popped and the timeout expired.
    /// * The list of tuple with
    ///     * the first element being the name of the key where a member was popped,
    ///     * the second element is the popped member itself,
    ///     * and the third element is the score of the popped element.
    ///
    /// # See Also
    /// [<https://redis.io/commands/bzpopmin/>](https://redis.io/commands/bzpopmin/)
    #[must_use]
    fn bzpopmin<R1: Response + DeserializeOwned, R2: Response + DeserializeOwned>(
        self,
        keys: impl Serialize,
        timeout: f64,
    ) -> PreparedCommand<'a, Self, BZpopMinMaxResult<R1, R2>> {
        prepare_command(self, cmd("BZPOPMIN").keys(keys).arg(timeout))
    }

    /// Debugging command that streams back every command processed by the Redis server.
    ///
    /// **Takes over its connection for as long as the stream lives**, which is
    /// until the returned [`MonitorStream`](crate::client::MonitorStream) is
    /// closed or dropped — either sends `RESET` to leave monitor mode. Call it
    /// on a client dedicated to it — see
    /// [the trait documentation](BlockingCommands).
    ///
    /// # See Also
    /// [<https://redis.io/commands/monitor/>](https://redis.io/commands/monitor/)
    #[must_use]
    #[allow(async_fn_in_trait)]
    async fn monitor(self) -> Result<MonitorStream>;
}
