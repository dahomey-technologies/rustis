use crate::{
    Error, Result, StandaloneConnection,
    commands::{BeginSearch, CommandInfo, FindKeys, ServerCommands},
    network::Version,
    resp::{Command, CommandArgsMut, NetworkCommand, cmd},
};
use smallvec::SmallVec;
use std::collections::HashMap;

pub(crate) struct CommandInfoManager {
    command_info_map: HashMap<Vec<u8>, CommandInfo>,
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
                    (c.name.as_bytes().to_vec(), c)
                })
                .collect(),
            legacy: version.major < 7,
        })
    }

    pub fn get_command_info_by_name(&self, command_name: &str) -> Option<&CommandInfo> {
        self.command_info_map.get(command_name.as_bytes())
    }

    pub fn get_command_info(&self, command: &NetworkCommand) -> Option<&CommandInfo> {
        // let command_info = self.command_info_map.get(command.get_name());
        // if let Some(command_info) = command_info
        //     && command_info.arity == -2
        //     && !command_info.sub_commands.is_empty()
        //     && let Some(first_arg) = command.args().next()
        //     && let Ok(first_arg) = std::str::from_utf8(first_arg)
        // {
        //     let command_name = format!("{}|{}", command.get_name(), first_arg);
        //     return self.command_info_map.get(&command_name);
        // }

        // command_info
        todo!()
    }

    /// see <https://redis.io/docs/reference/key-specs/>
    pub async fn extract_keys(
        &self,
        command: &Command,
        connection: &mut StandaloneConnection,
    ) -> Result<SmallVec<[String; 10]>> {/*
        let command_info = if let Some(command_info) = self.command_info_map.get(command.name) {
            command_info
        } else {
            return Err(Error::Client(format!("Unknown command {}", command.name)));
        };

        if self.legacy {
            if command_info.first_key == 0 || command_info.last_key == 0 {
                return Ok(SmallVec::new());
            } else if command_info.flags.iter().any(|f| f == "movablekeys") {
                let args = Self::prepare_command_getkeys_args(command);
                let keys: SmallVec<[String; 10]> = connection.command_getkeys(args).await?;
                return Ok(keys);
            } else {
                let args_len = command.args.len() - (command_info.first_key - 1);
                let stop_index = if command_info.last_key >= 0 {
                    command_info.last_key as usize
                } else {
                    args_len - (-command_info.last_key as usize) + 1
                };

                let mut keys = SmallVec::new();
                for index in (command_info.first_key - 1..stop_index).step_by(command_info.step) {
                    if let Some(arg) = command.args.get_arg(index)
                        && !arg.is_empty()
                    {
                        keys.push(String::from_utf8(arg.to_vec())?);
                    }
                }

                return Ok(keys);
            }
        }

        let mut keys = SmallVec::<[String; 10]>::new();

        for key_spec in &command_info.key_specifications {
            let args_len = command.args.len();
            let mut start_index = 0;

            // begin_search
            match &key_spec.begin_search {
                BeginSearch::Index(i) => {
                    start_index = *i - 1;
                }
                BeginSearch::Keyword {
                    keyword,
                    start_from,
                } => {
                    let keyword_bytes = keyword.as_bytes();

                    let found_index = if *start_from >= 0 {
                        command
                            .args
                            .iter()
                            .skip(*start_from as usize - 1)
                            .position(|arg| arg == keyword_bytes)
                            .map(|i| i + *start_from as usize)
                    } else {
                        command
                            .args
                            .iter()
                            .rev()
                            .skip((-*start_from - 1) as usize)
                            .position(|arg| arg == keyword_bytes)
                            .map(|i| args_len - (i + -start_from as usize - 1))
                    };

                    if let Some(found_index) = found_index {
                        start_index = found_index;
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

            // // find_keys
            // let key_step = match &key_spec.find_keys {
            //     FindKeys::Range {
            //         last_key,
            //         key_step,
            //         limit,
            //     } => {
            //         let stop_index = if *last_key >= 0 {
            //             *last_key as usize
            //         } else if *last_key == -1 && *limit >= 2 {
            //             slice.len() / limit - 1
            //         } else {
            //             slice.len() - (-*last_key as usize)
            //         };

            //         slice = &slice[..stop_index + 1];
            //         *key_step
            //     }
            //     FindKeys::KeyNum {
            //         key_num_idx,
            //         first_key,
            //         key_step,
            //     } => {
            //         let num_keys = slice[*key_num_idx].as_slice();
            //         let num_keys: usize = atoi::atoi(num_keys).ok_or_else(|| {
            //             Error::Client(format!(
            //                 "Cannot parse integer from {}",
            //                 String::from_utf8_lossy(num_keys)
            //             ))
            //         })?;

            //         slice = &slice[*first_key..num_keys + 1];
            //         *key_step
            //     }
            //     FindKeys::Unknown {} => {
            //         let args = Self::prepare_command_getkeys_args(command);
            //         let keys: SmallVec<[String; 10]> = connection.command_getkeys(args).await?;
            //         return Ok(keys);
            //     }
            // };

            // keys.extend(slice.iter().step_by(key_step).filter_map(|bs| {
            //     if bs.is_empty() {
            //         None
            //     } else {
            //         String::from_utf8(bs.clone()).ok()
            //     }
            // }));
        }

        Ok(keys)*/
        todo!()
    }

    /// Function used for commands which RequestPolicy is MultiShard
    /// Its purpose consists in building a command for a specific shard,
    /// based on the original command provided by the user.
    /// Redis 7 official commands offer multi shard commands only in the context of
    /// BeginSearch::Index & FindKeys::Range with a single key specification.
    ///  We will only support this configuration for the first implementation
    pub fn prepare_command_for_shard<'a>(
        &self,
        command: &NetworkCommand,
        shard_keys: impl Iterator<Item = &'a String>,
    ) -> Result<NetworkCommand> {
        let command_info = if let Some(command_info) = self.command_info_map.get(command.get_name()) {
            command_info
        } else {
            return Err(Error::Client(format!("Unknown command {}", String::from_utf8_lossy(command.get_name()))));
        };

        if let Some(key_spec) = command_info.key_specifications.first() {
            //let slice: &[Vec<u8>] = &command.args;
            //let mut shard_command = cmd(command.get_name);

            // // begin_search
            // let keys_start_index = match &key_spec.begin_search {
            //     BeginSearch::Index(i) => *i - 1,
            //     BeginSearch::Keyword {
            //         keyword: _,
            //         start_from: _,
            //     } => todo!("Command not yet supported, ask for it !"),
            //     BeginSearch::Unknown => todo!("Command not yet supported, ask for it !"),
            // };

            // // find_keys
            // let (keys_end_index, key_step) = match &key_spec.find_keys {
            //     FindKeys::Range {
            //         last_key,
            //         key_step,
            //         limit,
            //     } => {
            //         let keys_end_index = if *last_key >= 0 {
            //             *last_key as usize
            //         } else if *last_key == -1 && *limit >= 2 {
            //             slice.len() / limit - 1
            //         } else {
            //             slice.len() - (-*last_key as usize)
            //         } + keys_start_index;

            //         (keys_end_index, key_step)
            //     }
            //     FindKeys::KeyNum {
            //         key_num_idx: _,
            //         first_key: _,
            //         key_step: _,
            //     } => todo!("Command not yet supported, ask for it !"),
            //     FindKeys::Unknown {} => todo!("Command not yet supported, ask for it !"),
            // };

            // if keys_start_index > 0 {
            //     for arg in &slice[..keys_start_index + 1] {
            //         shard_command = shard_command.arg(arg.clone());
            //     }
            // }

            // for shard_key in shard_keys {
            //     let key_index = if let Some(key_index) = slice
            //         .iter()
            //         .position(|arg| arg.as_slice() == shard_key.as_bytes())
            //     {
            //         key_index
            //     } else {
            //         return Err(Error::Client(format!("Cannot find key {}", *shard_key)));
            //     };

            //     for key in &slice[key_index..key_index + key_step] {
            //         shard_command = shard_command.arg(key.clone());
            //     }
            // }

            // if keys_end_index < command.args.len() - 1 {
            //     for arg in &slice[keys_end_index..] {
            //         shard_command = shard_command.arg(arg.clone());
            //     }
            // }

            //return Ok(shard_command);
            todo!()
        }

        unreachable!();
    }

    // fn prepare_command_getkeys_args(command: &Command) -> CommandArgs {
    //     let mut args = CommandArgs::default();
    //     args.arg(command.name);
    //     args.arg(&command.args);
    //     args
    // }
}
