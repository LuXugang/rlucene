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
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::search::collector::Collector;
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::weight::Weight;
use crate::core::util::error::lucene_error::Result;

pub trait SimpleCollector: Collector + LeafCollector {
    fn do_set_next_reader<LR>(&mut self, context: &LeafReaderContext<LR>) -> Result<()>
    where
        LR: LeafReader;

    fn get_leaf_collector<W, LR, IRC>(
        &mut self,
        context: &LeafReaderContext<LR>,
        _weight: Option<&W>,
    ) -> Result<()>
    where
        LR: LeafReader,
        IRC: IndexReaderContext,
        W: Weight<IRC> + ?Sized,
    {
        self.do_set_next_reader(context)
    }
}
