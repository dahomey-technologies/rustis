//! A command in flight across the shards, and how the replies it draws become
//! one answer.
//!
//! A cluster command is not one request/reply pair. Routing may split it over
//! several nodes, each answering on its own connection, and the caller is owed a
//! single response that looks like the one a standalone server would have sent.
//! Everything that reads the sub-replies lives here: the redirections they
//! carry, the transient errors that ask for a replay, and the per-command
//! aggregation rule Redis publishes as a response policy.
//!
//! Nothing in this module touches a connection or the topology. A request holds
//! its own answers, so the whole reassembly can be driven from a value built in
//! a test.

use super::cluster_topology::NodeId;
use crate::{
    ClientError, Error, ErrorKind, RedisError, RedisErrorKind, Result, RetryReason, RetryReasons,
    commands::ResponsePolicy,
    resp::{Command, RespResponse, RespView},
};
use bytes::Bytes;
use smallvec::{SmallVec, smallvec};
use std::{
    collections::{HashMap, VecDeque},
    iter::zip,
    time::Duration,
};
use tracing::{debug, trace};

#[derive(Debug)]
pub(super) struct SubRequest {
    pub node_id: NodeId,
    pub keys: SmallVec<[Bytes; 10]>,
    pub result: Option<Option<Result<RespResponse>>>,
}

impl SubRequest {
    /// A sub-request of a command that names no key, so nothing of the reply has
    /// to be lined up against one: a broadcast, or a command routed by its
    /// channel rather than by a slot.
    pub(super) fn keyless(node_id: NodeId) -> Self {
        Self {
            node_id,
            keys: SmallVec::new(),
            result: None,
        }
    }
}

#[derive(Debug)]
pub(super) struct RequestInfo {
    pub response_policy: Option<ResponsePolicy>,
    pub keys: SmallVec<[Bytes; 10]>,
    pub sub_requests: SmallVec<[SubRequest; 10]>,
    /// The command the sub-requests were derived from, kept only when a single
    /// sub-request can be re-sent on its own — i.e. when the command was split
    /// across several shards. Everything else is retried as a whole and does not
    /// pay for the clone.
    pub command: Option<Command>,
    /// Whether the command is a subscription one, whose answer is a push frame
    /// the network handler consumes on its own. See `retire_pub_sub_request`.
    pub is_pub_sub: bool,
    #[allow(unused)]
    #[cfg(test)]
    pub command_seq: usize,
}

/// Delay observed before replaying a command the cluster answered `TRYAGAIN`
/// to. The slot is being migrated: the hand-over of a single key is short, so a
/// brief pause is enough for the retry to land on the settled side.
pub(super) const TRY_AGAIN_DELAY: Duration = Duration::from_millis(25);

/// Delay observed before replaying a command the cluster answered `CLUSTERDOWN`
/// to. This one waits on a failover, which is decided in seconds rather than
/// milliseconds, so retrying sooner would only spend the message's attempts.
pub(super) const CLUSTER_DOWN_DELAY: Duration = Duration::from_millis(250);

/// The retry a transient cluster error calls for, or `None` for a server error
/// that belongs to the caller.
///
/// Both kinds report a command that was *not* executed — a slot in migration
/// whose keys are split across two nodes, or a shard momentarily without a
/// master — and the cluster spec asks the client to absorb them instead of
/// surfacing them, since they are what a routine resharding or failover
/// produces.
pub(super) fn transient_retry_reason(kind: &RedisErrorKind) -> Option<RetryReason> {
    match kind {
        RedisErrorKind::TryAgain => Some(RetryReason::TryAgain {
            delay: TRY_AGAIN_DELAY,
            refresh_topology: false,
        }),
        RedisErrorKind::ClusterDown => Some(RetryReason::TryAgain {
            delay: CLUSTER_DOWN_DELAY,
            refresh_topology: true,
        }),
        _ => None,
    }
}

/// Collects the ASK/MOVED redirections carried by a fulfilled request, paired
/// with the sub-request that received them.
pub(super) fn collect_redirections(
    request_info: &RequestInfo,
) -> SmallVec<[(usize, RetryReason); 1]> {
    let mut redirections = SmallVec::<[(usize, RetryReason); 1]>::new();

    for (idx, sub_request) in request_info.sub_requests.iter().enumerate() {
        let Some(Some(Ok(result))) = &sub_request.result else {
            continue;
        };

        let Ok(RespView::Error(error)) = result.view() else {
            continue;
        };

        match RedisError::try_from(error) {
            Ok(RedisError {
                kind: RedisErrorKind::Ask { hash_slot, address },
                ..
            }) => redirections.push((idx, RetryReason::Ask { hash_slot, address })),
            Ok(RedisError {
                kind: RedisErrorKind::Moved { hash_slot, address },
                ..
            }) => redirections.push((idx, RetryReason::Moved { hash_slot, address })),
            _ => (),
        }
    }

    redirections
}

