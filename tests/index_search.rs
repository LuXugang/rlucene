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
#![allow(dead_code)]
#![allow(unused_imports)]

#[macro_use]
#[path = "../test_framework/integration_prelude.rs"]
mod integration_prelude;
pub use integration_prelude::test_framework;

#[path = "../src/analysis/mod.rs"]
pub mod analysis;
#[path = "../src/core/mod.rs"]
pub mod core;
#[path = "../src/migration_notes.rs"]
pub mod migration_notes;
#[path = "../src/queries/mod.rs"]
pub mod queries;
#[path = "../src/queryparser/mod.rs"]
pub mod queryparser;
#[path = "../src/sandbox/mod.rs"]
pub mod sandbox;

#[path = "../unit_tests/core/index/mod.rs"]
pub mod index_tests;
#[path = "../unit_tests/core/search/mod.rs"]
pub mod search_tests;
#[path = "../unit_tests/core/test_demo.rs"]
mod test_demo;
#[path = "../unit_tests/core/test_search.rs"]
mod test_search;
#[path = "../unit_tests/core/test_search_for_duplicates.rs"]
mod test_search_for_duplicates;
