use crate::{
    resp::{Array, BulkString, Command, Value},
    ClusterCommands, ClusterConfig, ClusterNodeResult, ClusterShardResult, CommandInfoManager,
    CommandTip, Config, Error, RedisError, RedisErrorKind, RequestPolicy, ResponsePolicy, Result,
    RetryReason, StandaloneConnection,
};
use futures::{future, FutureExt};
use log::{debug, info, trace, warn};
use rand::Rng;
use smallvec::{smallvec, SmallVec};
use std::{
    cmp::Ordering,
    collections::VecDeque,
    fmt::{Debug, Formatter},
    iter::zip,
};

struct Node {
    pub id: String,
    pub is_master: bool,
    pub address: (String, u16),
    pub connection: StandaloneConnection,
}

impl Debug for Node {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("id", &self.id)
            .field("is_master", &self.is_master)
            .field("address", &self.address)
            .finish()
    }
}

#[derive(Debug)]
struct SlotRange {
    pub slot_range: (u16, u16),
    /// node ids of the shard that owns the slot range,
    /// the first node id being the master node id
    pub node_ids: SmallVec<[String; 6]>,
}

#[derive(Debug)]
struct SubRequest {
    pub node_id: String,
    pub keys: SmallVec<[String; 10]>,
    pub result: Option<Option<Result<Value>>>,
}

#[derive(Debug)]
struct RequestInfo {
    pub command_name: String,
    pub keys: SmallVec<[String; 10]>,
    pub sub_requests: SmallVec<[SubRequest; 10]>,
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
    command_info_manager: CommandInfoManager,
    pending_requests: VecDeque<RequestInfo>,
}

impl ClusterConnection {
    pub async fn connect(
        cluster_config: &ClusterConfig,
        config: &Config,
    ) -> Result<ClusterConnection> {
        let (mut nodes, slot_ranges) = Self::connect_to_cluster(cluster_config, config).await?;

        let command_info_manager = CommandInfoManager::initialize(&mut nodes[0].connection).await?;

        Ok(ClusterConnection {
            cluster_config: cluster_config.clone(),
            config: config.clone(),
            nodes,
            slot_ranges,
            command_info_manager,
            pending_requests: VecDeque::new(),
        })
    }

    pub async fn write_batch(
        &mut self,
        commands: impl Iterator<Item = &Command>,
        retry_reasons: &[RetryReason],
    ) -> Result<()> {
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

        for command in commands {
            debug!("Analyzing command {command:?}");

            let command_info = self.command_info_manager.get_command_info(command);

            let command_info = if let Some(command_info) = command_info {
                command_info
            } else {
                return Err(Error::Client(format!("Unknown command {}", command.name)));
            };

            let command_name = command_info.name.to_string();

            let request_policy = command_info.command_tips.iter().find_map(|tip| {
                if let CommandTip::RequestPolicy(request_policy) = tip {
                    Some(request_policy)
                } else {
                    None
                }
            });

            let node_idx = self.get_random_node_index();
            let keys = self
                .command_info_manager
                .extract_keys(command, &mut self.nodes[node_idx].connection)
                .await?;
            let slots = Self::hash_slots(&keys);

            debug!("keys: {keys:?}, slots: {slots:?}");

            if let Some(request_policy) = request_policy {
                match request_policy {
                    RequestPolicy::AllNodes => {
                        self.request_policy_all_nodes(command, &command_name, keys)
                            .await?;
                    }
                    RequestPolicy::AllShards => {
                        self.request_policy_all_shards(command, &command_name, keys)
                            .await?;
                    }
                    RequestPolicy::MultiShard => {
                        self.request_policy_multi_shard(
                            command,
                            &command_name,
                            keys,
                            slots,
                            &ask_reasons,
                        )
                        .await?;
                    }
                    RequestPolicy::Special => {
                        self.request_policy_special(command, command_name, keys, slots);
                    }
                }
            } else {
                self.no_request_policy(command, command_name, keys, slots, &ask_reasons)
                    .await?;
            }
        }

        Ok(())
    }

