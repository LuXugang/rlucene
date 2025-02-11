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
use crate::store::DataOutput;
use crate::util::error::lucene_error::LuceneError;

pub struct DummyDataOutput;
impl DataOutput for DummyDataOutput {
    fn write_byte(&mut self, _b: u8) -> Result<(), LuceneError> {
        unreachable!("DummyDataOutput should not be called");
    }

    fn write_bytes_range(
        &mut self,
        _b: &[u8],
        _offset: i32,
        _length: i32,
    ) -> Result<(), LuceneError> {
        unreachable!("DummyDataOutput should not be called");
    }
}
