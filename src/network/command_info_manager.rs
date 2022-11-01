use crate::{
    resp::{BulkString, Command},
    BeginSearch, CommandInfo, Error, FindKeys, Result, ServerCommands, Streams,
};
use smallvec::SmallVec;
use std::collections::HashMap;

pub(crate) struct CommandInfoManager {
    command_info_map: HashMap<String, CommandInfo>,
}

impl CommandInfoManager {
    pub async fn initialize(streams: &mut Streams) -> Result<CommandInfoManager> {
        let command_info_result = streams.command().await?;

        Ok(CommandInfoManager {
            command_info_map: command_info_result
                .into_iter()
                .map(|c| (c.name.clone(), c))
                .collect(),
        })
    }

    /// see <https://redis.io/docs/reference/key-specs/>
    pub async fn extract_keys(
        &self,
        command: &Command,
        streams: &mut Streams,
    ) -> Result<SmallVec<[String; 10]>> {
        let command_info = self.command_info_map.get(&command.name.to_lowercase());

        if let Some(command_info) = command_info {
            let mut keys = SmallVec::<[String; 10]>::new();

            for key_spec in &command_info.key_specifications {
                let mut slice: &[BulkString] = &command.args;

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
                        let keys: SmallVec<[String; 10]> = streams.command_getkeys(args).await?;
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
                    },
                    FindKeys::Unknown => {
                        let args = Self::prepare_command_getkeys_args(command);
                        let keys: SmallVec<[String; 10]> = streams.command_getkeys(args).await?;
                        return Ok(keys);
                    }
                };

                keys.extend(
                    slice
                        .iter()
                        .step_by(key_step)
                        .filter_map(|bs| {
                            if bs.is_empty() {
                                None
                            } else {
                                Some(bs.to_string())
                            }
                        }),
                );
            }

            Ok(keys)
        } else {
            Err(Error::Client(format!("Unknown command {}", command.name)))
        }
    }

    fn prepare_command_getkeys_args(command: &Command) -> SmallVec<[BulkString; 10]> {
        let mut args = SmallVec::new();
        args.push(command.name.into());
        args.extend(command.args.into_iter().map(|c| c.clone()));
        args
    }
}