    /// The client should execute the command on all master shards (e.g., the DBSIZE command).
    /// This tip is in-use by commands that don't accept key name arguments.
    /// The command operates atomically per shard.
    async fn request_policy_all_shards(
        &mut self,
        command: &Command,
        command_name: &str,
        keys: SmallVec<[String; 10]>,
    ) -> Result<()> {
        let mut sub_requests = SmallVec::<[SubRequest; 10]>::new();

        for node in self.nodes.iter_mut().filter(|n| n.is_master) {
            node.connection.write(command).await?;
            sub_requests.push(SubRequest {
                node_id: node.id.clone(),
                keys: smallvec![],
                result: None,
            });
        }

        let request_info = RequestInfo {
            command_name: command_name.to_string(),
            sub_requests,
            keys,
        };

        self.pending_requests.push_back(request_info);

        Ok(())
    }

    /// The client should execute the command on all nodes - masters and replicas alike.
    /// An example is the CONFIG SET command.
    /// This tip is in-use by commands that don't accept key name arguments.
    /// The command operates atomically per shard.
    async fn request_policy_all_nodes(
        &mut self,
        command: &Command,
        command_name: &str,
        keys: SmallVec<[String; 10]>,
    ) -> Result<()> {
        if self.nodes.iter().all(|n| n.is_master) {
            self.connect_replicas().await?;
        }
        let mut sub_requests = SmallVec::<[SubRequest; 10]>::new();

        for node in self.nodes.iter_mut() {
            node.connection.write(command).await?;
            sub_requests.push(SubRequest {
                node_id: node.id.clone(),
                keys: smallvec![],
                result: None,
            });
        }

        let request_info = RequestInfo {
            command_name: command_name.to_string(),
            sub_requests,
            keys,
        };

        self.pending_requests.push_back(request_info);

        Ok(())
    }

    /// The client should execute the command on several shards.
    /// The shards that execute the command are determined by the hash slots of its input key name arguments.
    /// Examples for such commands include MSET, MGET and DEL.
    /// However, note that SUNIONSTORE isn't considered as multi_shard because all of its keys must belong to the same hash slot.
    async fn request_policy_multi_shard(
        &mut self,
        command: &Command,
        command_name: &str,
        keys: SmallVec<[String; 10]>,
        slots: SmallVec<[u16; 10]>,
        ask_reasons: &[(u16, (String, u16))],
    ) -> Result<()> {
        let mut node_slot_keys_ask = (0..keys.len())
            .map(|i| {
                let (node_index, should_ask) = self
                    .get_master_node_index_by_slot(slots[i], ask_reasons)
                    .ok_or_else(|| Error::Client("Cluster misconfiguration".to_owned()))?;
                Ok((node_index, slots[i], keys[i].clone(), should_ask))
            })
            .collect::<Result<Vec<_>>>()?;

        node_slot_keys_ask.sort();
        trace!("shard_slot_keys_ask: {node_slot_keys_ask:?}");

        let mut last_slot = u16::MAX;
        let mut current_slot_keys = SmallVec::<[String; 10]>::new();
        let mut sub_requests = SmallVec::<[SubRequest; 10]>::new();
        let mut last_node_index: usize = 0;
        let mut last_should_ask = false;

        let mut node = &mut self.nodes[last_node_index];

        for (node_index, slot, key, should_ask) in &node_slot_keys_ask {
            if *slot != last_slot {
                if !current_slot_keys.is_empty() {
                    if last_should_ask {
                        node.connection.asking().await?;
                    }

                    let shard_command = self
                        .command_info_manager
                        .prepare_command_for_shard(command, current_slot_keys.iter())?;
                    node.connection.write(&shard_command).await?;
                    sub_requests.push(SubRequest {
                        node_id: node.id.clone(),
                        keys: current_slot_keys.clone(),
                        result: None,
                    });

                    current_slot_keys.clear();
                }

                last_slot = *slot;
                last_should_ask = *should_ask;
            }

            current_slot_keys.push(key.clone());

            if *node_index != last_node_index {
                node = &mut self.nodes[*node_index];
                last_node_index = *node_index;
            }
        }

        if last_should_ask {
            node.connection.asking().await?;
        }

        let shard_command = self
            .command_info_manager
            .prepare_command_for_shard(command, current_slot_keys.iter())?;

        node.connection.write(&shard_command).await?;

        sub_requests.push(SubRequest {
            node_id: node.id.clone(),
            keys: current_slot_keys.clone(),
            result: None,
        });

        let request_info = RequestInfo {
            command_name: command_name.to_string(),
            keys,
            sub_requests,
        };

        trace!("{request_info:?}");

        self.pending_requests.push_back(request_info);

        Ok(())
    }

