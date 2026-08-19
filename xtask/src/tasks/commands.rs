/*
 * Licensed to the Apache Software Foundation (ASF) under one or more
 * contributor license agreements.  See the NOTICE file distributed with
 * this work for additional information regarding copyright ownership.
 * The ASF licenses this file to You under the Apache License, Version 2.0
 * (the "License"); you may not use this file except in compliance with
 * the License.  You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use std::{
  env, fs,
  io::IsTerminal,
  path::{Path, PathBuf},
  process,
};

struct CommandInfo {
  name: String,
  purpose: String,
}

fn find_config_file() -> Result<PathBuf, String> {
  let mut directory = env::current_dir().map_err(|error| error.to_string())?;
  loop {
    let candidate = directory.join(".cargo/config.toml");
    if candidate.is_file() {
      return Ok(candidate);
    }
    if !directory.pop() {
      return Err("unable to find .cargo/config.toml".to_string());
    }
  }
}

fn read_commands(config_file: &Path) -> Result<Vec<CommandInfo>, String> {
  let config = fs::read_to_string(config_file).map_err(|error| error.to_string())?;
  let mut commands = Vec::new();
  let mut in_alias_section = false;
  let mut purpose = None;

  for line in config.lines() {
    let line = line.trim();
    if line == "[alias]" {
      in_alias_section = true;
      continue;
    }
    if line.starts_with('[') {
      in_alias_section = false;
      purpose = None;
      continue;
    }
    if !in_alias_section {
      continue;
    }
    if let Some(comment) = line.strip_prefix('#') {
      let comment = comment.trim();
      if !comment.is_empty() {
        purpose = Some(comment.to_string());
      }
      continue;
    }
    if line.is_empty() {
      continue;
    }

    let Some((name, _)) = line.split_once('=') else {
      purpose = None;
      continue;
    };
    let name = name.trim();
    let command_purpose = purpose
      .take()
      .unwrap_or_else(|| "No description provided.".to_string());
    if name != "xtask" {
      commands.push(CommandInfo {
        name: name.to_string(),
        purpose: command_purpose,
      });
    }
  }

  commands.sort_by(|left, right| left.name.cmp(&right.name));
  Ok(commands)
}

fn style(text: &str, code: &str, color_enabled: bool) -> String {
  if color_enabled {
    format!("\x1b[{code}m{text}\x1b[0m")
  } else {
    text.to_string()
  }
}

pub(crate) fn run() {
  let config_file = match find_config_file() {
    Ok(config_file) => config_file,
    Err(error) => {
      eprintln!("Failed to locate Cargo command configuration: {error}");
      process::exit(1);
    },
  };
  let commands = match read_commands(&config_file) {
    Ok(commands) => commands,
    Err(error) => {
      eprintln!("Failed to read {}: {error}", config_file.display());
      process::exit(1);
    },
  };
  let color_enabled = std::io::stdout().is_terminal() && env::var_os("NO_COLOR").is_none();

  println!(
    "{}",
    style("Project-specific Cargo commands", "1;36", color_enabled)
  );
  println!(
    "{}",
    style("================================", "36", color_enabled)
  );
  println!("Run `cargo commands` from the rlucene project root to show this list.\n");

  for command in commands {
    println!(
      "{}",
      style(&format!("  cargo {}", command.name), "1;32", color_enabled)
    );
    println!("    {}", command.purpose);
  }
}
