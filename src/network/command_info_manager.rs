use crate::{
    Error, Result, StandaloneConnection,
    commands::{BeginSearch, CommandInfo, FindKeys, ServerCommands},
    network::Version,
    resp::{Command, CommandArgs, CommandArgsMut, CommandBuilder},
};
use bytes::Bytes;
use smallvec::SmallVec;
use std::collections::HashMap;

pub(crate) struct CommandInfoManager {
    command_info_map: HashMap<Bytes, CommandInfo>,
    legacy: bool,
}

impl CommandInfoManager {
    pub async fn initialize(connection: &mut StandaloneConnection) -> Result<CommandInfoManager> {
        let mut command_info_result = connection.command().await?;
        let sub_commands = command_info_result
            .iter()
            .filter_map(|c| {
                if c.sub_commands.is_empty() {
                    None
                } else {
                    Some(c.sub_commands.clone())
                }
            })
            .flatten()
            .collect::<Vec<_>>();
        command_info_result.extend(sub_commands);

        let version: Version = connection.get_version().try_into()?;

        Ok(CommandInfoManager {
            command_info_map: command_info_result
                .into_iter()
                .map(|mut c| {
                    c.name = c.name.to_uppercase();
                    (Bytes::copy_from_slice(c.name.as_bytes()), c)
                })
                .collect(),
            legacy: version.major < 7,
        })
    }

    pub fn get_command_info_by_name(&self, command_name: &[u8]) -> Option<&CommandInfo> {
        self.command_info_map.get(command_name)
    }

    pub fn get_command_info(&self, command: &Command) -> Option<&CommandInfo> {
        let command_info = self.command_info_map.get(&command.get_name());
        if let Some(command_info) = command_info
            && command_info.arity == -2
            && !command_info.sub_commands.is_empty()
            && let Some(first_arg) = command.args().next()
            && let Ok(first_arg) = std::str::from_utf8(&first_arg)
        {
            let command_name = format!("{}|{}", command_info.name, first_arg);
            return self.command_info_map.get(&Bytes::from(command_name));
        }

        command_info
    }

    /// see <https://redis.io/docs/reference/key-specs/>
    pub async fn extract_keys(
        &self,
        command: &Command,
        connection: &mut StandaloneConnection,
    ) -> Result<SmallVec<[Bytes; 10]>> {
        let command_info =
            if let Some(command_info) = self.command_info_map.get(&command.get_name()) {
                command_info
            } else {
                return Err(Error::Client(format!(
                    "Unknown command {}",
                    String::from_utf8_lossy(&command.get_name())
                )));
            };

        if self.legacy {
            if command_info.first_key == 0 || command_info.last_key == 0 {
                return Ok(SmallVec::new());
            } else if command_info.flags.iter().any(|f| f == "movablekeys") {
                let args = Self::prepare_command_getkeys_args(command);
                let keys: SmallVec<[Bytes; 10]> = connection.command_getkeys(args).await?;
                return Ok(keys);
            } else {
                let mut idx_range = command_info.first_key - 1..command.num_args();
                let stop_index = if command_info.last_key >= 0 {
                    command_info.last_key as usize
                } else {
                    idx_range.len() - (-command_info.last_key as usize) + 1
                };
                idx_range.end = stop_index;

                let keys = idx_range
                    .step_by(command_info.step)
                    .filter_map(|i| command.get_arg(i).filter(|a| !a.is_empty()))
                    .collect();
                return Ok(keys);
            }
        }

        let mut keys = SmallVec::<[Bytes; 10]>::new();

        for key_spec in &command_info.key_specifications {
            let mut idx_range = 0..command.num_args();

            // begin_search
            match &key_spec.begin_search {
                BeginSearch::Index(i) => idx_range.start = *i - 1,
                BeginSearch::Keyword {
                    keyword,
                    start_from,
                } => {
                    let start_index = if *start_from >= 0 {
                        command
                            .args()
                            .skip(*start_from as usize - 1)
                            .position(|arg| arg.as_ref() == keyword.as_bytes())
                            .map(|i| i + *start_from as usize)
                    } else {
                        command
                            .args()
                            .rev()
                            .skip(-start_from as usize - 1)
                            .position(|arg| arg.as_ref() == keyword.as_bytes())
                            .map(|i| idx_range.len() - (i + -start_from as usize - 1))
                    };

                    if let Some(start_index) = start_index {
                        idx_range.start = start_index;
                    } else {
                        return Err(Error::Client(format!(
                            "Cannot find keyword {} in {:?}",
                            *keyword, command
                        )));
                    }
                }
                BeginSearch::Unknown => {
                    let args = Self::prepare_command_getkeys_args(command);
                    let keys: SmallVec<[Bytes; 10]> = connection.command_getkeys(args).await?;
                    return Ok(keys);
                }
            }

            // find_keys
            let key_step = match &key_spec.find_keys {
                FindKeys::Range {
                    last_key,
                    key_step,
                    limit,
                } => {
                    let stop_index = if *last_key >= 0 {
                        *last_key as usize
                    } else if *last_key == -1 && *limit >= 2 {
                        idx_range.len() / limit - 1
                    } else {
                        idx_range.len() - (-*last_key as usize)
                    } + idx_range.start;

                    idx_range.end = stop_index + 1;
                    *key_step
                }
                FindKeys::KeyNum {
                    key_num_idx,
                    first_key,
                    key_step,
                } => {
                    let Some(num_keys) = command.get_arg(idx_range.start + *key_num_idx) else {
                        return Err(Error::Client("Bad keynum".to_string()));
                    };
                    log::debug!("keynum: {}", String::from_utf8_lossy(&num_keys));

                    let num_keys: usize = atoi::atoi(&num_keys).ok_or_else(|| {
                        Error::Client(format!(
                            "Cannot parse integer from {}",
                            String::from_utf8_lossy(&num_keys)
                        ))
                    })?;

                    idx_range.start += *first_key;
                    idx_range.end = idx_range.start + num_keys;
                    *key_step
                }
                FindKeys::Unknown {} => {
                    let args = Self::prepare_command_getkeys_args(command);
                    let keys: SmallVec<[Bytes; 10]> = connection.command_getkeys(args).await?;
                    return Ok(keys);
                }
            };

            keys.extend(
                idx_range
                    .step_by(key_step)
                    .filter_map(|i| command.get_arg(i).filter(|a| !a.is_empty())),
            );
        }

        Ok(keys)
    }

