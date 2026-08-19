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
mod tasks;

use chrono::Local;
use std::{
  env,
  process::{self, Command},
};

pub(crate) fn run_cargo(args: &[&str]) {
  run_cargo_with_env(args, &[]);
}

pub(crate) fn run_cargo_with_env(args: &[&str], envs: &[(&str, &str)]) {
  let mut command = Command::new("cargo");
  command.args(args);
  for (key, value) in envs {
    command.env(key, value);
  }
  let status = command.status();
  match status {
    Err(e) => {
      log(&format!("Failed to execute cargo: {}", e));
      process::exit(1);
    },
    Ok(exit) if !exit.success() => {
      log(&format!(
        "cargo {:?} exited with status: {}",
        args,
        exit.code().unwrap_or_default()
      ));
      process::exit(1);
    },
    Ok(_) => {},
  }
}

pub(crate) enum LogColor {
  Green,
  Red,
}

impl LogColor {
  fn code(self) -> u8 {
    match self {
      LogColor::Green => 32,
      LogColor::Red => 31,
    }
  }
}

pub(crate) fn colorize(msg: &str, color: LogColor, bold: bool) -> String {
  let code = color.code();
  if bold {
    format!("\x1b[1;{code}m{msg}\x1b[0m")
  } else {
    format!("\x1b[{code}m{msg}\x1b[0m")
  }
}

pub(crate) fn log(msg: &str) {
  let now = Local::now();
  eprintln!("[{}] {}", now.format("%Y-%m-%d %H:%M:%S"), msg);
}

fn main() {
  let mut args = env::args().skip(1);
  let command = args.next();
  match command.as_deref() {
    Some("commands") => tasks::commands::run(),
    Some("tidy") => tasks::tidy::run(),
    Some("commit") => tasks::commit::run(),
    Some("ci") => tasks::ci::run(),
    Some("nightly") => tasks::nightly::run(),
    Some("monster") => tasks::monster::run(),
    Some("test-light") => tasks::test_light::run(),
    Some("check-uncommitted") => tasks::check_uncommitted::run(),
    Some("license-check") => tasks::license::license_check::run(),
    Some("nextest-run") => tasks::nextest::run(),
    _ => {
      log(&format!(
        "{:?} not supported. Run `cargo commands` to see available project commands.",
        command
      ));
      process::exit(1);
    },
  }
}
