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
use crate::core::document::field::{BinaryTokenStream, Field, StringTokenStream};
use crate::core::document::string_field::TYPE_NOT_STORED;
use crate::core::index::indexable_field::{IndexableField, ReusedIndexingTokenStream};
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::analysis::base_token_stream_test_case::assert_token_stream_contents15;
use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test_framework::core::util::lucene_test_case::random;

/// Test tokenstream reuse by DefaultIndexingChain.
#[allow(dead_code)] // for quick search
struct TestFieldReuse;

#[test]
fn test_string_field() -> Result<()> {
  // IMPORTANT: Rust Lucene not support field reuse yey
  Ok(())
}