/// The requests fed to the cluster and still awaiting their replies, oldest
/// first.
///
/// The order is what the caller sees: a reply is reported only once every
/// request ahead of it is answered, so a request that can never complete blocks
/// every later one. That is why an orphaned front request is failed rather than
/// waited on, and why a subscription's request — acknowledged by a push frame
/// nothing files — is retired instead of left at the head.
#[derive(Debug, Default)]
pub(super) struct RequestQueue {
    pending: VecDeque<RequestInfo>,
}

impl RequestQueue {
    pub(super) fn push(&mut self, request_info: RequestInfo) {
        self.pending.push_back(request_info);
    }

    /// Puts a partially redirected request back at the head, where it was: its
    /// place in the order is what the caller's reply ordering rests on.
    pub(super) fn push_front(&mut self, request_info: RequestInfo) {
        self.pending.push_front(request_info);
    }

    pub(super) fn clear(&mut self) {
        self.pending.clear();
    }

    /// Whether the oldest request is still owed a reply by a node the topology
    /// no longer holds — a wait that will never end.
    pub(super) fn front_awaits_a_missing_node(&self, holds: impl Fn(&NodeId) -> bool) -> bool {
        let Some(request_info) = self.pending.front() else {
            return false;
        };

        request_info
            .sub_requests
            .iter()
            .any(|sr| sr.result.is_none() && !holds(&sr.node_id))
    }

    /// Test-only: the node the oldest request is still waiting on.
    #[cfg(test)]
    pub(super) fn front_awaited_node_id(&self) -> Option<NodeId> {
        self.pending
            .front()?
            .sub_requests
            .iter()
            .find(|sr| sr.result.is_none())
            .map(|sr| sr.node_id.clone())
    }

    pub(super) fn pop_front(&mut self) {
        self.pending.pop_front();
    }

    /// Takes the oldest request once every one of its sub-requests has answered.
    /// `None` while any is still outstanding — reporting it then would resolve
    /// the caller on a partial reply.
    pub(super) fn take_fulfilled_front(&mut self) -> Option<RequestInfo> {
        let request_info = self.pending.front()?;
        if !request_info
            .sub_requests
            .iter()
            .all(|sr| sr.result.is_some())
        {
            return None;
        }

        trace!("fulfilled request_info: {request_info:?}");
        self.pending.pop_front()
    }

    /// Files `result` against the oldest sub-request still awaiting a reply from
    /// `node_id`, and reports whether one was found.
    ///
    /// The search and the store are one step. Split in two, the indices the
    /// first produced could name nothing by the time the second ran, which is a
    /// case the caller then has to have an answer for — an answer that is
    /// unreachable and therefore untested.
    pub(super) fn file_reply(
        &mut self,
        node_id: &NodeId,
        result: Option<crate::Result<RespResponse>>,
    ) -> bool {
        let Some(sub_request) = self.pending.iter_mut().find_map(|request| {
            request
                .sub_requests
                .iter_mut()
                .find(|sr| sr.node_id == *node_id && sr.result.is_none())
        }) else {
            return false;
        };

        sub_request.result = Some(result);
        true
    }

    /// Drops the request a subscription command left behind, now that the server
    /// has acknowledged it with a push frame.
    ///
    /// Without this the request waits for a reply that never comes and blocks
    /// every later one. Only a subscription acknowledgement retires a request:
    /// an error reply such as `MOVED` is filed like any other, so the
    /// redirection path keeps working.
    pub(super) fn retire_pub_sub(&mut self, node_id: &NodeId) {
        let Some(index) = self.pending.iter().position(|request| {
            request.is_pub_sub
                && request
                    .sub_requests
                    .iter()
                    .any(|sr| sr.node_id == *node_id && sr.result.is_none())
        }) else {
            return;
        };

        self.pending.remove(index);
    }
}

/// A subscription command is acknowledged by a push frame, not by an ordinary
/// reply: `read` hands it to the network handler, which matches it against the
/// caller itself. Nothing therefore ever fills the sub-request the connection
/// filed for it.
pub(super) fn is_pub_sub_command(command: &Command) -> bool {
    matches!(
        command.name(),
        b"SUBSCRIBE"
            | b"PSUBSCRIBE"
            | b"SSUBSCRIBE"
            | b"UNSUBSCRIBE"
            | b"PUNSUBSCRIBE"
            | b"SUNSUBSCRIBE"
    )
}

impl RequestInfo {
    /// The bookkeeping a routed command owes, once routing has said which nodes
    /// it reached.
    ///
    /// Everything but the sub-requests is read off the command here, so the five
    /// routing policies cannot describe the same command differently — which
    /// they would have no way of noticing, each field being read by a different
    /// part of the reassembly.
    pub(super) fn new(command: &Command, sub_requests: SmallVec<[SubRequest; 10]>) -> Self {
        Self::with_keys(command, sub_requests, command.keys().collect())
    }