    async fn no_request_policy(
        &mut self,
        command: &Command,
        command_name: String,
        keys: SmallVec<[String; 10]>,
        slots: SmallVec<[u16; 10]>,
        ask_reasons: &[(u16, (String, u16))],
    ) -> Result<()> {
        // test if all slots are equal
        if slots.windows(2).all(|s| s[0] == s[1]) {
            let (node_idx, should_ask) = if slots.is_empty() {
                (self.get_random_node_index(), false)
            } else {
                self.get_master_node_index_by_slot(slots[0], ask_reasons)
                    .ok_or_else(|| Error::Client("Cluster misconfiguration".to_owned()))?
            };

            let node = &mut self.nodes[node_idx];
            let connection = &mut node.connection;

            if should_ask {
                connection.asking().await?;
            }
            connection.write(command).await?;

            let request_info = RequestInfo {
                command_name: command_name.to_string(),
                sub_requests: smallvec![SubRequest {
                    node_id: node.id.clone(),
                    keys: keys.clone(),
                    result: None,
                }],
                keys,
            };

            self.pending_requests.push_back(request_info);
        } else {
            return Err(Error::Client(format!(
                "Cannot send command {} with mistmatched key slots",
                command_name
            )));
        }

        Ok(())
    }

    fn request_policy_special(
        &mut self,
        _command: &Command,
        _command_name: String,
        _keys: SmallVec<[String; 10]>,
        _slots: SmallVec<[u16; 10]>,
    ) {
        todo!("Command not yet supported in cluster mode")
    }

