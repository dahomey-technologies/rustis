use crate::{
    resp::{Array, BulkString, Command, Value},
    ClusterCommands, ClusterConfig, ClusterShardResult, CommandInfoManager, CommandTip, Connection,
    Error, Future, IntoConfig, RequestPolicy, ResponsePolicy, Result,
};
use futures::{channel::mpsc, SinkExt, Stream, StreamExt};
use log::{debug, trace};
use rand::Rng;
use smallvec::{smallvec, SmallVec};
use std::{
    cmp::Ordering,
    fmt::{Debug, Formatter},
    iter::zip,
    pin::Pin,
    task::{Context, Poll},
};

struct Node {
    pub address: (String, u16),
    pub connection: Connection,
}

impl Debug for Node {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("address", &self.address)
            .finish()
    }
}

#[derive(Debug)]
struct Shard {
    pub slot_range: (u16, u16),
    /// Master & replica nodes. Master is always the first node
    pub nodes: Vec<Node>,
}

#[derive(Debug)]
struct RequestInfo {
    pub command_name: String,
    pub keys: SmallVec<[String; 10]>,
    pub sub_requests: SmallVec<[SubRequest; 10]>,
}

#[derive(Debug)]
struct SubRequest {
    pub shard_index: usize,
    pub node_index: usize,
    pub keys: SmallVec<[String; 10]>,
}

type RequestSender = mpsc::UnboundedSender<RequestInfo>;
type RequestReceiver = mpsc::UnboundedReceiver<RequestInfo>;

struct RequestStream {
    receiver: RequestReceiver,
}

impl Stream for RequestStream {
    type Item = RequestInfo;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.get_mut().receiver.poll_next_unpin(cx)
    }
}

pub struct Cluster {
    shards: Vec<Shard>,
    command_info_manager: CommandInfoManager,
    request_sender: RequestSender,
    request_stream: RequestStream,
}

impl Cluster {
    pub fn connect(cluster_config: &ClusterConfig) -> Future<Cluster> {
        Box::pin(async move {
            let mut shards = Vec::<Shard>::with_capacity(cluster_config.nodes.len());

            for node_config in &cluster_config.nodes {
                let connection = Connection::initialize(node_config.clone().into_config()?).await?;
                shards.push(Shard {
                    slot_range: (0, 0),
                    nodes: vec![Node {
                        address: node_config.clone(),
                        connection,
                    }],
                });
            }

            let shard_infos: Vec<ClusterShardResult> =
                shards[0].nodes[0].connection.cluster_shards().await?;

            for shard in shards.iter_mut() {
                let shard_info = shard_infos.iter().find(|s| {
                    if let Some(port) = s.nodes[0].port {
                        shard.nodes[0].address.0 == s.nodes[0].ip
                            && shard.nodes[0].address.1 == port
                    } else {
                        false
                    }
                });

                if let Some(shard_info) = shard_info {
                    shard.slot_range = shard_info.slots
                } else {
                    return Err(Error::Client("Cluster misconfiguration".to_owned()));
                }
            }

            shards.sort_by_key(|m| m.slot_range.0);

            let command_info_manager =
                CommandInfoManager::initialize(&mut shards[0].nodes[0].connection).await?;

            let (request_sender, request_receiver): (RequestSender, RequestReceiver) =
                mpsc::unbounded();

            let request_stream = RequestStream {
                receiver: request_receiver,
            };

            debug!("Cluster connected: {:?}", shards);

            Ok(Cluster {
                shards,
                command_info_manager,
                request_sender,
                request_stream,
            })
        })
    }

