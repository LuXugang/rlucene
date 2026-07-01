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
pub use integration_prelude::test;

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

#[path = "../unit_tests/core/analysis/mod.rs"]
pub mod analysis_tests;
#[path = "../unit_tests/core/codecs/mod.rs"]
pub mod codecs_tests;
#[path = "../unit_tests/core/document/mod.rs"]
pub mod document_tests;
#[path = "../unit_tests/core/geo/mod.rs"]
pub mod geo_tests;
#[path = "../unit_tests/queries/mod.rs"]
pub mod queries_tests;
#[path = "../unit_tests/sandbox/document/mod.rs"]
pub mod sandbox_document_tests;
#[path = "../unit_tests/sandbox/search/mod.rs"]
pub mod sandbox_search_tests;
#[path = "../unit_tests/core/store/mod.rs"]
pub mod store_tests;
#[path = "../unit_tests/core/util/mod.rs"]
pub mod util_tests;
