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
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::info_stream::{InfoStream, InfoStreamEnum};

/// Hackidy-Häck-Hack to cause a test to fail on non-bulk merges.
// TODO: we should probably be a wrapper so verbose still works...
pub struct FailOnNonBulkMergesInfoStream;

impl CloseableRef for FailOnNonBulkMergesInfoStream {
  fn close(&self) -> Result<()> {
    Ok(())
  }
}

impl InfoStream for FailOnNonBulkMergesInfoStream {
  fn message(&self, _component: &str, message: &str) -> Result<()> {
    assert!(!message.contains("non-bulk merges"));
    Ok(())
  }

  fn is_enabled(&self, _component: &str) -> bool {
    true
  }
}

impl From<FailOnNonBulkMergesInfoStream> for InfoStreamEnum {
  fn from(info_stream: FailOnNonBulkMergesInfoStream) -> Self {
    InfoStreamEnum::Custom(Box::new(info_stream))
  }
}