    pub async fn write_batch(&mut self, commands: impl Iterator<Item = &Command>) -> Result<()> {
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

            let keys = self
                .command_info_manager
                .extract_keys(command, &mut self.shards[0].nodes[0].connection)
                .await?;
            let slots = Self::hash_slots(&keys);

            debug!("keys: {keys:?}, slots: {slots:?}");

            if let Some(request_policy) = request_policy {
                match request_policy {
                    // the client should execute the command on all nodes - masters and replicas alike.
                    // An example is the CONFIG SET command.
                    // This tip is in-use by commands that don't accept key name arguments.
                    // The command operates atomically per shard.
                    RequestPolicy::AllNodes => {
                        if self.shards[0].nodes.len() == 1 {
                            self.connect_replicas().await?;
                        }

                        let mut sub_requests = SmallVec::<[SubRequest; 10]>::new();

                        for (shard_index, shard) in &mut self.shards.iter_mut().enumerate() {
                            for (node_index, node) in &mut shard.nodes.iter_mut().enumerate() {
                                node.connection.write(command).await?;
                                sub_requests.push(SubRequest {
                                    shard_index,
                                    node_index,
                                    keys: smallvec![],
                                });
                            }
                        }

                        let request_info = RequestInfo {
                            command_name: command_name.to_string(),
                            sub_requests,
                            keys,
                        };

                        self.request_sender.send(request_info).await?;
                    }
                    // the client should execute the command on all master shards (e.g., the DBSIZE command).
                    // This tip is in-use by commands that don't accept key name arguments.
                    // The command operates atomically per shard.
                    RequestPolicy::AllShards => {
                        for shard in &mut self.shards {
                            shard.nodes[0].connection.write(command).await?;
                        }

                        let request_info = RequestInfo {
                            command_name: command_name.to_string(),
                            sub_requests: (0..self.shards.len())
                                .map(|i| SubRequest {
                                    shard_index: i,
                                    node_index: 0,
                                    keys: smallvec![],
                                })
                                .collect(),
                            keys,
                        };

                        self.request_sender.send(request_info).await?;
                    }
                    // the client should execute the command on several shards.
                    // The shards that execute the command are determined by the hash slots of its input key name arguments.
                    // Examples for such commands include MSET, MGET and DEL.
                    // However, note that SUNIONSTORE isn't considered as multi_shard because all of its keys must belong to the same hash slot.
                    RequestPolicy::MultiShard => {
                        let mut shard_slot_keys = (0..keys.len())
                            .map(|i| {
                                (
                                    self.get_shard_index_by_slot(slots[i]),
                                    slots[i],
                                    keys[i].clone(),
                                )
                            })
                            .collect::<Vec<_>>();

                        shard_slot_keys.sort();

                        trace!("shard_slot_keys: {shard_slot_keys:?}");

                        let mut last_slot = u16::MAX;
                        let mut current_slot_keys = SmallVec::<[String; 10]>::new();
                        let mut sub_requests = SmallVec::<[SubRequest; 10]>::new();
                        let mut last_shard_index = 0;
                        let mut connection: &mut Connection =
                            &mut self.shards[last_shard_index].nodes[0].connection;

                        for (shard_index, slot, key) in &shard_slot_keys {
                            if *slot != last_slot {
                                if !current_slot_keys.is_empty() {
                                    let shard_command =
                                        self.command_info_manager.prepare_command_for_shard(
                                            command,
                                            current_slot_keys.iter(),
                                        )?;
                                    connection.write(&shard_command).await?;
                                    sub_requests.push(SubRequest {
                                        shard_index: last_shard_index,
                                        node_index: 0,
                                        keys: current_slot_keys.clone(),
                                    });

                                    current_slot_keys.clear();
                                }

                                last_slot = *slot;
                            }

                            current_slot_keys.push(key.clone());

                            if *shard_index != last_shard_index {
                                connection = &mut self.shards[*shard_index].nodes[0].connection;
                                last_shard_index = *shard_index;
                            }
                        }

                        let shard_command = self
                            .command_info_manager
                            .prepare_command_for_shard(command, current_slot_keys.iter())?;
                        connection.write(&shard_command).await?;
                        sub_requests.push(SubRequest {
                            shard_index: last_shard_index,
                            node_index: 0,
                            keys: current_slot_keys.clone(),
                        });

                        let request_info = RequestInfo {
                            command_name: command_name.to_string(),
                            keys,
                            sub_requests,
                        };

                        trace!("{request_info:?}");

                        self.request_sender.send(request_info).await?;
                    }
                    RequestPolicy::Special => todo!("Command not yet supported in cluster mode"),
                }
            // test if all slots are equal
            } else if slots.windows(2).all(|s| s[0] == s[1]) {
                let shard_index = if slots.is_empty() {
                    rand::thread_rng().gen_range(0..self.shards.len())
                } else {
                    self.get_shard_index_by_slot(slots[0])
                };

                let connection = &mut self.shards[shard_index].nodes[0].connection;

                connection.write(command).await?;

                let request_info = RequestInfo {
                    command_name: command_name.to_string(),
                    sub_requests: smallvec![SubRequest {
                        shard_index,
                        node_index: 0,
                        keys: keys.clone()
                    }],
                    keys,
                };
                self.request_sender.send(request_info).await?;
            } else {
                return Err(Error::Client(format!(
                    "Cannot send command {} with mistmatched key slots",
                    command_name
                )));
            }
        }