    /// The same, for a command that reached a single node.
    ///
    /// Such a command owes no key list, at either level. A key list has one
    /// reader: the reassembly that lines one node's replies up against another's,
    /// so that the caller gets its keys back in the order it named them. With one
    /// node there is nothing to line up — `no_response_policy` hands the single
    /// sub-reply back untouched, and every `ResponsePolicy` branch ignores the
    /// keys outright. Collecting them clones one `Bytes` per key, twice, for a
    /// list nothing reads.
    pub(super) fn single_shard(command: &Command, sub_request: SubRequest) -> Self {
        Self::with_keys(command, smallvec![sub_request], SmallVec::new())
    }

    fn with_keys(
        command: &Command,
        sub_requests: SmallVec<[SubRequest; 10]>,
        keys: SmallVec<[Bytes; 10]>,
    ) -> Self {
        Self {
            response_policy: command.response_policy(),
            keys,
            sub_requests,
            command: None,
            is_pub_sub: is_pub_sub_command(command),
            #[cfg(test)]
            command_seq: command.command_seq,
        }
    }

    /// Keeps the command, so that a sub-request redirected on its own can be
    /// re-sent without replaying the whole thing.
    ///
    /// Only worth the clone when the command was actually split: a single
    /// sub-request is retried as a whole by the caller. The shards that already
    /// answered must not run it twice — a replayed `DEL` reports 0 for the keys
    /// it deleted the first time.
    pub(super) fn replayable_per_shard(mut self, command: &Command) -> Self {
        if self.sub_requests.len() > 1 {
            self.command = Some(command.clone());
        }
        self
    }

    /// Turns the collected sub-replies into the single answer the caller is
    /// owed, or into the retry reasons that say the command never ran.
    ///
    /// `None` means the request is not answerable: a sub-request still waiting,
    /// or a node stream that ended. The caller reads that as a disconnection.
    pub(super) fn into_reply(mut self) -> Option<Result<RespResponse>> {
        let mut sub_results = Vec::<Result<RespResponse>>::with_capacity(self.sub_requests.len());
        let mut retry_reasons = SmallVec::<[RetryReason; 1]>::new();

        for sub_request in self.sub_requests.iter_mut() {
            // A sub-request still waiting for its result, or one whose node stream
            // ended, leaves nothing to aggregate.
            let result = sub_request.result.take()??;

            if let Ok(result) = result {
                match result.view() {
                    Ok(RespView::Error(error)) => match RedisError::try_from(error) {
                        Ok(RedisError {
                            kind: RedisErrorKind::Ask { hash_slot, address },
                            description: _,
                        }) => retry_reasons.push(RetryReason::Ask {
                            hash_slot,
                            address: address.clone(),
                        }),
                        Ok(RedisError {
                            kind: RedisErrorKind::Moved { hash_slot, address },
                            description: _,
                        }) => retry_reasons.push(RetryReason::Moved {
                            hash_slot,
                            address: address.clone(),
                        }),
                        // `TRYAGAIN` / `CLUSTERDOWN`: the command did not run,
                        // so it is replayed rather than reported to the caller.
                        Ok(RedisError { kind, .. }) => match transient_retry_reason(&kind) {
                            Some(reason) => retry_reasons.push(reason),
                            None => sub_results.push(Ok(result)),
                        },
                        _ => sub_results.push(Ok(result)),
                    },
                    _ => sub_results.push(Ok(result)),
                }
            } else {
                sub_results.push(result);
            }
        }

        if !retry_reasons.is_empty() {
            debug!(
                "read failed and will be retried. reasons: {:?}",
                retry_reasons
            );
            return Some(Err(Error::from(ErrorKind::Retry(RetryReasons::new(
                retry_reasons,
            )))));
        }

        // The response_policy tip is set for commands that reply with scalar data types,
        // or when it's expected that clients implement a non-default aggregate.
        let Some(response_policy) = &self.response_policy else {
            return no_response_policy(sub_results, &self);
        };

        match response_policy {
            ResponsePolicy::OneSucceeded => response_policy_one_succeeded(sub_results),
            ResponsePolicy::AllSucceeded => response_policy_all_succeeded(sub_results),
            ResponsePolicy::AggLogicalAnd => {
                response_policy_agg(sub_results, |a, b| i64::from(a == 1 && b == 1))
            }
            ResponsePolicy::AggLogicalOr => {
                response_policy_agg(sub_results, |a, b| if a == 0 && b == 0 { 0 } else { 1 })
            }
            ResponsePolicy::AggMin => response_policy_agg(sub_results, i64::min),
            ResponsePolicy::AggMax => response_policy_agg(sub_results, i64::max),
            ResponsePolicy::AggSum => {
                // The operands are integers the shards sent, so the sum is
                // driven by server data. Saturating keeps an implausible total
                // implausible instead of wrapping it into a small one.
                response_policy_agg(sub_results, i64::saturating_add)
            }
            ResponsePolicy::Special => response_policy_special(sub_results),
        }
    }
}

fn response_policy_one_succeeded(
    sub_results: Vec<Result<RespResponse>>,
) -> Option<Result<RespResponse>> {
    let mut result: Result<RespResponse> = Ok(RespResponse::null());

    for sub_result in sub_results {
        match &sub_result {
            Err(_) => result = sub_result,
            Ok(resp_buf) if resp_buf.is_error() => result = sub_result,
            _ => return Some(sub_result),
        }
    }

    Some(result)
}