    pub async fn read(&mut self) -> Option<Result<Value>> {
        let mut request_info: RequestInfo;

        loop {
            let read_futures = self.nodes.iter_mut().map(|n| n.connection.read().boxed());
            let (result, node_idx, _) = future::select_all(read_futures).await;

            if let Some(value) = &result {
                if is_push_message(value) {
                    return result;
                }
            }

            let node_id = &self.nodes[node_idx].id;

            if let Some(sub_request) = self.pending_requests.iter_mut().find_map(|r| {
                r.sub_requests
                    .iter_mut()
                    .find(|sr| sr.node_id == *node_id && sr.result.is_none())
            }) {
                sub_request.result = Some(result);
            } else {
                return Some(Err(Error::Client("Received unexpected message".to_owned())));
            };

            if let Some(ri) = self.pending_requests.front() {
                trace!("request_info: {ri:?}");
                if ri.sub_requests.iter().all(|sr| sr.result.is_some()) {
                    if let Some(ri) = self.pending_requests.pop_front() {
                        request_info = ri;
                        break;
                    }
                }
            }
        }

        let mut sub_results = Vec::<Result<Value>>::with_capacity(request_info.sub_requests.len());
        let mut retry_reasons = SmallVec::<[RetryReason; 1]>::new();

        for sub_request in request_info.sub_requests.iter_mut() {
            let result = sub_request.result.take().unwrap();

            if let Some(result) = result {
                match &result {
                    Ok(Value::Error(RedisError {
                        kind: RedisErrorKind::Ask { hash_slot, address },
                        description: _,
                    })) => retry_reasons.push(RetryReason::Ask {
                        hash_slot: *hash_slot,
                        address: address.clone(),
                    }),
                    Ok(Value::Error(RedisError {
                        kind: RedisErrorKind::Moved { hash_slot, address },
                        description: _,
                    })) => retry_reasons.push(RetryReason::Moved {
                        hash_slot: *hash_slot,
                        address: address.clone(),
                    }),
                    _ => sub_results.push(result),
                }
            } else {
                return None;
            }
        }

        if !retry_reasons.is_empty() {
            debug!(
                "read failed and will be retried. reasons: {:?}",
                retry_reasons
            );
            return Some(Err(Error::Retry(retry_reasons)));
        }

        let command_name = &request_info.command_name;
        let command_info = self
            .command_info_manager
            .get_command_info_by_name(command_name);

        let command_info = if let Some(command_info) = command_info {
            command_info
        } else {
            return Some(Err(Error::Client(format!(
                "Unknown command {}",
                command_name
            ))));
        };

        let response_policy = command_info.command_tips.iter().find_map(|tip| {
            if let CommandTip::ResponsePolicy(response_policy) = tip {
                Some(response_policy)
            } else {
                None
            }
        });

        // The response_policy tip is set for commands that reply with scalar data types,
        // or when it's expected that clients implement a non-default aggregate.
        if let Some(response_policy) = response_policy {
            match response_policy {
                ResponsePolicy::OneSucceeded => {
                    self.response_policy_one_succeeded(sub_results).await
                }
                ResponsePolicy::AllSucceeded => {
                    self.response_policy_all_succeeded(sub_results).await
                }
                ResponsePolicy::AggLogicalAnd => {
                    self.response_policy_agg(sub_results, |a, b| i64::from(a == 1 && b == 1))
                        .await
                }
                ResponsePolicy::AggLogicalOr => {
                    self.response_policy_agg(
                        sub_results,
                        |a, b| if a == 0 && b == 0 { 0 } else { 1 },
                    )
                    .await
                }
                ResponsePolicy::AggMin => self.response_policy_agg(sub_results, i64::min).await,
                ResponsePolicy::AggMax => self.response_policy_agg(sub_results, i64::max).await,
                ResponsePolicy::AggSum => self.response_policy_agg(sub_results, |a, b| a + b).await,
                ResponsePolicy::Special => self.response_policy_special(sub_results).await,
            }
        } else {
            self.no_response_policy(sub_results, &request_info).await
        }
    }

    async fn response_policy_one_succeeded(
        &mut self,
        sub_results: Vec<Result<Value>>,
    ) -> Option<Result<Value>> {
        let mut result: Result<Value> = Ok(Value::BulkString(BulkString::Nil));

        for sub_result in sub_results {
            if let Err(_) | Ok(Value::Error(_)) = sub_result {
                result = sub_result;
            } else {
                return Some(sub_result);
            }
        }

        Some(result)
    }

    async fn response_policy_all_succeeded(
        &mut self,
        sub_results: Vec<Result<Value>>,
    ) -> Option<Result<Value>> {
        let mut result: Result<Value> = Ok(Value::BulkString(BulkString::Nil));

        for sub_result in sub_results {
            if let Err(_) | Ok(Value::Error(_)) = sub_result {
                return Some(sub_result);
            } else {
                result = sub_result;
            }
        }

        Some(result)
    }

