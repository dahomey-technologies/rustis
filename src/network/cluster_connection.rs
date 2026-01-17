use crate::{
    Error, RedisError, RedisErrorKind, Result, RetryReason, StandaloneConnection,
    client::{ClusterConfig, Config},
    commands::{
        ClusterCommands, ClusterHealthStatus, ClusterNodeResult, ClusterShardResult,
        LegacyClusterShardResult, RequestPolicy, ResponsePolicy,
    },
    network::Version,
    resp::{Command, CommandBuilder, RespBuf, RespDeserializer, RespSerializer},
};
use bytes::Bytes;
use futures_util::{FutureExt, future};
use log::{debug, info, trace, warn};
use rand::Rng;
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{self, value::SeqAccessDeserializer},
};
use smallvec::{SmallVec, smallvec};
use std::{
    cmp::Ordering,
    collections::VecDeque,
    fmt::{self, Debug, Formatter},
    iter::zip,
    sync::Arc,
    task::Poll,
};

#[derive(Clone, PartialEq, Eq, Debug, PartialOrd, Ord)]
#[repr(transparent)]
struct NodeId(Arc<str>);

impl From<&str> for NodeId {
    fn from(value: &str) -> Self {
        Self(Arc::from(value))
    }
}

impl AsRef<str> for NodeId {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

struct Node {
    pub id: NodeId,
    pub is_master: bool,
    pub address: (String, u16),
    pub connection: StandaloneConnection,
    pub is_dirty: bool,
}

impl Node {
    pub async fn feed(&mut self, command: &Command) -> Result<()> {
        self.connection.feed(command, &[]).await?;
        self.is_dirty = true;
        Ok(())
    }
}

impl Debug for Node {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("id", &self.id)
            .field("is_master", &self.is_master)
            .field("tag", &self.connection.tag())
            .finish()
    }
}

#[derive(Debug)]
struct SlotRange {
    pub slot_range: (u16, u16),
    /// node ids of the shard that owns the slot range,
    /// the first node id being the master node id
    pub node_ids: SmallVec<[NodeId; 6]>,
}

#[derive(Debug)]
struct SubRequest {
    pub node_id: NodeId,
    pub keys: SmallVec<[Bytes; 10]>,
    pub result: Option<Option<Result<RespBuf>>>,
}

#[derive(Debug)]
struct RequestInfo {
    pub response_policy: Option<ResponsePolicy>,
    pub keys: SmallVec<[Bytes; 10]>,
    pub sub_requests: SmallVec<[SubRequest; 10]>,
    #[allow(unused)]
    #[cfg(debug_assertions)]
    pub command_seq: usize,
}

/// Stores the state related to the current transaction (MULTI/EXEC block).
#[derive(Debug, Default)]
struct TransactionState {
    /// Holds the MULTI command temporarily until we know which shard to send it to.
    pending_multi: Option<Command>,
    /// The index of the node currently locked for the transaction.
    node_index: Option<usize>,
}

impl ClusterNodeResult {
    pub(crate) fn get_port(&self) -> Result<u16> {
        match (self.port, self.tls_port) {
            (None, Some(port)) => Ok(port),
            (Some(port), None) => Ok(port),
            _ => Err(Error::Client("Cluster misconfiguration".to_owned())),
        }
    }
}

/// Cluster connection
/// read & write_batch functions are implemented following Redis Command Tips
/// See <https://redis.io/docs/reference/command-tips/>
pub struct ClusterConnection {
    cluster_config: ClusterConfig,
    config: Config,
    nodes: Vec<Node>,
    slot_ranges: Vec<SlotRange>,
    pending_requests: VecDeque<RequestInfo>,
    tag: Arc<str>,
    /// State to manage the "Lazy MULTI" logic
    transaction_state: TransactionState,
}

impl ClusterConnection {
    pub async fn connect(
        cluster_config: &ClusterConfig,
        config: &Config,
    ) -> Result<ClusterConnection> {
        let (mut nodes, slot_ranges) = Self::connect_to_cluster(cluster_config, config).await?;
        let first_node = nodes
            .get_mut(0)
            .ok_or_else(|| Error::Client("No cluster nodes".to_owned()))?;

        let tag = first_node.connection.tag();

        Ok(ClusterConnection {
            cluster_config: cluster_config.clone(),
            config: config.clone(),
            nodes,
            slot_ranges,
            pending_requests: VecDeque::new(),
            tag,
            transaction_state: TransactionState::default(),
        })
    }