        Ok(())
    }

    pub fn read(&mut self) -> futures::future::BoxFuture<'_, Option<Result<Value>>> {
        Box::pin(async move {
            let request_info = self.request_stream.next().await?;

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
                        let mut value: Result<Value> = Ok(Value::BulkString(BulkString::Nil));

                        for sub_request in request_info.sub_requests {
                            let node = &mut self.shards[sub_request.shard_index].nodes
                                [sub_request.node_index];
                            value = node.connection.read().await?;

                            if let Err(_) | Ok(Value::Error(_)) = value {
                                continue;
                            }

                            return Some(value);
                        }

                        Some(value)
                    }
                    ResponsePolicy::AllSucceeded => {
                        let mut value: Result<Value> = Ok(Value::BulkString(BulkString::Nil));

                        for sub_request in request_info.sub_requests {
                            let node = &mut self.shards[sub_request.shard_index].nodes
                                [sub_request.node_index];
                            value = node.connection.read().await?;

                            if let Err(_) | Ok(Value::Error(_)) = value {
                                return Some(value);
                            }
                        }

                        Some(value)
                    }
                    ResponsePolicy::AggLogicalAnd => {
                        self.read_and_aggregate(
                            request_info,
                            |a, b| if a == 1 && b == 1 { 1 } else { 0 },
                        )
                        .await
                    }
                    ResponsePolicy::AggLogicalOr => {
                        self.read_and_aggregate(
                            request_info,
                            |a, b| if a == 0 && b == 0 { 0 } else { 1 },
                        )
                        .await
                    }
                    ResponsePolicy::AggMin => self.read_and_aggregate(request_info, i64::min).await,
                    ResponsePolicy::AggMax => self.read_and_aggregate(request_info, i64::max).await,
                    ResponsePolicy::AggSum => {
                        self.read_and_aggregate(request_info, |a, b| a + b).await
                    }
                    ResponsePolicy::Special => todo!("Command not yet supported in cluster mode"),
                }
            } else if request_info.sub_requests.len() == 1 {
                // when there is a single sub request, we just read the response
                // on the right connection. For example, GET's reply
                self.shards[request_info.sub_requests[0].shard_index].nodes[0]
                    .connection
                    .read()
                    .await
            } else if request_info.keys.is_empty() {
                // The command doesn't accept key name arguments:
                // the client can aggregate all replies within a single nested data structure.
                // For example, the array replies we get from calling KEYS against all shards.
                // These should be packed in a single in no particular order.
                let mut values = Vec::<Value>::new();
                for sub_request in request_info.sub_requests {
                    let node =
                        &mut self.shards[sub_request.shard_index].nodes[sub_request.node_index];
                    let value = node.connection.read().await?;

                    match value {
                        Ok(Value::Array(Array::Vec(v))) => {
                            values.extend(v);
                        }
                        Err(_) | Ok(Value::Error(_)) => {
                            return Some(value);
                        }
                        _ => {
                            return Some(Err(Error::Client(format!("Unexpected value {value:?}"))));
                        }
                    }
                }

                Some(Ok(Value::Array(Array::Vec(values))))
            } else {
                // For commands that accept one or more key name arguments:
                // the client needs to retain the same order of replies as the input key names.
                // For example, MGET's aggregated reply.
                let mut results = Vec::<(&String, Value)>::new();

                for sub_request in &request_info.sub_requests {
                    let value = self.shards[sub_request.shard_index].nodes[0]
                        .connection
                        .read()
                        .await?;

                    match value {
                        Ok(Value::Array(Array::Vec(values)))
                            if sub_request.keys.len() == values.len() =>
                        {
                            results.extend(zip(&sub_request.keys, values))
                        }
                        Err(_) | Ok(Value::Error(_)) => return Some(value),
                        _ => {
                            return Some(Err(Error::Client(format!(
                                "Unexpected result {:?}",
                                value
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
        })
    }

    async fn connect_replicas(&mut self) -> Result<()> {
        let shard_infos: Vec<ClusterShardResult> =
            self.shards[0].nodes[0].connection.cluster_shards().await?;

        for shard_info in shard_infos {
            let shard = if let Some(shard) = self
                .shards
                .iter_mut()
                .find(|s| s.slot_range == shard_info.slots)
            {
                shard
            } else {
                return Err(Error::Client(format!(
                    "Cannot find shard into for slot range {:?}",
                    shard_info.slots
                )));
            };

            for node in shard_info.nodes.iter().filter(|n| n.role == "replica") {
                if let Some(port) = node.port {
                    let connection =
                        Connection::initialize((node.ip.clone(), port).into_config()?).await?;

                    shard.nodes.push(Node {
                        address: (node.ip.clone(), port),
                        connection,
                    });
                }
            }
        }

        Ok(())
    }

    fn get_shard_index_by_slot(&mut self, slot: u16) -> usize {
        self.shards
            .binary_search_by(|m| {
                if m.slot_range.0 > slot {
                    Ordering::Greater
                } else if m.slot_range.1 < slot {
                    Ordering::Less
                } else {
                    Ordering::Equal
                }
            })
            .unwrap()
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

    async fn read_and_aggregate<F>(
        &mut self,
        request_info: RequestInfo,
        f: F,
    ) -> Option<Result<Value>>
    where
        F: Fn(i64, i64) -> i64,
    {
        let mut result = Value::BulkString(BulkString::Nil);

        for sub_request in request_info.sub_requests {
            let shard = &mut self.shards[sub_request.shard_index];
            let value = shard.nodes[sub_request.node_index]
                .connection
                .read()
                .await?;
            result = match value {
                Ok(Value::Error(_)) => {
                    return Some(value);
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
                    return Some(value);
                }
            };
        }

        Some(Ok(result))
    }
}