    async fn response_policy_agg<F>(
        &mut self,
        sub_results: Vec<Result<Value>>,
        f: F,
    ) -> Option<Result<Value>>
    where
        F: Fn(i64, i64) -> i64,
    {
        let mut result = Value::BulkString(BulkString::Nil);

        for sub_result in sub_results {
            result = match sub_result {
                Ok(Value::Error(_)) => {
                    return Some(sub_result);
                }
                Ok(value) => match (value, result) {
                    (Value::Integer(v), Value::Integer(r)) => Value::Integer(f(v, r)),
                    (Value::Integer(v), Value::BulkString(BulkString::Nil)) => Value::Integer(v),
                    (Value::Array(Array::Vec(v)), Value::Array(Array::Vec(mut r)))
                        if v.len() == r.len() =>
                    {
                        for i in 0..v.len() {
                            match (&v[i], &r[i]) {
                                (Value::Integer(vi), Value::Integer(ri)) => {
                                    r[i] = Value::Integer(f(*vi, *ri));
                                }
                                _ => {
                                    return Some(Err(Error::Client("Unexpected value".to_owned())));
                                }
                            }
                        }
                        Value::Array(Array::Vec(r))
                    }
                    (Value::Array(Array::Vec(v)), Value::BulkString(BulkString::Nil)) => {
                        Value::Array(Array::Vec(v))
                    }
                    _ => {
                        return Some(Err(Error::Client("Unexpected value".to_owned())));
                    }
                },
                Err(_) => {
                    return Some(sub_result);
                }
            };
        }

        Some(Ok(result))
    }

    async fn response_policy_special(
        &mut self,
        _sub_results: Vec<Result<Value>>,
    ) -> Option<Result<Value>> {
        todo!("Command not yet supported in cluster mode");
    }

