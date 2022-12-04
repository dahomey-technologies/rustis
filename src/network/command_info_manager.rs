use crate::{
    commands::{BeginSearch, CommandInfo, FindKeys, ServerCommands},
    resp::{cmd, Command, CommandArg},
    Error, Result, StandaloneConnection,
};
use smallvec::SmallVec;
use std::collections::HashMap;

pub(crate) struct CommandInfoManager {
    command_info_map: HashMap<String, CommandInfo>,
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

        Ok(CommandInfoManager {
            command_info_map: command_info_result
                .into_iter()
                .map(|mut c| {
                    c.name = c.name.to_uppercase();
                    (c.name.to_string(), c)
                })
                .collect(),
        })
    }

    pub fn get_command_info_by_name(&self, command_name: &str) -> Option<&CommandInfo> {
        self.command_info_map.get(command_name)
    }

    pub fn get_command_info(&self, command: &Command) -> Option<&CommandInfo> {
        let command_info = self.command_info_map.get(command.name);
        if let Some(command_info) = command_info {
            if command_info.arity == -2 && !command_info.sub_commands.is_empty() {
                let command_name = format!(
                    "{}|{}",
                    command.name,
                    std::ops::Deref::deref(&command.args[0])
                );
                return self.command_info_map.get(&command_name);
            }
        }

        command_info
    }

    /// see <https://redis.io/docs/reference/key-specs/>
    pub async fn extract_keys(
        &self,
        command: &Command,
        connection: &mut StandaloneConnection,
    ) -> Result<SmallVec<[String; 10]>> {
        let command_info = if let Some(command_info) = self.command_info_map.get(command.name) {
            command_info
        } else {
            return Err(Error::Client(format!("Unknown command {}", command.name)));
        };

        let mut keys = SmallVec::<[String; 10]>::new();

        for key_spec in &command_info.key_specifications {
            let mut slice: &[CommandArg] = &command.args;

            // begin_search
            match &key_spec.begin_search {
                BeginSearch::Index(i) => slice = &slice[*i - 1..],
                BeginSearch::Keyword {
                    keyword,
                    start_from,
                } => {
                    let start_index = if *start_from >= 0 {
                        slice
                            .iter()
                            .skip(*start_from as usize - 1)
                            .position(|arg| *arg == *keyword)
                            .map(|i| i + 1)
                    } else {
                        slice
                            .iter()
                            .rev()
                            .skip((-*start_from - 1) as usize)
                            .position(|arg| *arg == *keyword)
                            .map(|i| slice.len() - (i + -start_from as usize - 1))
                    };

                    if let Some(start_index) = start_index {
                        slice = &slice[start_index..];
                    } else {
                        return Err(Error::Client(format!(
                            "Cannot find keyword {} in {:?}",
                            *keyword, command
                        )));
                    }
                }
                BeginSearch::Unknown => {
                    let args = Self::prepare_command_getkeys_args(command);
                    let keys: SmallVec<[String; 10]> = connection.command_getkeys(args).await?;
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
                        slice.len() / limit - 1
                    } else {
                        slice.len() - (-*last_key as usize)
                    };

                    slice = &slice[..stop_index + 1];
                    *key_step
                }
                FindKeys::KeyEnum {
                    key_num_idx,
                    first_key,
                    key_step,
                } => {
                    let num_keys = slice[*key_num_idx].to_usize()?;

                    slice = &slice[*first_key..num_keys + 1];
                    *key_step
                }
                FindKeys::Unknown => {
                    let args = Self::prepare_command_getkeys_args(command);
                    let keys: SmallVec<[String; 10]> = connection.command_getkeys(args).await?;
                    return Ok(keys);
                }
            };

            keys.extend(slice.iter().step_by(key_step).filter_map(|bs| {
                if bs.is_empty() {
                    None
                } else {
                    Some(bs.to_string())
                }
            }));
        }

        Ok(keys)
    }

    /// Function used for commands which RequestPolicy is MultiShard
    /// Its purpose consists in building a command for a specific shard,
    /// based on the original command provided by the user.
    /// Redis 7 official commands offer multi shard commands only in the context of
    /// BeginSearch::Index & FindKeys::Range with a single key specification.
    ///  We will only support this configuration for the first implementation
    pub fn prepare_command_for_shard<'a>(
        &self,
        command: &Command,
        shard_keys: impl Iterator<Item = &'a String>,
    ) -> Result<Command> {
        let command_info = if let Some(command_info) = self.command_info_map.get(command.name) {
            command_info
        } else {
            return Err(Error::Client(format!("Unknown command {}", command.name)));
        };

        if let Some(key_spec) = command_info.key_specifications.first() {
            let slice: &[CommandArg] = &command.args;
            let mut shard_command = cmd(command.name);

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
                        slice.len() / limit - 1
                    } else {
                        slice.len() - (-*last_key as usize)
                    } + keys_start_index;

                    (keys_end_index, key_step)
                }
                FindKeys::KeyEnum {
                    key_num_idx: _,
                    first_key: _,
                    key_step: _,
                } => todo!("Command not yet supported, ask for it !"),
                FindKeys::Unknown => todo!("Command not yet supported, ask for it !"),
            };

            if keys_start_index > 0 {
                for arg in &slice[..keys_start_index + 1] {
                    shard_command = shard_command.arg(arg.clone());
                }
            }

            for shard_key in shard_keys {
                let key_index =
                    if let Some(key_index) = slice.iter().position(|arg| *arg == *shard_key) {
                        key_index
                    } else {
                        return Err(Error::Client(format!("Cannot find key {}", *shard_key)));
                    };

                for key in &slice[key_index..key_index + key_step] {
                    shard_command = shard_command.arg(key.clone());
                }
            }

            if keys_end_index < command.args.len() - 1 {
                for arg in &slice[keys_end_index..] {
                    shard_command = shard_command.arg(arg.clone());
                }
            }

            return Ok(shard_command);
        }

        unreachable!();
    }

    fn prepare_command_getkeys_args(command: &Command) -> SmallVec<[CommandArg; 10]> {
        let mut args = SmallVec::new();
        args.push(command.name.into());
        args.extend(command.args.into_iter().cloned());
        args
    }
}
