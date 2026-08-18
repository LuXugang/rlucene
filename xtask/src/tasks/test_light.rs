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
use crate::{LogColor, colorize, log, run_cargo_with_env};

pub(crate) fn run(extra_args: &[String]) {
  let mut args = vec!["test".to_string(), "--lib".to_string()];
  args.extend_from_slice(extra_args);
  let args: Vec<&str> = args.iter().map(String::as_str).collect();

  log(&colorize("Running Cargo test-light", LogColor::Green, true));
  run_cargo_with_env(&args, &[("tests.light", "true")]);
  log(&colorize(
    "✅ ✅ ✅ Finished Cargo test-light",
    LogColor::Green,
    true,
  ));
}
