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
#![cfg_attr(not(test), forbid(clippy::mutable_key_type))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![cfg_attr(not(test), deny(clippy::unwrap_used))]
// The monolithic macOS lib-test binary exceeds ld's compact-unwind table limit.
#![cfg_attr(all(test, target_os = "macos"), allow(linker_messages))]
#[macro_use]
mod macros;

pub mod analysis;
pub mod codec;
pub mod core;
pub mod migration_notes;
pub mod queries;
pub mod queryparser;
pub mod sandbox;

#[cfg(test)]
#[macro_use]
#[path = "../test_framework/macros.rs"]
mod test_framework_macros;

#[cfg(test)]
#[path = "../test_framework/mod.rs"]
pub(crate) mod test_framework;

#[cfg(test)]
pub mod test;