fn response_policy_all_succeeded(
    sub_results: Vec<Result<RespResponse>>,
) -> Option<Result<RespResponse>> {
    let mut result: Result<RespResponse> = Ok(RespResponse::null());

    for sub_result in sub_results {
        match &sub_result {
            Err(_) => return Some(sub_result),
            Ok(resp_buf) if resp_buf.is_error() => return Some(sub_result),
            _ => result = sub_result,
        }
    }

    Some(result)
}

enum Integer {
    Single(i64),
    Array(Vec<i64>),
    Null,
}

fn response_policy_agg<F>(
    sub_results: Vec<Result<RespResponse>>,
    f: F,
) -> Option<Result<RespResponse>>
where
    F: Fn(i64, i64) -> i64,
{
    let mut integer = Integer::Null;

    for sub_result in sub_results {
        let Ok(sub_result) = sub_result else {
            return Some(sub_result);
        };

        let view = match sub_result.view() {
            Ok(view) => view,
            Err(e) => return Some(Err(e)),
        };
        match view {
            RespView::Integer(i, _) => match &mut integer {
                Integer::Single(current) => *current = f(*current, i),
                Integer::Null => integer = Integer::Single(i),
                Integer::Array(_) => {
                    return Some(Err(Error::from(ClientError::IncompatibleShardReplies)));
                }
            },
            RespView::Array(resp_array)
            | RespView::Set(resp_array)
            | RespView::Push(resp_array) => {
                match &mut integer {
                    Integer::Single(_) => {
                        return Some(Err(Error::from(ClientError::IncompatibleShardReplies)));
                    }
                    Integer::Array(items) => {
                        // Unequal per-shard array lengths must not be silently
                        // truncated by `zip`: an uncombined tail would be a wrong
                        // aggregate reported as success.
                        if items.len() != resp_array.len() {
                            return Some(Err(Error::from(ClientError::IncompatibleShardReplies)));
                        }
                        for (item, view) in items.iter_mut().zip(resp_array) {
                            match view {
                                Ok(RespView::Integer(i, _)) => *item = f(*item, i),
                                Ok(_) => {
                                    return Some(Err(Error::from(
                                        ClientError::IncompatibleShardReplies,
                                    )));
                                }
                                Err(e) => return Some(Err(e)),
                            }
                        }
                    }
                    Integer::Null => {
                        let mut int_array = Vec::with_capacity(resp_array.len());

                        for view in resp_array {
                            match view {
                                Ok(RespView::Integer(i, _)) => int_array.push(i),
                                Ok(_) => {
                                    return Some(Err(Error::from(
                                        ClientError::IncompatibleShardReplies,
                                    )));
                                }
                                Err(e) => return Some(Err(e)),
                            }
                        }

                        integer = Integer::Array(int_array)
                    }
                }
            }
            _ => return Some(Err(Error::from(ClientError::IncompatibleShardReplies))),
        }
    }

    match integer {
        Integer::Single(i) => Some(Ok(RespResponse::integer(i))),
        Integer::Array(v) => Some(Ok(RespResponse::integer_array(v))),
        Integer::Null => Some(Ok(RespResponse::null())),
    }
}

fn response_policy_special(
    _sub_results: Vec<Result<RespResponse>>,
) -> Option<Result<RespResponse>> {
    Some(Err(Error::from(ClientError::CommandNotSupportedInCluster)))
}

