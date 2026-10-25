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
use crate::core::util::access::ByteSource;
use std::sync::Arc;

/// Floor bytes acquire an owner only after a frame receives real index data.
#[derive(Default)]
pub(crate) enum FloorData {
  #[default]
  Empty,
  Shared(Arc<Vec<u8>>),
}

impl FloorData {
  pub(crate) fn shares_storage_with(&self, bytes: &Arc<Vec<u8>>) -> bool {
    matches!(self, Self::Shared(current) if Arc::ptr_eq(current, bytes))
  }
}

impl ByteSource for FloorData {
  fn as_slice(&self) -> &[u8] {
    match self {
      Self::Empty => &[],
      Self::Shared(bytes) => bytes,
    }
  }
}
