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
use crate::util::error::lucene_error::Result;
/// An object whose RAM usage can be computed.
///
/// # Note
/// This is an internal API.
pub trait Accountable {
    /// Return the memory usage of this object in bytes. Negative values are illegal.
    fn ram_bytes_used(&self) -> Result<i64>;

    /// Returns nested resources of this class. The result should be a point-in-time snapshot (to avoid
    /// race conditions).
    fn get_child_resources<T: Accountable>(&self) -> Vec<T> {
        vec![]
    }
}

#[allow(unused)]
struct EmptyAccountable;
impl EmptyAccountable {
    #[allow(unused)]
    pub fn new() -> Self {
        EmptyAccountable
    }
}
impl Accountable for EmptyAccountable {
    fn ram_bytes_used(&self) -> Result<i64> {
        Ok(0)
    }
}