fn no_response_policy(
    sub_results: Vec<Result<RespResponse>>,
    request_info: &RequestInfo,
) -> Option<Result<RespResponse>> {
    trace!("no_response_policy");

    if sub_results.len() == 1 {
        // when there is a single sub request, we just read the response
        // on the right connection. For example, GET's reply
        sub_results.into_iter().next()
    } else if request_info.keys.is_empty() {
        // The command doesn't accept key name arguments:
        // the client can aggregate all replies within a single nested data structure.
        // For example, the array replies we get from calling KEYS against all shards.
        // These should be packed in a single array in no particular order.
        let mut results = Vec::<RespResponse>::new();
        for sub_result in sub_results {
            // Propagate the shard's failure as a failure. Returning `None` here
            // would mean "disconnected" to the network handler, which would
            // reconnect the whole cluster over what is merely one shard
            // answering an error.
            let iter = match sub_result.and_then(RespResponse::into_collection_iter) {
                Ok(iter) => iter,
                Err(e) => return Some(Err(e)),
            };
            for item in iter {
                match item {
                    Ok(item) => results.push(item),
                    Err(e) => return Some(Err(e)),
                }
            }
        }

        Some(Ok(RespResponse::owned_array(results)))
    } else {
        // For commands that accept one or more key name arguments:
        // the client needs to retain the same order of replies as the input key names.
        // For example, MGET's aggregated reply.
        let mut results = SmallVec::<[(&Bytes, RespResponse); 10]>::new();

        for (sub_result, sub_request) in zip(sub_results, &request_info.sub_requests) {
            // Same reasoning as above: one shard's error is an error for the
            // caller, not a lost connection.
            let iter = match sub_result.and_then(RespResponse::into_collection_iter) {
                Ok(iter) => iter,
                Err(e) => return Some(Err(e)),
            };
            for (key, item) in sub_request.keys.iter().zip(iter) {
                match item {
                    Ok(item) => results.push((key, item)),
                    Err(e) => return Some(Err(e)),
                }
            }
        }

        // Precompute each key's position in the request's key list once, so
        // the reorder is O(n log n) instead of O(n² log n): the previous
        // comparator ran two linear `position` scans per comparison, making a
        // 10k-key MGET ~10⁹ `Bytes` comparisons. Duplicate keys keep their
        // first position, matching the old `position` semantics.
        let mut key_order = HashMap::<&Bytes, usize>::with_capacity(request_info.keys.len());
        for (i, k) in request_info.keys.iter().enumerate() {
            key_order.entry(k).or_insert(i);
        }
        results.sort_by_key(|(k, _)| *key_order.get(k).unwrap_or(&usize::MAX));

        let results = results.into_iter().map(|(_, v)| v).collect::<Vec<_>>();
        Some(Ok(RespResponse::owned_array(results)))
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        reason = "test code: a panic is how a test reports failure"
    )]
    use super::{
        CLUSTER_DOWN_DELAY, RequestInfo, SubRequest, TRY_AGAIN_DELAY, collect_redirections,
        transient_retry_reason,
    };
    use crate::{
        Error, ErrorKind, RedisErrorKind, Result, RetryReason,
        commands::ResponsePolicy,
        resp::{RespFrameParser, RespResponse, RespTapeMut, RespView},
    };
    use bytes::Bytes;
    use smallvec::{SmallVec, smallvec};

    /// Builds a reply out of the bytes a server would have sent, which is the
    /// only way to obtain the error frames the redirection paths read.
    fn reply(raw: &str) -> RespResponse {
        let bytes = Bytes::copy_from_slice(raw.as_bytes());
        let mut tape = RespTapeMut::default();
        let mut parser = RespFrameParser::new(&bytes, &mut tape);
        let (frame, _) = parser.parse().expect("a well-formed frame");
        RespResponse::new(bytes.into(), frame)
    }

    fn sub_request(keys: &[&str], result: Option<Result<RespResponse>>) -> SubRequest {
        SubRequest {
            node_id: "n".into(),
            keys: keys.iter().map(|k| Bytes::from(k.to_string())).collect(),
            result: Some(result),
        }
    }

    fn request(
        response_policy: Option<ResponsePolicy>,
        keys: &[&str],
        sub_requests: SmallVec<[SubRequest; 10]>,
    ) -> RequestInfo {
        RequestInfo {
            response_policy,
            keys: keys.iter().map(|k| Bytes::from(k.to_string())).collect(),
            sub_requests,
            command: None,
            is_pub_sub: false,
            #[cfg(test)]
            command_seq: 0,
        }
    }

    fn as_integer(result: Option<Result<RespResponse>>) -> i64 {
        let response = result.expect("an answer").expect("a success");
        match response.view().expect("a readable frame") {
            RespView::Integer(i, _) => i,
            other => panic!("expected an integer, got {other:?}"),
        }
    }

    /// A command that reached a single shard is answered by that shard: nothing
    /// is aggregated, and in particular nothing is wrapped in an array.
    #[test]
    fn a_command_served_by_one_shard_answers_with_its_reply() {
        let request = request(
            None,
            &["k"],
            smallvec![sub_request(&["k"], Some(Ok(reply(":7\r\n"))))],
        );
        assert_eq!(7, as_integer(request.into_reply()));
    }

    /// A redirection is not an answer: the command did not run on the node that
    /// sent it, so the caller must be given a reason to replay rather than the
    /// error frame itself.
    #[test]
    fn a_moved_reply_becomes_a_retry_reason() {
        let request = request(
            None,
            &["k"],
            smallvec![sub_request(
                &["k"],
                Some(Ok(reply("-MOVED 1234 127.0.0.1:7001\r\n"))),
            )],
        );

        let Some(Err(e)) = request.into_reply() else {
            panic!("a redirection must not answer the caller");
        };
        assert!(matches!(
            e.into_kind(),
            ErrorKind::Retry(reasons) if matches!(
                reasons.first(),
                Some(RetryReason::Moved { hash_slot: 1234, .. })
            )
        ));
    }

    /// A request one of whose shards has not answered yet is not answerable.
    /// Reporting anything here would resolve the caller on a partial reply.
    #[test]
    fn a_request_still_missing_a_shard_reply_answers_nothing() {
        let mut request = request(
            None,
            &[],
            smallvec![
                sub_request(&[], Some(Ok(reply(":1\r\n")))),
                sub_request(&[], Some(Ok(reply(":2\r\n")))),
            ],
        );
        // Not `Some(None)` — which means a closed stream — but never filled.
        if let Some(sub_request) = request.sub_requests.get_mut(1) {
            sub_request.result = None;
        }

        assert!(request.into_reply().is_none());
    }

    /// A counting command broadcast to every shard is owed the total, not one
    /// shard's share of it.
    #[test]
    fn a_sum_policy_totals_the_shard_counters() {
        let request = request(
            Some(ResponsePolicy::AggSum),
            &[],
            smallvec![
                sub_request(&[], Some(Ok(reply(":10\r\n")))),
                sub_request(&[], Some(Ok(reply(":32\r\n")))),
            ],
        );

        assert_eq!(42, as_integer(request.into_reply()));
    }

    /// Shards answering different shapes cannot be combined into one number.
    /// Folding them anyway would report an aggregate the cluster never produced.
    #[test]
    fn shards_answering_different_shapes_cannot_be_aggregated() {
        let request = request(
            Some(ResponsePolicy::AggSum),
            &[],
            smallvec![
                sub_request(&[], Some(Ok(reply(":10\r\n")))),
                sub_request(&[], Some(Ok(reply("*1\r\n:1\r\n")))),
            ],
        );

        let Some(Err(e)) = request.into_reply() else {
            panic!("incompatible shapes must not answer a value");
        };
        assert!(matches!(
            e.into_kind(),
            ErrorKind::Client(crate::ClientError::IncompatibleShardReplies)
        ));
    }

    /// A keyed command is answered in the order of the keys the caller asked
    /// for, not in the order the shards happened to reply. `MGET a b c` split
    /// over two shards must still line up value by value.
    #[test]
    fn a_keyed_command_answers_in_the_order_of_its_keys() {
        let request = request(
            None,
            &["a", "b", "c"],
            smallvec![
                sub_request(&["b"], Some(Ok(reply("*1\r\n$2\r\nvb\r\n")))),
                sub_request(
                    &["a", "c"],
                    Some(Ok(reply("*2\r\n$2\r\nva\r\n$2\r\nvc\r\n")))
                ),
            ],
        );

        let response = request.into_reply().expect("an answer").expect("a success");
        let RespView::OwnedArray(items) = response.view().expect("a readable frame") else {
            panic!("a keyed command answers an array");
        };
        let values = items
            .iter()
            .map(|item| match item.view().expect("a readable item") {
                RespView::BulkString(s) => String::from_utf8_lossy(s).into_owned(),
                other => panic!("expected a bulk string, got {other:?}"),
            })
            .collect::<Vec<_>>();

        assert_eq!(vec!["va", "vb", "vc"], values);
    }

    /// Only the redirected sub-requests may be re-sent, so each redirection is
    /// reported with the index of the one that earned it. Pairing them by
    /// position in the redirection list would re-send the wrong shard's keys.
    #[test]
    fn each_redirection_names_the_sub_request_that_earned_it() {
        let request = request(
            None,
            &["a", "b"],
            smallvec![
                sub_request(&["a"], Some(Ok(reply("$2\r\nva\r\n")))),
                sub_request(&["b"], Some(Ok(reply("-ASK 42 127.0.0.1:7002\r\n")))),
            ],
        );

        let redirections = collect_redirections(&request);
        assert_eq!(1, redirections.len());
        assert!(matches!(
            redirections.first(),
            Some((1, RetryReason::Ask { hash_slot: 42, .. }))
        ));
    }

    /// A `CLUSTERDOWN` follows a failover, which changes who owns the slot, so
    /// the replay is worthless against the topology that earned the error.
    #[test]
    fn cluster_down_is_replayed_against_a_reloaded_topology() {
        assert!(matches!(
            transient_retry_reason(&RedisErrorKind::ClusterDown),
            Some(RetryReason::TryAgain {
                delay: CLUSTER_DOWN_DELAY,
                refresh_topology: true
            })
        ));
    }

    /// A `TRYAGAIN` only reports a slot mid-migration: the topology it was read
    /// against is still the right one, so the replay must not pay a discovery.
    #[test]
    fn try_again_is_replayed_without_a_discovery() {
        assert!(matches!(
            transient_retry_reason(&RedisErrorKind::TryAgain),
            Some(RetryReason::TryAgain {
                delay: TRY_AGAIN_DELAY,
                refresh_topology: false
            })
        ));
    }

    /// Every other server error belongs to the caller: replaying it would hide a
    /// real failure behind the attempt cap and, for a command that did run,
    /// execute it twice.
    #[test]
    fn other_server_errors_are_not_retried() {
        for kind in [
            RedisErrorKind::WrongType,
            RedisErrorKind::NoPerm,
            RedisErrorKind::OutOfMemory,
            RedisErrorKind::CrossSlot,
            RedisErrorKind::Err,
        ] {
            assert!(
                transient_retry_reason(&kind).is_none(),
                "{kind:?} must reach the caller"
            );
        }
    }

    /// A transient error is absorbed rather than surfaced, even when it arrives
    /// as one shard's reply to a command the other shards answered normally.
    #[test]
    fn a_transient_error_from_one_shard_replays_the_command() {
        let request = request(
            Some(ResponsePolicy::AggSum),
            &[],
            smallvec![
                sub_request(&[], Some(Ok(reply(":1\r\n")))),
                sub_request(&[], Some(Ok(reply("-TRYAGAIN slot in migration\r\n")))),
            ],
        );

        let Some(Err(e)) = request.into_reply() else {
            panic!("a transient error must not answer the caller");
        };
        assert!(matches!(e.into_kind(), ErrorKind::Retry(_)));
    }

    /// A shard reporting a genuine failure fails the whole command: it is the
    /// caller's error, and answering `None` would tell the handler the cluster
    /// connection dropped.
    #[test]
    fn a_shard_failure_fails_the_command_rather_than_the_connection() {
        let request = request(
            Some(ResponsePolicy::AllSucceeded),
            &[],
            smallvec![
                sub_request(&[], Some(Ok(reply("+OK\r\n")))),
                sub_request(
                    &[],
                    Some(Err(Error::from(crate::ClientError::ClusterConfig)))
                ),
            ],
        );

        assert!(matches!(request.into_reply(), Some(Err(_))));
    }
}

