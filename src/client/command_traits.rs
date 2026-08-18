//! The command families each executor offers, held in one list.
//!
//! Four types execute commands — [`Client`](crate::client::Client),
//! [`ExclusiveClient`](crate::client::ExclusiveClient),
//! [`Pipeline`](crate::client::Pipeline) and
//! [`Transaction`](crate::client::Transaction) — and each used to carry its own
//! hand-written block of empty `impl`s. Nothing checked the four against each
//! other, so a family added to the client and forgotten in the batch executors
//! compiled fine and failed at the call site: that is exactly how the whole
//! vector-set family came to be unusable in a pipeline or a transaction.
//!
//! The 22 data families live in [`data_command_families`] and are written once.
//! What each executor adds on top is named at its own call site, so the
//! differences between the four surfaces are a short list to read rather than
//! four long lists to diff.

/// Invokes `$mac` with `$ty`, the 22 data command families, and any extra
/// families the call site names.
///
/// These are the families that are meaningful on every executor: they read and
/// write data and nothing else. A new command family belongs here unless it
/// steers the connection or the topology, in which case it goes in the extras
/// of the executors that can honour it.
macro_rules! data_command_families {
    ($mac:ident, $ty:ty $(, $extra:ident)* $(,)?) => {
        $mac!(
            $ty,
            ArrayCommands,
            BitmapCommands,
            BloomCommands,
            CountMinSketchCommands,
            CuckooCommands,
            GenericCommands,
            GeoCommands,
            HashCommands,
            HyperLogLogCommands,
            JsonCommands,
            ListCommands,
            ScriptingCommands,
            SearchCommands,
            ServerCommands,
            SetCommands,
            SortedSetCommands,
            StreamCommands,
            StringCommands,
            TDigestCommands,
            TimeSeriesCommands,
            TopKCommands,
            VectorSetCommands,
            $($extra,)*
        );
    };
}

/// Implements each named family for `&'a $ty` — the executors that send a
/// command as soon as it is awaited.
macro_rules! impl_families_for_ref {
    ($ty:ty $(, $family:ident)+ $(,)?) => {
        $(impl<'a> $crate::commands::$family<'a> for &'a $ty {})+
    };
}

/// Implements each named family for `&'a mut $ty` — the batch executors, whose
/// `queue`/`forget` need a unique borrow.
///
/// The receiver is what makes this a separate macro rather than an argument:
/// `BatchPreparedCommand` is implemented for `PreparedCommand<'a, &'a mut _, R>`,
/// so a family implemented for the shared reference compiles here and then fails
/// to resolve `.queue()`.
macro_rules! impl_families_for_mut_ref {
    ($ty:ty $(, $family:ident)+ $(,)?) => {
        $(impl<'a> $crate::commands::$family<'a> for &'a mut $ty {})+
    };
}

/// Implements every command family a client offers, for `&$ty`.
///
/// [`Client`](crate::client::Client) and
/// [`ExclusiveClient`](crate::client::ExclusiveClient) must offer exactly the
/// same commands apart from the two families reserved to an exclusive
/// connection — [`BlockingCommands`](crate::commands::BlockingCommands) and
/// [`TransactionCommands`](crate::commands::TransactionCommands), which are
/// implemented on the exclusive client alone.
///
/// Over the data families a client adds four: cluster and connection management,
/// the internal pub/sub commands the subscription API is built on, and the
/// sentinel ones. `DebugCommands` and `InternalCommands` are test-only here and
/// are not part of the published surface.
macro_rules! impl_shared_command_traits {
    ($ty:ty) => {
        $crate::client::command_traits::data_command_families!(
            impl_families_for_ref,
            $ty,
            ClusterCommands,
            ConnectionCommands,
            InternalPubSubCommands,
            SentinelCommands,
        );

        #[cfg(test)]
        impl<'a> $crate::commands::DebugCommands<'a> for &'a $ty {}

        // The connection-mechanism commands, reachable from the suite and from
        // nowhere else. `InternalCommands` documents why they are not a public
        // family; the tests that cover them talk to a client like any caller
        // would, which is what makes them worth running.
        #[cfg(test)]
        impl<'a> $crate::commands::InternalCommands<'a> for &'a $ty {}
    };
}

/// Implements every command family a pipeline offers, for `&mut $ty`.
///
/// The connection family is here for one command in particular: `CLIENT REPLY
/// OFF` … `CLIENT REPLY ON` around a run of writes is the bulk-load idiom a
/// pipeline exists for, and the suppressed replies are what each command's
/// `forget()` accounts for in the positional matching. The cluster family comes
/// along as introspection that is meaningful mid-batch.
///
/// Neither the pub/sub nor the sentinel family is here. A subscription is
/// answered by push frames rather than by a reply in the batch, and a sentinel
/// command queued on a pipeline would go to the data connection, which is not a
/// sentinel.
macro_rules! impl_pipeline_command_traits {
    ($ty:ty) => {
        $crate::client::command_traits::data_command_families!(
            impl_families_for_mut_ref,
            $ty,
            ClusterCommands,
            ConnectionCommands,
        );
    };
}

/// Implements every command family a transaction offers, for `&mut $ty`.
///
/// The data families and nothing else, which is where a transaction parts
/// company with a pipeline. `CLIENT REPLY`, the reason the connection family is
/// on a pipeline, means nothing inside `MULTI`: every reply is delivered at once
/// in `EXEC`'s array, so there is no per-command reply to suppress. The rest of
/// that family either discards the block outright (`RESET`) or changes the state
/// the queued commands were written against (`SELECT`, `AUTH`).
macro_rules! impl_transaction_command_traits {
    ($ty:ty) => {
        $crate::client::command_traits::data_command_families!(impl_families_for_mut_ref, $ty,);
    };
}

pub(crate) use {
    data_command_families, impl_families_for_mut_ref, impl_families_for_ref,
    impl_pipeline_command_traits, impl_shared_command_traits, impl_transaction_command_traits,
};
