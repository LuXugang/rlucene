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
use crate::index::parallel_postings_array::{PostingsArrayBase, PostingsArrayEnum};

pub(crate) struct TermVectorsPostingsArray;
impl TermVectorsPostingsArray {
    pub(crate) fn new_instance(size: i32) -> TermVectorsPostingsArray {
        todo!()
    }
}
impl PostingsArrayBase for TermVectorsPostingsArray {
    fn bytes_per_posting(&self) -> i32 {
        todo!()
    }

    fn copy_to(&self, to_array: &mut PostingsArrayEnum, num_to_copy: i32) {
        todo!()
    }
}