    #[inline]
    pub async fn feed(&mut self, command: &Command, retry_reasons: &[RetryReason]) -> Result<()> {
        if retry_reasons.iter().any(|r| {
            matches!(
                r,
                RetryReason::Moved {
                    hash_slot: _,
                    address: _
                }
            )
        }) {
            self.refresh_nodes_and_slot_ranges().await?;
        }

        let ask_reasons = retry_reasons
            .iter()
            .filter_map(|r| {
                if let RetryReason::Ask { hash_slot, address } = r {
                    Some((*hash_slot, address.clone()))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        if let Some(multi_cmd) = self.transaction_state.pending_multi.take() {
            let (node_idx, _) = self.get_no_request_policy_node(command, &ask_reasons)?;
            self.feed_no_request_policy(&multi_cmd, node_idx, false)
                .await?;
            self.transaction_state.node_index = Some(node_idx);
        }

        match command.name().as_ref() {
            b"MULTI" => {
                // We do not send it to the network yet. We wait for the first key-based command
                // to decide which shard owns this transaction.
                self.transaction_state.pending_multi = Some(command.clone());
            }
            b"EXEC" => {
                if let Some(node_idx) = self.transaction_state.node_index {
                    self.feed_no_request_policy(command, node_idx, false)
                        .await?;
                    self.transaction_state = TransactionState::default();
                } else {
                    return Err(Error::Client(format!(
                        "[{}] EXEC called without a preceding MULTI",
                        self.tag
                    )));
                }
            }
            _ => self.internal_feed(command, &ask_reasons).await?,
        }

        Ok(())
    }

    async fn internal_feed(
        &mut self,
        command: &Command,
        ask_reasons: &[(u16, (String, u16))],
    ) -> Result<()> {
        trace!("[{}] Analyzing command {command:?}", self.tag);
        let request_policy = command.request_policy();

        if let Some(request_policy) = request_policy {
            match request_policy {
                RequestPolicy::AllNodes => {
                    self.request_policy_all_nodes(command).await?;
                }
                RequestPolicy::AllShards => {
                    self.request_policy_all_shards(command).await?;
                }
                RequestPolicy::MultiShard => {
                    self.request_policy_multi_shard(command, ask_reasons)
                        .await?;
                }
                RequestPolicy::Special => {
                    self.request_policy_special(command)?;
                }
            }
        } else {
            self.no_request_policy(command, ask_reasons).await?;
        }

        Ok(())
    }

    #[inline]
    pub async fn flush(&mut self) -> Result<()> {
        let mut flush_futures = SmallVec::<[_; 16]>::new();

        for node in self.nodes.iter_mut() {
            if node.is_dirty {
                node.is_dirty = false;
                flush_futures.push(node.connection.flush());
            }
        }

        let results = future::join_all(flush_futures).await;

        for res in results {
            res?;
        }

        Ok(())
    }

    /// The client should execute the command on all master shards (e.g., the DBSIZE command).
    /// This tip is in-use by commands that don't accept key name arguments.
    /// The command operates atomically per shard.
    async fn request_policy_all_shards(&mut self, command: &Command) -> Result<()> {
        let mut sub_requests = SmallVec::<[SubRequest; 10]>::new();

        for node in self.nodes.iter_mut().filter(|n| n.is_master) {
            node.feed(command).await?;
            sub_requests.push(SubRequest {
                node_id: node.id.clone(),
                keys: smallvec![],
                result: None,
            });
        }

        let request_info = RequestInfo {
            response_policy: command.response_policy(),
            sub_requests,
            keys: command.keys().collect(),
            #[cfg(debug_assertions)]
            command_seq: command.command_seq,
        };

        self.pending_requests.push_back(request_info);

        Ok(())
    }

    /// The client should execute the command on all nodes - masters and replicas alike.
    /// An example is the CONFIG SET command.
    /// This tip is in-use by commands that don't accept key name arguments.
    /// The command operates atomically per shard.
    async fn request_policy_all_nodes(&mut self, command: &Command) -> Result<()> {
        if self.nodes.iter().all(|n| n.is_master) {
            self.connect_replicas().await?;
        }
        let mut sub_requests = SmallVec::<[SubRequest; 10]>::new();

        for node in self.nodes.iter_mut() {
            node.feed(command).await?;
            sub_requests.push(SubRequest {
                node_id: node.id.clone(),
                keys: smallvec![],
                result: None,
            });
        }

        let request_info = RequestInfo {
            response_policy: command.response_policy(),
            sub_requests,
            keys: command.keys().collect(),
            #[cfg(debug_assertions)]
            command_seq: command.command_seq,
        };

        self.pending_requests.push_back(request_info);

        Ok(())
    }

    /// The client should execute the command on multiple shards.
    /// The shards that execute the command are determined by the hash slots of its input key name arguments.
    /// Examples for such commands include MSET, MGET and DEL.
    /// However, note that SUNIONSTORE isn't considered as multi_shard because all of its keys must belong to the same hash slot.
    async fn request_policy_multi_shard(
        &mut self,
        command: &Command,
        ask_reasons: &[(u16, (String, u16))],
    ) -> Result<()> {
        let mut node_slot_keys_ask = command
            .args_for_cluster()
            .filter_map(|(arg, is_key, slot)| {
                is_key.then(|| {
                    let (node_index, should_ask) = self
                        .get_master_node_index_by_slot(slot, ask_reasons)
                        .ok_or_else(|| Error::Client("Cluster misconfiguration".to_owned()))?;
                    Ok((node_index, slot, arg, should_ask))
                })
            })
            .collect::<Result<Vec<_>>>()?;

        if node_slot_keys_ask.is_empty() {
            return Ok(());
        }

        node_slot_keys_ask.sort();
        trace!("[{}] node_slot_keys_ask: {node_slot_keys_ask:?}", self.tag);

        let mut current_slot_keys = SmallVec::<[Bytes; 10]>::new();
        let mut sub_requests = SmallVec::<[SubRequest; 10]>::new();
        let mut last_slot = u16::MAX;
        let mut last_node_index: usize = usize::MAX;
        let mut last_should_ask = false;

        let mut node = &mut self.nodes[0];

        for (node_index, slot, key, should_ask) in node_slot_keys_ask {
            if slot != last_slot {
                if !current_slot_keys.is_empty() {
                    if last_should_ask {
                        node.connection.asking().await?;
                    }

                    let shard_command = prepare_command_for_shard(command, &current_slot_keys);
                    node.feed(&shard_command).await?;
                    sub_requests.push(SubRequest {
                        node_id: node.id.clone(),
                        keys: std::mem::take(&mut current_slot_keys),
                        result: None,
                    });
                }

                last_slot = slot;
                last_should_ask = should_ask;
            }

            current_slot_keys.push(key);

            if node_index != last_node_index {
                node = &mut self.nodes[node_index];
                last_node_index = node_index;
            }
        }

        if last_should_ask {
            node.connection.asking().await?;
        }

        let shard_command = prepare_command_for_shard(command, &current_slot_keys);
        node.feed(&shard_command).await?;
        sub_requests.push(SubRequest {
            node_id: node.id.clone(),
            keys: std::mem::take(&mut current_slot_keys),
            result: None,
        });

        let request_info = RequestInfo {
            response_policy: command.response_policy(),
            keys: command.keys().collect(),
            sub_requests,
            #[cfg(debug_assertions)]
            command_seq: command.command_seq,
        };

        trace!("{request_info:?}");

        self.pending_requests.push_back(request_info);

        Ok(())
    }

    async fn no_request_policy(
        &mut self,
        command: &Command,
        ask_reasons: &[(u16, (String, u16))],
    ) -> Result<usize> {
        let (node_idx, should_ask) = self.get_no_request_policy_node(command, ask_reasons)?;
        self.feed_no_request_policy(command, node_idx, should_ask)
            .await?;
        Ok(node_idx)
    }

    fn get_no_request_policy_node(
        &mut self,
        command: &Command,
        ask_reasons: &[(u16, (String, u16))],
    ) -> Result<(usize, bool)> {
        let mut slots = command.slots();

        if let Some(first_slot) = slots.next() {
            if !slots.all(|s| s == first_slot) {
                return Err(Error::Client(format!(
                    "[{}] Cannot send command {} with mismatched key slots",
                    self.tag,
                    String::from_utf8_lossy(&command.name())
                )));
            }

            self.get_master_node_index_by_slot(first_slot, ask_reasons)
                .ok_or_else(|| Error::Client("Cluster misconfiguration".to_owned()))
        } else {
            Ok((self.get_random_node_index(), false))
        }
    }

    async fn feed_no_request_policy(
        &mut self,
        command: &Command,
        node_idx: usize,
        should_ask: bool,
    ) -> Result<()> {
        let node = &mut self.nodes[node_idx];
        if should_ask {
            node.connection.asking().await?;
        }
        node.feed(command).await?;
        let keys: SmallVec<[Bytes; 10]> = command.keys().collect();
        let request_info = RequestInfo {
            response_policy: command.response_policy(),
            sub_requests: smallvec![SubRequest {
                node_id: node.id.clone(),
                keys: keys.clone(),
                result: None,
            }],
            keys,
            #[cfg(debug_assertions)]
            command_seq: command.command_seq,
        };
        self.pending_requests.push_back(request_info);
        Ok(())
    }

    fn request_policy_special(&mut self, command: &Command) -> Result<()> {
        Err(Error::Client(format!(
            "[{}] Command {} not yet supported in cluster mode",
            String::from_utf8_lossy(&command.name()),
            self.tag
        )))
    }

    pub async fn read(&mut self) -> Option<Result<RespBuf>> {
        let request_info: RequestInfo;

        loop {
            if let Some(ri) = self.pending_requests.front()
                && ri.sub_requests.iter().all(|sr| sr.result.is_some())
            {
                trace!("[{}] fulfilled request_info: {ri:?}", self.tag);
                if let Some(ri) = self.pending_requests.pop_front() {
                    request_info = ri;
                    break;
                }
            }

            let read_futures = self.nodes.iter_mut().map(|n| n.connection.read().boxed());
            let (result, node_idx, _) = future::select_all(read_futures).await;

            result.as_ref()?;

            if let Some(Ok(bytes)) = &result
                && bytes.is_push_message()
            {
                return result;
            }

            let node_id = &self.nodes[node_idx].id;

            let Some((req_idx, sub_req_idx)) =
                self.pending_requests
                    .iter()
                    .enumerate()
                    .find_map(|(req_idx, req)| {
                        let sub_req_idx = req
                            .sub_requests
                            .iter()
                            .position(|sr| sr.node_id == *node_id && sr.result.is_none())?;
                        Some((req_idx, sub_req_idx))
                    })
            else {
                log::error!(
                    "[{}] Received unexpected message: {result:?} from {}",
                    self.tag,
                    self.nodes[node_idx].connection.tag()
                );
                return Some(Err(Error::Client(format!(
                    "[{}] Received unexpected message",
                    self.tag
                ))));
            };

            self.pending_requests[req_idx].sub_requests[sub_req_idx].result = Some(result);
            trace!(
                "[{}] Did store sub-request result into {:?}",
                self.tag, self.pending_requests[req_idx]
            );
        }

        self.internal_read(request_info)
    }

    pub fn try_read(&mut self) -> Poll<Option<Result<RespBuf>>> {
        let request_info: RequestInfo;

        loop {
            if let Some(ri) = self.pending_requests.front()
                && ri.sub_requests.iter().all(|sr| sr.result.is_some())
            {
                trace!("[{}] fulfilled request_info: {ri:?}", self.tag);
                if let Some(ri) = self.pending_requests.pop_front() {
                    request_info = ri;
                    break;
                }
            }

            let Some((node_idx, result)) =
                self.nodes.iter_mut().enumerate().find_map(|(node_idx, n)| {
                    match n.connection.try_read() {
                        Poll::Ready(result) => Some((node_idx, result)),
                        Poll::Pending => None,
                    }
                })
            else {
                return Poll::Pending;
            };

            if let Some(Ok(bytes)) = &result
                && bytes.is_push_message()
            {
                return Poll::Ready(result);
            }

            let node = &self.nodes[node_idx];
            let node_id = &node.id;

            let Some((req_idx, sub_req_idx)) =
                self.pending_requests
                    .iter()
                    .enumerate()
                    .find_map(|(req_idx, req)| {
                        let sub_req_idx = req
                            .sub_requests
                            .iter()
                            .position(|sr| sr.node_id == *node_id && sr.result.is_none())?;
                        Some((req_idx, sub_req_idx))
                    })
            else {
                log::error!(
                    "[{}] Received unexpected message: {result:?}",
                    node.connection.tag()
                );
                return Poll::Ready(Some(Err(Error::Client(format!(
                    "[{}] Received unexpected message",
                    node.connection.tag()
                )))));
            };

            self.pending_requests[req_idx].sub_requests[sub_req_idx].result = Some(result);
            trace!(
                "[{}] Did store sub-request result into {:?}",
                self.tag, self.pending_requests[req_idx]
            );
        }

        Poll::Ready(self.internal_read(request_info))
    }

    fn internal_read(
        &mut self,
        mut request_info: RequestInfo,
    ) -> Option<std::result::Result<RespBuf, Error>> {
        let mut sub_results =
            Vec::<Result<RespBuf>>::with_capacity(request_info.sub_requests.len());
        let mut retry_reasons = SmallVec::<[RetryReason; 1]>::new();

        for sub_request in request_info.sub_requests.iter_mut() {
            let result = sub_request.result.take()?;

            if let Some(result) = result {
                match &result {
                    Ok(resp_buf) if resp_buf.is_error() => match resp_buf.to::<()>() {
                        Err(Error::Redis(RedisError {
                            kind: RedisErrorKind::Ask { hash_slot, address },
                            description: _,
                        })) => retry_reasons.push(RetryReason::Ask {
                            hash_slot,
                            address: address.clone(),
                        }),
                        Err(Error::Redis(RedisError {
                            kind: RedisErrorKind::Moved { hash_slot, address },
                            description: _,
                        })) => retry_reasons.push(RetryReason::Moved {
                            hash_slot,
                            address: address.clone(),
                        }),
                        _ => sub_results.push(result),
                    },
                    _ => sub_results.push(result),
                }
            } else {
                return None;
            }
        }

        if !retry_reasons.is_empty() {
            debug!(
                "[{}] read failed and will be retried. reasons: {:?}",
                self.tag, retry_reasons
            );
            return Some(Err(Error::Retry(retry_reasons)));
        }

        // The response_policy tip is set for commands that reply with scalar data types,
        // or when it's expected that clients implement a non-default aggregate.
        if let Some(response_policy) = &request_info.response_policy {
            match response_policy {
                ResponsePolicy::OneSucceeded => self.response_policy_one_succeeded(sub_results),
                ResponsePolicy::AllSucceeded => self.response_policy_all_succeeded(sub_results),
                ResponsePolicy::AggLogicalAnd => {
                    self.response_policy_agg(sub_results, |a, b| i64::from(a == 1 && b == 1))
                }
                ResponsePolicy::AggLogicalOr => self
                    .response_policy_agg(sub_results, |a, b| if a == 0 && b == 0 { 0 } else { 1 }),
                ResponsePolicy::AggMin => self.response_policy_agg(sub_results, i64::min),
                ResponsePolicy::AggMax => self.response_policy_agg(sub_results, i64::max),
                ResponsePolicy::AggSum => self.response_policy_agg(sub_results, |a, b| a + b),
                ResponsePolicy::Special => self.response_policy_special(sub_results),
            }
        } else {
            self.no_response_policy(sub_results, &request_info)
        }
    }

    fn response_policy_one_succeeded(
        &mut self,
        sub_results: Vec<Result<RespBuf>>,
    ) -> Option<Result<RespBuf>> {
        let mut result: Result<RespBuf> = Ok(RespBuf::nil());

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
        &mut self,
        sub_results: Vec<Result<RespBuf>>,
    ) -> Option<Result<RespBuf>> {
        let mut result: Result<RespBuf> = Ok(RespBuf::nil());

        for sub_result in sub_results {
            match &sub_result {
                Err(_) => return Some(sub_result),
                Ok(resp_buf) if resp_buf.is_error() => return Some(sub_result),
                _ => result = sub_result,
            }
        }

        Some(result)
    }

    fn response_policy_agg<F>(
        &mut self,
        sub_results: Vec<Result<RespBuf>>,
        f: F,
    ) -> Option<Result<RespBuf>>
    where
        F: Fn(i64, i64) -> i64,
    {
        let mut visitor = AggVisitor {
            integer: Integer::Nil,
            f,
        };

        for sub_result in sub_results {
            let Ok(sub_result) = sub_result else {
                return Some(sub_result);
            };

            let mut deserializer = RespDeserializer::new(&sub_result);
            if let Err(e) = deserializer.deserialize_any(&mut visitor) {
                return Some(Err(e));
            }
        }

        match visitor.integer {
            Integer::Single(i) => {
                let mut serializer = RespSerializer::new();
                if let Err(e) = i.serialize(&mut serializer) {
                    return Some(Err(e));
                }
                Some(Ok(RespBuf::new(serializer.get_output().freeze())))
            }
            Integer::Array(vec) => {
                let mut serializer = RespSerializer::new();
                if let Err(e) = vec.serialize(&mut serializer) {
                    return Some(Err(e));
                }
                Some(Ok(RespBuf::new(serializer.get_output().freeze())))
            }
            Integer::Nil => Some(Ok(RespBuf::nil())),
        }
    }

    fn response_policy_special(
        &mut self,
        _sub_results: Vec<Result<RespBuf>>,
    ) -> Option<Result<RespBuf>> {
        Some(Err(Error::Client(format!(
            "[{}] Command not yet supported in cluster mode",
            self.tag
        ))))
    }

    fn no_response_policy(
        &mut self,
        sub_results: Vec<Result<RespBuf>>,
        request_info: &RequestInfo,
    ) -> Option<Result<RespBuf>> {
        log::trace!("[{}] no_response_policy", self.tag);
        if sub_results.len() == 1 {
            // when there is a single sub request, we just read the response
            // on the right connection. For example, GET's reply
            Some(sub_results.into_iter().next()?)
        } else if request_info.keys.is_empty() {
            // The command doesn't accept key name arguments:
            // the client can aggregate all replies within a single nested data structure.
            // For example, the array replies we get from calling KEYS against all shards.
            // These should be packed in a single array in no particular order.
            let mut results = Vec::<&[u8]>::new();
            for sub_result in &sub_results {
                match sub_result {
                    Ok(resp_buf) if !resp_buf.is_error() => {
                        let mut deserializer = RespDeserializer::new(resp_buf);
                        let Ok(chunks) = deserializer.array_chunks() else {
                            return Some(Err(Error::Client(format!(
                                "[{}] Unexpected result {sub_result:?}",
                                self.tag
                            ))));
                        };

                        for chunk in chunks {
                            results.push(chunk);
                        }
                    }
                    _ => {
                        return Some(sub_result.clone());
                    }
                }
            }

            Some(Ok(RespBuf::from_chunks(&results)))
        } else {
            // For commands that accept one or more key name arguments:
            // the client needs to retain the same order of replies as the input key names.
            // For example, MGET's aggregated reply.
            let mut results = SmallVec::<[(&Bytes, &[u8]); 10]>::new();

            for (sub_result, sub_request) in zip(&sub_results, &request_info.sub_requests) {
                match sub_result {
                    Ok(resp_buf) if !resp_buf.is_error() => {
                        let mut deserializer = RespDeserializer::new(resp_buf);
                        let Ok(chunks) = deserializer.array_chunks() else {
                            return Some(Err(Error::Client(format!(
                                "[{}] Unexpected result {sub_result:?}",
                                self.tag
                            ))));
                        };

                        if sub_request.keys.len() == chunks.len() {
                            results.extend(zip(&sub_request.keys, chunks));
                        } else {
                            return Some(Err(Error::Client(format!(
                                "[{}] Unexpected result {sub_result:?}",
                                self.tag
                            ))));
                        }
                    }
                    _ => {
                        return Some(sub_result.clone());
                    }
                }
            }

            results.sort_by(|(k1, _), (k2, _)| {
                request_info
                    .keys
                    .iter()
                    .position(|k| k == *k1)
                    .cmp(&request_info.keys.iter().position(|k| k == *k2))
            });

            let results = results.into_iter().map(|(_, v)| v).collect::<Vec<_>>();
            Some(Ok(RespBuf::from_chunks(&results)))
        }
    }

    pub async fn reconnect(&mut self) -> Result<()> {
        info!("[{}] Reconnecting to cluster...", self.tag);
        let (nodes, slot_ranges) =
            Self::connect_to_cluster(&self.cluster_config, &self.config).await?;
        info!("[{}] Reconnected to cluster!", self.tag);

        self.nodes = nodes;
        self.slot_ranges = slot_ranges;

        Ok(())

        // TODO improve reconnection strategy with multiple retries
    }

    async fn connect_to_cluster(
        cluster_config: &ClusterConfig,
        config: &Config,
    ) -> Result<(Vec<Node>, Vec<SlotRange>)> {
        debug!("Discovering cluster shard and slots...");

        let mut shard_info_list: Option<Vec<ClusterShardResult>> = None;

        for node_config in &cluster_config.nodes {
            match StandaloneConnection::connect(&node_config.0, node_config.1, config).await {
                Ok(mut connection) => {
                    let version: Result<Version> = connection.get_version().try_into();
                    let Ok(version) = version else {
                        warn!("[{}] Cannot execute get Redis version", connection.tag());
                        break;
                    };

                    // From Redis 7.x CLUSTER SLOTS is deprecated in favor of CLUSTER SHARDS
                    if version.major < 7 {
                        match connection.cluster_slots().await {
                            Ok(si) => {
                                shard_info_list =
                                    Some(Self::convert_from_legacy_shard_description(si));
                                break;
                            }
                            Err(e) => warn!(
                                "[{}] Cannot execute `cluster_slots` on node ({}:{}): {e}",
                                connection.tag(),
                                node_config.0,
                                node_config.1
                            ),
                        }
                    } else {
                        match connection.cluster_shards().await {
                            Ok(si) => {
                                shard_info_list = Some(si);
                                break;
                            }
                            Err(e) => warn!(
                                "[{}] Cannot execute `cluster_shards` on node ({}:{}): {e}",
                                connection.tag(),
                                node_config.0,
                                node_config.1
                            ),
                        }
                    }
                }
                Err(e) => warn!(
                    "Cannot connect to node ({}:{}): {}",
                    node_config.0, node_config.1, e
                ),
            }
        }

        let Some(shard_info_list) = shard_info_list else {
            return Err(Error::Client("Cluster misconfiguration".to_owned()));
        };

        let mut nodes = Vec::<Node>::new();
        let mut slot_ranges = Vec::<SlotRange>::new();

        for shard_info in shard_info_list.into_iter() {
            let Some(master_info) = shard_info
                .nodes
                .into_iter()
                .find(|n| n.role == "master" && n.health == ClusterHealthStatus::Online)
            else {
                return Err(Error::Client("Cluster misconfiguration".to_owned()));
            };
            let master_id: NodeId = master_info.id.as_str().into();

            let port = master_info.get_port()?;

            let connection = StandaloneConnection::connect(&master_info.ip, port, config).await?;

            slot_ranges.extend(shard_info.slots.iter().map(|s| SlotRange {
                slot_range: *s,
                node_ids: smallvec![master_id.clone()],
            }));

            nodes.push(Node {
                id: master_id.clone(),
                is_master: true,
                address: (master_info.ip, port),
                connection,
                is_dirty: false,
            });
        }

        slot_ranges.sort_by_key(|s| s.slot_range.0);
        nodes.sort_by(|n1, n2| n1.id.cmp(&n2.id));

        debug!("Cluster connected: nodes={nodes:?}, slot_ranges={slot_ranges:?}");

        Ok((nodes, slot_ranges))
    }

    async fn connect_replicas(&mut self) -> Result<()> {
        debug!("[{}] Connecting replicas...", self.tag);

        let connection = &mut self.get_random_node_mut().connection;
        let version: Version = connection.get_version().try_into()?;

        // From Redis 7.x CLUSTER SLOTS is deprecated in favor of CLUSTER SHARDS
        let shard_info_list: Vec<ClusterShardResult> = if version.major < 7 {
            Self::convert_from_legacy_shard_description(connection.cluster_slots().await?)
        } else {
            connection.cluster_shards().await?
        };

        for shard_info in shard_info_list {
            for node_info in shard_info.nodes.into_iter().filter(|n| n.role == "replica") {
                let port = node_info.get_port()?;
                let node_id: NodeId = node_info.id.as_str().into();

                let connection =
                    StandaloneConnection::connect(&node_info.ip, port, &self.config).await?;

                for slot_range_info in &shard_info.slots {
                    if let Some(slot_range) = self.get_slot_range_by_slot_mut(slot_range_info.0)
                        && slot_range.slot_range.1 == slot_range_info.1
                    {
                        slot_range.node_ids.push(node_id.clone())
                    }
                }

                self.nodes.push(Node {
                    id: node_id,
                    is_master: false,
                    address: (node_info.ip.clone(), port),
                    connection,
                    is_dirty: false,
                });
            }
        }

        self.nodes.sort_by(|n1, n2| n1.id.cmp(&n2.id));

        debug!(
            "[{}] Cluster replicas connected: nodes={:?}, slot_ranges={:?}",
            self.tag, self.nodes, self.slot_ranges
        );

        Ok(())
    }

    /// Keep existing connection, connect new nodes, remove obsolte ones
    /// Rebuild slot_ranges from scratch
    async fn refresh_nodes_and_slot_ranges(&mut self) -> Result<()> {
        debug!("[{}] Reloading slot ranges", self.tag);

        let connection = &mut self.get_random_node_mut().connection;
        let version: Version = connection.get_version().try_into()?;

        // From Redis 7.x CLUSTER SLOTS is deprecated in favor of CLUSTER SHARDS
        let shard_info_list: Vec<ClusterShardResult> = if version.major < 7 {
            Self::convert_from_legacy_shard_description(connection.cluster_slots().await?)
        } else {
            connection.cluster_shards().await?
        };

        // filter out nodes that do not exist anymore
        let mut node_ids = shard_info_list
            .iter()
            .flat_map(|s| s.nodes.iter().map(|n| n.id.as_str()))
            .collect::<Vec<_>>();
        node_ids.sort();
        self.nodes.retain(|node| {
            node_ids
                .binary_search_by(|n| (*n).cmp(node.id.as_ref()))
                .is_ok()
        });

        // create slot_ranges from scratch
        self.slot_ranges.clear();

        // add missing nodes and connect them
        for mut shard_info in shard_info_list {
            // ensure that the first node is master
            if shard_info.nodes[0].role != "master" {
                let Some(master_idx) = shard_info.nodes.iter().position(|n| n.role == "master")
                else {
                    return Err(Error::Client("Cluster misconfiguration".to_owned()));
                };

                // swap first node & master node
                shard_info.nodes.swap(0, master_idx);
            }

            // add slot_ranges
            for slot_range_info in &shard_info.slots {
                self.slot_ranges.push(SlotRange {
                    slot_range: *slot_range_info,
                    node_ids: shard_info
                        .nodes
                        .iter()
                        .map(|n| n.id.as_str().into())
                        .collect(),
                });
            }

            for node_info in shard_info.nodes {
                let node_id: NodeId = node_info.id.as_str().into();
                if let Some(node) = self.nodes.iter_mut().find(|n| n.id == node_id) {
                    // refresh is_master flag in case a failover happened
                    node.is_master = node_info.role == "master";
                } else {
                    // add missing node
                    let port = node_info.get_port()?;

                    let connection =
                        StandaloneConnection::connect(&node_info.ip, port, &self.config).await?;

                    self.nodes.push(Node {
                        id: node_id,
                        is_master: node_info.role == "master",
                        address: (node_info.ip, port),
                        connection,
                        is_dirty: false,
                    });
                }
            }
        }

        self.slot_ranges.sort_by_key(|s| s.slot_range.0);
        self.nodes.sort_by(|n1, n2| n1.id.cmp(&n2.id));

        debug!(
            "[{}] Cluster new setup: nodes={:?}, slot_ranges={:?}",
            self.tag, self.nodes, self.slot_ranges
        );

        Ok(())
    }

    #[inline]
    fn get_node_index_by_id(&self, id: &NodeId) -> Option<usize> {
        self.nodes.binary_search_by_key(&id, |n| &n.id).ok()
    }

    #[inline]
    fn get_random_node_index(&self) -> usize {
        rand::rng().random_range(0..self.nodes.len())
    }

    #[inline]
    fn get_random_node_mut(&mut self) -> &mut Node {
        let node_idx = self.get_random_node_index();
        &mut self.nodes[node_idx]
    }

    #[inline]
    fn get_slot_range_index(&self, slot: u16) -> Option<usize> {
        self.slot_ranges
            .binary_search_by(|s| {
                if s.slot_range.0 > slot {
                    Ordering::Greater
                } else if s.slot_range.1 < slot {
                    Ordering::Less
                } else {
                    Ordering::Equal
                }
            })
            .ok()
    }

    #[inline]
    fn get_slot_range_by_slot(&self, slot: u16) -> Option<&SlotRange> {
        self.get_slot_range_index(slot)
            .map(|idx| &self.slot_ranges[idx])
    }

    #[inline]
    fn get_slot_range_by_slot_mut(&mut self, slot: u16) -> Option<&mut SlotRange> {
        self.get_slot_range_index(slot)
            .map(|idx| &mut self.slot_ranges[idx])
    }

    fn get_master_node_index_by_slot(
        &mut self,
        slot: u16,
        ask_reasons: &[(u16, (String, u16))],
    ) -> Option<(usize, bool)> {
        let ask_reason = ask_reasons
            .iter()
            .find(|(hash_slot, (_ip, _port))| *hash_slot == slot);

        if let Some((_hash_slot, address)) = ask_reason {
            let node_index = self.nodes.iter().position(|n| n.address == *address)?;
            Some((node_index, true))
        } else {
            let slot_range = self.get_slot_range_by_slot(slot)?;
            let master_node_id = &slot_range.node_ids[0];
            let node_index = self.get_node_index_by_id(master_node_id)?;
            Some((node_index, false))
        }
    }

    pub(crate) fn convert_from_legacy_shard_description(
        mut legacy_shards: Vec<LegacyClusterShardResult>,
    ) -> Vec<ClusterShardResult> {
        legacy_shards.sort_by(|s1, s2| s1.nodes[0].id.cmp(&s2.nodes[0].id));

        let mut last_master_id = String::new();
        let mut shards = Vec::new();
        for legacy_shard in legacy_shards {
            let master_id = &legacy_shard.nodes[0].id;
            if master_id != &last_master_id {
                last_master_id.clone_from(master_id);
                shards.push(ClusterShardResult {
                    slots: vec![legacy_shard.slot],
                    nodes: legacy_shard
                        .nodes
                        .into_iter()
                        .enumerate()
                        .map(|(idx, node)| ClusterNodeResult {
                            id: node.id,
                            endpoint: node.preferred_endpoint.clone(),
                            ip: node.ip,
                            port: Some(node.port),
                            hostname: node.hostname,
                            tls_port: None,
                            role: if idx == 0 {
                                "master".to_owned()
                            } else {
                                "replica".to_owned()
                            },
                            replication_offset: 0,
                            health: ClusterHealthStatus::Online,
                        })
                        .collect(),
                });
            } else if let Some(shard) = shards.last_mut() {
                shard.slots.push(legacy_shard.slot);
            }
        }

        shards
    }

    pub(crate) fn tag(&self) -> Arc<str> {
        self.tag.clone()
    }
}

pub fn prepare_command_for_shard(command: &Command, shard_keys: &[Bytes]) -> Command {
    // Initialize a new command with the same base name
    let mut shard_command = CommandBuilder::new(&command.name());

    // Tracks how many subsequent arguments to keep after a valid key
    let mut keep_next = 0;

    // The step defines how many arguments form a logical group (e.g., 2 for MSET)
    let step = command.key_step();

    // Iterate through all arguments using the cluster helper
    for (arg, is_key, _) in command.args_for_cluster() {
        if is_key {
            // If the current argument is a key, check if it exists in our shard group
            if shard_keys.contains(&arg) {
                shard_command = shard_command.arg(arg);
                // Keep the next (step - 1) arguments associated with this key
                keep_next = step - 1;
            } else {
                // Key belongs to another shard
                keep_next = 0;
            }
        } else if keep_next > 0 {
            // This is a value/path associated with an accepted key
            shard_command = shard_command.arg(arg);
            keep_next -= 1;
        }
    }

    shard_command.into()
}

enum Integer {
    Single(i64),
    Array(Vec<i64>),
    Nil,
}

struct AggVisitor<F: Fn(i64, i64) -> i64> {
    integer: Integer,
    f: F,
}

impl<'de, F: Fn(i64, i64) -> i64> de::Visitor<'de> for &mut AggVisitor<F> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("()")
    }

    fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
    where
        E: de::Error,
    {
        match &self.integer {
            Integer::Nil => self.integer = Integer::Single(v),
            Integer::Single(i) => self.integer = Integer::Single((self.f)(v, *i)),
            _ => {
                return Err(de::Error::custom("Unexpected value".to_owned()));
            }
        }

        Ok(())
    }

    fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        match &mut self.integer {
            Integer::Nil => {
                self.integer =
                    Integer::Array(Vec::<i64>::deserialize(SeqAccessDeserializer::new(seq))?)
            }
            Integer::Array(a) => {
                for i in a {
                    let Some(next_i) = seq.next_element()? else {
                        return Err(de::Error::custom("Unexpected value".to_owned()));
                    };

                    *i = (self.f)(*i, next_i);
                }
            }
            _ => {
                return Err(de::Error::custom("Unexpected value".to_owned()));
            }
        }

        Ok(())
    }
}
