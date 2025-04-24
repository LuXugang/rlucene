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
use crate::index::doc_values_skipper::DocValuesSkipper;
use crate::util::error::lucene_error::Result;

pub struct DummyDocValuesSkipper;
impl DocValuesSkipper for DummyDocValuesSkipper {
    fn advance(&mut self, _target: i32) -> Result<()> {
        todo!()
    }

    fn num_levels(&self) -> i32 {
        todo!()
    }

    fn min_doc_id(&self, _level: i32) -> i32 {
        todo!()
    }

    fn max_doc_id(&self, _level: i32) -> i32 {
        todo!()
    }

    fn min_value(&self, _level: i32) -> i64 {
        todo!()
    }

    fn max_value(&self, _level: i32) -> i64 {
        todo!()
    }

    fn doc_count_level(&self, _level: i32) -> i32 {
        todo!()
    }

    fn global_min_value(&self) -> i64 {
        todo!()
    }

    fn global_max_value(&self) -> i64 {
        todo!()
    }

    fn global_doc_count(&self) -> i32 {
        todo!()
    }

    fn advance_by_range(&mut self, _min_value: i64, _max_value: i64) -> Result<()> {
        todo!()
    }
}
