/// Implements every command family that any client may use, for `&$ty`.
///
/// [`Client`](crate::client::Client) and
/// [`ExclusiveClient`](crate::client::ExclusiveClient) must offer exactly the
/// same commands apart from the two families reserved to an exclusive
/// connection — [`BlockingCommands`](crate::commands::BlockingCommands) and
/// [`TransactionCommands`](crate::commands::TransactionCommands), which are
/// implemented on the exclusive client alone. Holding the shared list here is
/// what stops the two surfaces from drifting apart as families are added.
///
/// The traits are named through `$crate`, so a caller needs nothing in scope.
macro_rules! impl_shared_command_traits {
    ($ty:ty) => {
        impl<'a> $crate::commands::ArrayCommands<'a> for &'a $ty {}
        impl<'a> $crate::commands::BitmapCommands<'a> for &'a $ty {}
        impl<'a> $crate::commands::BloomCommands<'a> for &'a $ty {}
        impl<'a> $crate::commands::ClusterCommands<'a> for &'a $ty {}
        impl<'a> $crate::commands::CountMinSketchCommands<'a> for &'a $ty {}
        impl<'a> $crate::commands::CuckooCommands<'a> for &'a $ty {}
        impl<'a> $crate::commands::ConnectionCommands<'a> for &'a $ty {}
        #[cfg(test)]
        impl<'a> $crate::commands::DebugCommands<'a> for &'a $ty {}
        impl<'a> $crate::commands::GenericCommands<'a> for &'a $ty {}
        impl<'a> $crate::commands::GeoCommands<'a> for &'a $ty {}
        impl<'a> $crate::commands::HashCommands<'a> for &'a $ty {}
        impl<'a> $crate::commands::HyperLogLogCommands<'a> for &'a $ty {}
        impl<'a> $crate::commands::InternalPubSubCommands<'a> for &'a $ty {}
        impl<'a> $crate::commands::JsonCommands<'a> for &'a $ty {}
        impl<'a> $crate::commands::ListCommands<'a> for &'a $ty {}
        impl<'a> $crate::commands::ScriptingCommands<'a> for &'a $ty {}
        impl<'a> $crate::commands::SearchCommands<'a> for &'a $ty {}
        impl<'a> $crate::commands::SentinelCommands<'a> for &'a $ty {}
        impl<'a> $crate::commands::ServerCommands<'a> for &'a $ty {}
        impl<'a> $crate::commands::SetCommands<'a> for &'a $ty {}
        impl<'a> $crate::commands::SortedSetCommands<'a> for &'a $ty {}
        impl<'a> $crate::commands::StreamCommands<'a> for &'a $ty {}
        impl<'a> $crate::commands::StringCommands<'a> for &'a $ty {}
        impl<'a> $crate::commands::TDigestCommands<'a> for &'a $ty {}
        impl<'a> $crate::commands::TimeSeriesCommands<'a> for &'a $ty {}
        impl<'a> $crate::commands::TopKCommands<'a> for &'a $ty {}
        impl<'a> $crate::commands::VectorSetCommands<'a> for &'a $ty {}
    };
}

pub(crate) use impl_shared_command_traits;
