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
use crate::store::random_access_input::RandomAccessInput;
use crate::util::error::lucene_error::Result;

pub struct DummyRandomAccessInput;
impl RandomAccessInput for DummyRandomAccessInput {
    fn length(&self) -> i64 {
        unreachable!(" this method should never be called")
    }

    fn read_byte(&mut self, _pos: i64) -> Result<u8> {
        unreachable!(" this method should never be called")
    }

    fn read_short(&mut self, _pos: i64) -> Result<i16> {
        unreachable!(" this method should never be called")
    }

    fn read_int(&mut self, _pos: i64) -> Result<i32> {
        unreachable!(" this method should never be called")
    }

    fn read_long(&mut self, _pos: i64) -> Result<i64> {
        unreachable!(" this method should never be called")
    }

    fn prefetch(&mut self, _pos: i64, _len: i64) -> Result<()> {
        unreachable!(" this method should never be called")
    }
}