#[cfg(test)]
mod construction_tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        reason = "test code: a panic is how a test reports failure"
    )]
    use super::{RequestInfo, SubRequest};
    use crate::resp::{Command, RespFrameParser, RespResponse, RespTapeMut, cmd};
    use bytes::Bytes;
    use smallvec::SmallVec;

    fn reply(raw: &str) -> RespResponse {
        let bytes = Bytes::copy_from_slice(raw.as_bytes());
        let mut tape = RespTapeMut::default();
        let mut parser = RespFrameParser::new(&bytes, &mut tape);
        let (frame, _) = parser.parse().expect("a well-formed frame");
        RespResponse::new(bytes.into(), frame)
    }

    fn sub_requests(count: usize) -> SmallVec<[SubRequest; 10]> {
        (0..count)
            .map(|i| SubRequest::keyless(format!("n{i}").as_str().into()))
            .collect()
    }

    /// A channel is not a key. Filing one as such would make the reassembly
    /// treat a subscription reply as a keyed one and try to line it up against
    /// a key list the caller never sent.
    #[test]
    fn a_subscription_names_channels_not_keys() {
        for name in ["SUBSCRIBE", "PSUBSCRIBE", "UNSUBSCRIBE", "PUNSUBSCRIBE"] {
            // Built the way the command families build it: a channel is passed
            // as a plain argument, never through `key`.
            let command: Command = cmd(name).arg("some-channel").into();
            let request = RequestInfo::new(&command, sub_requests(1));
            assert!(
                request.keys.is_empty(),
                "{name} must file no key, got {:?}",
                request.keys
            );
            assert!(request.is_pub_sub, "{name} is answered by a push frame");
        }
    }

    /// A command routed to one node files no key, at either level: the only
    /// reader lines one node's reply up against another's, and there is no other.
    /// The reply must still come back untouched.
    #[test]
    fn a_single_shard_command_files_no_key() {
        let command: Command = cmd("MGET").key("{tag}a").key("{tag}b").into();
        let mut sub_request = SubRequest::keyless("n".into());
        sub_request.result = Some(Some(Ok(reply("*2\r\n$1\r\nA\r\n$1\r\nB\r\n"))));
        let request = RequestInfo::single_shard(&command, sub_request);

        assert!(
            request.keys.is_empty(),
            "a single-shard command names no key list, got {:?}",
            request.keys
        );
        assert!(
            request
                .sub_requests
                .iter()
                .all(|sub_request| sub_request.keys.is_empty()),
            "nor does its sub-request"
        );

        let Some(Ok(answer)) = request.into_reply() else {
            panic!("the single sub-reply is the answer");
        };
        assert_eq!(
            vec![Some(String::from("A")), Some(String::from("B"))],
            answer.to::<Vec<Option<String>>>().unwrap()
        );
    }

    /// A keyed command files its keys, which is what the reassembly orders the
    /// answer by.
    #[test]
    fn a_keyed_command_files_the_keys_it_names() {
        let command: Command = cmd("MGET").key("a").key("b").key("c").into();
        let request = RequestInfo::new(&command, sub_requests(2));

        assert_eq!(3, request.keys.len());
        assert!(!request.is_pub_sub);
    }

    /// The command is kept only when it was split, since that is the only case
    /// where one sub-request can be re-sent without replaying what the other
    /// shards already applied.
    #[test]
    fn only_a_split_command_is_kept_for_a_partial_replay() {
        let command: Command = cmd("MGET").key("a").key("b").into();

        let one_shard = RequestInfo::new(&command, sub_requests(1)).replayable_per_shard(&command);
        assert!(one_shard.command.is_none());

        let two_shards = RequestInfo::new(&command, sub_requests(2)).replayable_per_shard(&command);
        assert!(two_shards.command.is_some());
    }
}

