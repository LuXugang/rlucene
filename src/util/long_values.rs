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

/// Abstraction over an array of longs.
pub trait LongValues {
    fn get(&mut self, index: i64) -> Result<i64>;

    /// Add an extra, immutable version of the method.
    /// If you need to call get in an immutable context, you can implement this method.
    fn get_immutable(&self, _index: i64) -> i64 {
        unimplemented!("")
    }
}

pub struct Zeroes;
impl LongValues for Zeroes {
    fn get(&mut self, _index: i64) -> Result<i64> {
        Ok(0)
    }
}
pub struct Identity;
impl LongValues for Identity {
    fn get(&mut self, index: i64) -> Result<i64> {
        Ok(index)
    }
}
