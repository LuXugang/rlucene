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
use crate::index::BytesRef;
use crate::util::attribute_source::AttributeSource;
use std::borrow::Cow;

pub struct DummyAttributeSource;
impl AttributeSource for DummyAttributeSource {
    fn start_offset(&self) -> Option<i32> {
        unimplemented!("start_offset() must be implemented if it needs to be used")
    }

    fn end_offset(&self) -> Option<i32> {
        unimplemented!("start_offset() must be implemented if it needs to be used")
    }

    fn get_position_increment(&self) -> Option<i32> {
        unimplemented!("start_offset() must be implemented if it needs to be used")
    }

    fn get_payload(&self) -> Option<&BytesRef<Vec<u8>>> {
        unimplemented!("start_offset() must be implemented if it needs to be used")
    }

    fn get_bytes_ref(&mut self) -> Option<Cow<BytesRef<Vec<u8>>>> {
        unimplemented!("start_offset() must be implemented if it needs to be used")
    }

    fn get_term_frequency(&self) -> Option<i32> {
        unimplemented!("start_offset() must be implemented if it needs to be used")
    }
}