#[cfg(test)]
mod queue_tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        reason = "test code: a panic is how a test reports failure"
    )]
    use super::{RequestInfo, RequestQueue, SubRequest};
    use crate::resp::{Command, RespResponse, cmd};

    fn request(name: &'static str, node_ids: &[&str]) -> RequestInfo {
        let command: Command = cmd(name).into();
        RequestInfo::new(
            &command,
            node_ids
                .iter()
                .map(|id| SubRequest::keyless((*id).into()))
                .collect(),
        )
    }

    fn ok() -> Option<crate::Result<RespResponse>> {
        Some(Ok(RespResponse::ok()))
    }

    /// A reply belongs to the oldest sub-request still awaiting one from that
    /// node. Filing it against a later one would leave the older request
    /// unfulfilled forever, blocking every reply behind it.
    #[test]
    fn a_reply_fills_the_oldest_sub_request_awaiting_that_node() {
        let mut queue = RequestQueue::default();
        queue.push(request("PING", &["n1"]));
        queue.push(request("PING", &["n1"]));

        assert!(queue.file_reply(&"n1".into(), ok()));

        let first = queue.take_fulfilled_front().expect("the first is answered");
        assert!(first.sub_requests.first().unwrap().result.is_some());
        assert!(
            queue.take_fulfilled_front().is_none(),
            "the second is still waiting"
        );
    }

    /// A reply no request awaits is reported rather than dropped: it means the
    /// connection and the queue disagree about what is in flight, and every
    /// later reply would be matched to the wrong caller.
    #[test]
    fn a_reply_nothing_awaits_is_refused() {
        let mut queue = RequestQueue::default();
        assert!(!queue.file_reply(&"n1".into(), ok()));

        queue.push(request("PING", &["n1"]));
        assert!(queue.file_reply(&"n1".into(), ok()));
        // The sub-request is filled: a second reply from the same node has
        // nothing left to fill.
        assert!(!queue.file_reply(&"n1".into(), ok()));
    }

    /// A request split over two nodes is not answerable until both have
    /// replied. Reporting it early would resolve the caller on half an answer.
    #[test]
    fn a_split_request_is_taken_only_once_every_shard_replied() {
        let mut queue = RequestQueue::default();
        queue.push(request("MGET", &["n1", "n2"]));

        assert!(queue.file_reply(&"n1".into(), ok()));
        assert!(queue.take_fulfilled_front().is_none());

        assert!(queue.file_reply(&"n2".into(), ok()));
        assert!(queue.take_fulfilled_front().is_some());
    }

    /// A request waiting on a node the topology dropped can never complete, and
    /// the queue is reported in order, so it would block every later reply.
    #[test]
    fn the_front_request_reports_a_node_that_is_gone() {
        let mut queue = RequestQueue::default();
        assert!(
            !queue.front_awaits_a_missing_node(|_| false),
            "an empty queue waits on nobody"
        );

        queue.push(request("MGET", &["n1", "n2"]));
        assert!(queue.front_awaits_a_missing_node(|id| id.as_ref() == "n1"));
        assert!(!queue.front_awaits_a_missing_node(|_| true));

        // A sub-request already answered is owed nothing, so a node that left
        // after answering does not orphan the request.
        assert!(queue.file_reply(&"n2".into(), ok()));
        assert!(!queue.front_awaits_a_missing_node(|id| id.as_ref() == "n1"));
    }

    /// A subscription is acknowledged by a push frame the handler consumes on
    /// its own, so nothing ever fills its sub-request. Left in place it waits
    /// forever and deadlocks the connection.
    #[test]
    fn a_subscription_request_is_retired_by_its_acknowledgement() {
        let mut queue = RequestQueue::default();
        queue.push(request("SUBSCRIBE", &["n1"]));
        queue.push(request("PING", &["n1"]));

        queue.retire_pub_sub(&"n1".into());

        assert!(queue.file_reply(&"n1".into(), ok()));
        let front = queue.take_fulfilled_front().expect("the PING is now first");
        assert!(!front.is_pub_sub);
    }

    /// Only a subscription request is retired: an ordinary one waiting on the
    /// same node must stay, or its caller is never answered.
    #[test]
    fn retiring_a_subscription_leaves_an_ordinary_request_alone() {
        let mut queue = RequestQueue::default();
        queue.push(request("PING", &["n1"]));

        queue.retire_pub_sub(&"n1".into());

        assert!(queue.file_reply(&"n1".into(), ok()));
        assert!(queue.take_fulfilled_front().is_some());
    }

    /// A partially redirected request goes back to the head, not the tail: the
    /// caller sees replies in order, so requeuing it behind newer requests
    /// would answer them first.
    #[test]
    fn a_deferred_request_keeps_its_place_at_the_head() {
        let mut queue = RequestQueue::default();
        let deferred = request("MGET", &["n1"]);
        queue.push(request("PING", &["n2"]));
        queue.push_front(deferred);

        assert!(queue.file_reply(&"n1".into(), ok()));
        let front = queue.take_fulfilled_front().expect("the deferred one");
        assert_eq!("n1", front.sub_requests.first().unwrap().node_id.as_ref());
    }
}