    async fn no_response_policy(
        &mut self,
        sub_results: Vec<Result<Value>>,
        request_info: &RequestInfo,
    ) -> Option<Result<Value>> {
        if sub_results.len() == 1 {
            // when there is a single sub request, we just read the response
            // on the right connection. For example, GET's reply
            Some(sub_results.into_iter().next()?)
        } else if request_info.keys.is_empty() {
            // The command doesn't accept key name arguments:
            // the client can aggregate all replies within a single nested data structure.
            // For example, the array replies we get from calling KEYS against all shards.
            // These should be packed in a single in no particular order.
            let mut values = Vec::<Value>::new();
            for sub_result in sub_results {
                match sub_result {
                    Ok(Value::Array(Array::Vec(v))) => {
                        values.extend(v);
                    }
                    Err(_) | Ok(Value::Error(_)) => {
                        return Some(sub_result);
                    }
                    _ => {
                        return Some(Err(Error::Client(format!(
                            "Unexpected result {sub_result:?}"
                        ))));
                    }
                }
            }

            Some(Ok(Value::Array(Array::Vec(values))))
        } else {
            // For commands that accept one or more key name arguments:
            // the client needs to retain the same order of replies as the input key names.
            // For example, MGET's aggregated reply.
            let mut results = SmallVec::<[(&String, Value); 10]>::new();

            for (sub_result, sub_request) in zip(sub_results, &request_info.sub_requests) {
                match sub_result {
                    Ok(Value::Array(Array::Vec(values)))
                        if sub_request.keys.len() == values.len() =>
                    {
                        results.extend(zip(&sub_request.keys, values))
                    }
                    Err(_) | Ok(Value::Error(_)) => return Some(sub_result),
                    _ => {
                        return Some(Err(Error::Client(format!(
                            "Unexpected result {:?}",
                            sub_result
                        ))))
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

            let values = results.into_iter().map(|(_, v)| v).collect::<Vec<_>>();
            Some(Ok(Value::Array(Array::Vec(values))))
        }
    }

    pub async fn reconnect(&mut self) -> Result<()> {
        info!("Reconnecting to cluster...");
        let (nodes, slot_ranges) =
            Self::connect_to_cluster(&self.cluster_config, &self.config).await?;
        info!("Reconnected to cluster!");

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
                Ok(mut connection) => match connection.cluster_shards().await {
                    Ok(si) => {
                        shard_info_list = Some(si);
                        break;
                    }
                    Err(e) => warn!(
                        "Cannot execute `cluster_shards` on node ({}:{}): {}",
                        node_config.0, node_config.1, e
                    ),
                },
                Err(e) => warn!(
                    "Cannot connect to node ({}:{}): {}",
                    node_config.0, node_config.1, e
                ),
            }
        }

        let shard_info_list = if let Some(shard_info_list) = shard_info_list {
            shard_info_list
        } else {
            return Err(Error::Client("Cluster misconfiguration".to_owned()));
        };

        let mut nodes = Vec::<Node>::new();
        let mut slot_ranges = Vec::<SlotRange>::new();

        for shard_info in shard_info_list.into_iter() {
            let Some(master_info) = shard_info.nodes.into_iter().find(|n| n.role == "master") else {
                return Err(Error::Client("Cluster misconfiguration".to_owned()));
            };

            let port = master_info.get_port()?;

            let connection = StandaloneConnection::connect(&master_info.ip, port, config).await?;

            slot_ranges.extend(shard_info.slots.iter().map(|s| SlotRange {
                slot_range: *s,
                node_ids: smallvec![master_info.id.clone()],
            }));

            nodes.push(Node {
                id: master_info.id,
                is_master: true,
                address: (master_info.ip, port),
                connection,
            });
        }

        slot_ranges.sort_by_key(|s| s.slot_range.0);
        nodes.sort_by(|n1, n2| n1.id.cmp(&n2.id));

        debug!("Cluster connected: nodes={nodes:?}, slot_ranges={slot_ranges:?}");

        Ok((nodes, slot_ranges))
    }

    async fn connect_replicas(&mut self) -> Result<()> {
        debug!("Connecting replicas...");

        let shard_info_list: Vec<ClusterShardResult> = self
            .get_random_node_mut()
            .connection
            .cluster_shards()
            .await?;

        for shard_info in shard_info_list {
            for node_info in shard_info.nodes.into_iter().filter(|n| n.role == "replica") {
                let port = node_info.get_port()?;

                let connection =
                    StandaloneConnection::connect(&node_info.ip, port, &self.config).await?;

                for slot_range_info in &shard_info.slots {
                    if let Some(slot_range) = self.get_slot_range_by_slot_mut(slot_range_info.0) {
                        if slot_range.slot_range.1 == slot_range_info.1 {
                            slot_range.node_ids.push(node_info.id.clone())
                        }
                    }
                }

                self.nodes.push(Node {
                    id: node_info.id,
                    is_master: false,
                    address: (node_info.ip.clone(), port),
                    connection,
                });
            }
        }

        self.nodes.sort_by(|n1, n2| n1.id.cmp(&n2.id));

        debug!(
            "Cluster replicas connected: nodes={:?}, slot_ranges={:?}",
            self.nodes, self.slot_ranges
        );

        Ok(())
    }

    /// Keep existing connection, connect new nodes, remove obsolte ones
    /// Rebuild slot_ranges from scratch
    async fn refresh_nodes_and_slot_ranges(&mut self) -> Result<()> {
        debug!("Reloading slot ranges");

        let shard_info_list: Vec<ClusterShardResult> = self
            .get_random_node_mut()
            .connection
            .cluster_shards()
            .await?;

        // filter out nodes that do not exist anymore
        let mut node_ids = shard_info_list
            .iter()
            .flat_map(|s| s.nodes.iter().map(|n| n.id.as_str()))
            .collect::<Vec<_>>();
        node_ids.sort();
        self.nodes
            .retain(|node| node_ids.binary_search(&node.id.as_str()).is_ok());

        // create slot_ranges from scratch
        self.slot_ranges.clear();

        // add missing nodes and connect them
        for mut shard_info in shard_info_list {
            // ensure that the first node is master
            if shard_info.nodes[0].role != "master" {
                let Some(master_idx) = shard_info.nodes.iter().position(|n| n.role == "master") else {
                    return Err(Error::Client("Cluster misconfiguration".to_owned()));
                };

                // swap first node & master node
                shard_info.nodes.swap(0, master_idx);
            }

            // add slot_ranges
            for slot_range_info in &shard_info.slots {
                self.slot_ranges.push(SlotRange {
                    slot_range: *slot_range_info,
                    node_ids: shard_info.nodes.iter().map(|n| n.id.clone()).collect(),
                });
            }

            for node_info in shard_info.nodes {
                if let Some(node) = self.nodes.iter_mut().find(|n| n.id == node_info.id) {
                    // refresh is_master flag in case a failover happened
                    node.is_master = node_info.role == "master";
                } else {
                    // add missing node
                    let port = node_info.get_port()?;

                    let connection =
                        StandaloneConnection::connect(&node_info.ip, port, &self.config).await?;

                    self.nodes.push(Node {
                        id: node_info.id,
                        is_master: node_info.role == "master",
                        address: (node_info.ip, port),
                        connection,
                    });
                }
            }
        }

        self.slot_ranges.sort_by_key(|s| s.slot_range.0);
        self.nodes.sort_by(|n1, n2| n1.id.cmp(&n2.id));

        debug!(
            "Cluster new setup: nodes={:?}, slot_ranges={:?}",
            self.nodes, self.slot_ranges
        );

        Ok(())
    }

    #[inline]
    fn get_node_index_by_id(&self, id: &str) -> Option<usize> {
        self.nodes.binary_search_by_key(&id, |n| &n.id).ok()
    }

    #[inline]
    fn get_random_node_index(&self) -> usize {
        rand::thread_rng().gen_range(0..self.nodes.len())
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

    fn hash_slots(keys: &[String]) -> SmallVec<[u16; 10]> {
        keys.iter().map(|k| Self::hash_slot(k)).collect()
    }

    /// Implement hash_slot algorithm
    /// see. https://redis.io/docs/reference/cluster-spec/#hash-tags
    fn hash_slot(key: &str) -> u16 {
        let mut key = key;

        // { found
        if let Some(s) = key.find('{') {
            // } found
            if let Some(e) = key[s + 1..].find('}') {
                // hash tag non empty
                if e != 0 {
                    key = &key[s + 1..s + 1 + e];
                }
            }
        }

        Self::crc16(key) % 16384
    }

    fn crc16(str: &str) -> u16 {
        crc16::State::<crc16::XMODEM>::calculate(str.as_bytes())
    }
}

fn is_push_message(value: &Result<Value>) -> bool {
    match value {
        // RESP2 pub/sub messages
        Ok(Value::Array(Array::Vec(ref items))) => match &items[..] {
            [Value::BulkString(BulkString::Binary(command)), Value::BulkString(BulkString::Binary(_channel)), Value::BulkString(BulkString::Binary(_payload))] =>
            {
                matches!(command.as_slice(), b"message" | b"smessage")
            }
            [Value::BulkString(BulkString::Binary(command)), Value::BulkString(BulkString::Binary(_channel)), Value::Integer(_)] =>
            {
                matches!(
                    command.as_slice(),
                    b"subscribe"
                        | b"psubscribe"
                        | b"ssubscribe"
                        | b"unsubscribe"
                        | b"punsubscribe"
                        | b"sunsubscribe"
                )
            }
            [Value::BulkString(BulkString::Binary(command)), Value::BulkString(BulkString::Binary(_pattern)), Value::BulkString(BulkString::Binary(_channel)), Value::BulkString(BulkString::Binary(_payload))] => {
                command.as_slice() == b"pmessage"
            }
            _ => false,
        },
        // RESP2 monitor events are a SimpleString beginning by a numeric (unix timestamp)
        Ok(Value::SimpleString(monitor_event)) if monitor_event.starts_with(char::is_numeric) => {
            true
        }
        // RESP3
        Ok(Value::Push(_)) => true,
        _ => false,
    }
}