    /// Function used for commands which RequestPolicy is MultiShard
    /// Its purpose consists in building a command for a specific shard,
    /// based on the original command provided by the user.
    /// Redis 7 official commands offer multi shard commands only in the context of
    /// BeginSearch::Index & FindKeys::Range with a single key specification.
    /// We will only support this configuration for the first implementation
    pub fn prepare_command_for_shard<'a>(
        &self,
        command: &Command,
        shard_keys: impl Iterator<Item = &'a Bytes>,
    ) -> Result<Command> {
        let command_info =
            if let Some(command_info) = self.command_info_map.get(&command.get_name()) {
                command_info
            } else {
                return Err(Error::Client(format!(
                    "Unknown command {}",
                    String::from_utf8_lossy(command.get_name().as_ref())
                )));
            };

        if let Some(key_spec) = command_info.key_specifications.first() {
            let mut shard_command = CommandBuilder::new(&command.get_name());

            // begin_search
            let keys_start_index = match &key_spec.begin_search {
                BeginSearch::Index(i) => *i - 1,
                BeginSearch::Keyword {
                    keyword: _,
                    start_from: _,
                } => todo!("Command not yet supported, ask for it !"),
                BeginSearch::Unknown => todo!("Command not yet supported, ask for it !"),
            };

            // find_keys
            let (keys_end_index, key_step) = match &key_spec.find_keys {
                FindKeys::Range {
                    last_key,
                    key_step,
                    limit,
                } => {
                    let keys_end_index = if *last_key >= 0 {
                        *last_key as usize
                    } else if *last_key == -1 && *limit >= 2 {
                        command.num_args() / limit - 1
                    } else {
                        command.num_args() - (-*last_key as usize)
                    } + keys_start_index;

                    (keys_end_index, key_step)
                }
                FindKeys::KeyNum {
                    key_num_idx: _,
                    first_key: _,
                    key_step: _,
                } => todo!("Command not yet supported, ask for it !"),
                FindKeys::Unknown {} => todo!("Command not yet supported, ask for it !"),
            };

            if keys_start_index > 0 {
                for idx in 0..keys_start_index + 1 {
                    if let Some(key) = command.get_arg(idx) {
                        shard_command = shard_command.arg(key);
                    }
                }
            }

            for shard_key in shard_keys {
                let key_index =
                    if let Some(key_index) = command.args().position(|arg| arg == shard_key) {
                        key_index
                    } else {
                        return Err(Error::Client(format!(
                            "Cannot find key {}",
                            String::from_utf8_lossy(&shard_key)
                        )));
                    };

                for idx in key_index..key_index + key_step {
                    if let Some(key) = command.get_arg(idx) {
                        shard_command = shard_command.arg(key);
                    }
                }
            }

            if keys_end_index < command.num_args() - 1 {
                for idx in keys_end_index..command.num_args() {
                    if let Some(key) = command.get_arg(idx) {
                        shard_command = shard_command.arg(key);
                    }
                }
            }

            return Ok(shard_command.into());
        }

        unreachable!();
    }

    fn prepare_command_getkeys_args(command: &Command) -> CommandArgs {
        let mut args = CommandArgsMut::default().arg(command.get_name());
        for arg in command.args() {
            args = args.arg(arg);
        }
        args.freeze()
    }
}
