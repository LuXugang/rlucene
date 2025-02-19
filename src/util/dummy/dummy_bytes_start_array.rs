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
use crate::util::bytes_ref_hash::BytesStartArray;
use crate::util::dummy::dummy_counter::DummyCounter;
use crate::util::error::lucene_error::LuceneError;
use crate::util::CounterEnum;
use std::sync::{Arc, Mutex};

pub struct DummyBytesStartArray {
    dummy_vec: Vec<i32>,
}
impl Default for DummyBytesStartArray {
    fn default() -> Self {
        Self::new()
    }
}

impl DummyBytesStartArray {
    pub fn new() -> Self {
        Self { dummy_vec: vec![] }
    }
}
impl BytesStartArray for DummyBytesStartArray {
    fn init(&mut self) -> &Vec<i32> {
        debug_assert!(false, "should never be called");
        &self.dummy_vec
    }

    fn grow(&mut self) -> Result<(), LuceneError> {
        Err(LuceneError::not_implemented("DummyBytesStartArray::grow"))
    }

    fn clear(&mut self) -> Result<(), LuceneError> {
        Err(LuceneError::not_implemented("DummyBytesStartArray::clear"))
    }

    fn bytes_used(&mut self) -> Arc<Mutex<CounterEnum>> {
        debug_assert!(false, "should never be called");
        Arc::new(Mutex::new(CounterEnum::Dummy(DummyCounter)))
    }

    fn byte_start(&mut self) -> &mut Option<Vec<i32>> {
        todo!()
    }
}
