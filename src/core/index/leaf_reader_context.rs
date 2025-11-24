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
use crate::core::index::composite_reader_context::CompositeReaderContext;
use crate::core::index::index_reader_context::{
    IndexReaderContext, IndexReaderContextBase, IndexReaderContextSealed,
};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::fmt;
use std::sync::{Arc, Weak};

/// [`IndexReaderContext`] for [`LeafReader`] instances.
pub struct LeafReaderContext<LR>
where
    LR: LeafReader,
{
    /// The reader's ord in the top-level's leaves array
    pub(crate) ord: usize,
    /// The reader's absolute doc base
    pub(crate) doc_base: i32,
    reader: LR,
    base: IndexReaderContextBase,
    pub(crate) top_parent: Option<Weak<CompositeReaderContext<<LR as LeafReader>::ParentReader>>>,
}
impl<LR> LeafReaderContext<LR>
where
    LR: LeafReader,
{
    pub fn new(
        reader: LR,
        ord: i32,
        doc_base: i32,
        leaf_ord: usize,
        leaf_doc_base: i32,
        parent: Option<Weak<CompositeReaderContext<<LR as LeafReader>::ParentReader>>>,
    ) -> Self {
        Self {
            ord: leaf_ord,
            doc_base: leaf_doc_base,
            reader,
            base: IndexReaderContextBase::new(false, ord, doc_base),
            top_parent: parent,
        }
    }

    pub fn new_single(reader: LR) -> Self {
        Self::new(reader, 0, 0, 0, 0, None)
    }
}
impl<LR> IndexReaderContextSealed for LeafReaderContext<LR> where LR: LeafReader {}

impl<LR> IndexReaderContext for LeafReaderContext<LR>
where
    LR: LeafReader,
{
    type IndexReader = LR;

    fn reader(&self) -> &Self::IndexReader {
        &self.reader
    }

    type LeafReader = LR;

    fn leaves(&self) -> Result<Vec<Arc<LeafReaderContext<Self::LeafReader>>>> {
        if !self.base.is_top_level {
            return Err(LuceneError::unsupported_operation(
                "This is not a top-level context".to_string(),
            ));
        }
        Ok(vec![])
    }

    fn base(&self) -> &IndexReaderContextBase {
        &self.base
    }
}

impl<LR> LeafReaderContext<LR>
where
    LR: LeafReader,
{
    pub fn top_parent(&self) -> Option<Arc<CompositeReaderContext<LR::ParentReader>>> {
        self.top_parent.as_ref().unwrap().upgrade()
    }
}
impl<LR> fmt::Display for LeafReaderContext<LR>
where
    LR: LeafReader,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "LeafReaderContext({} docBase={} ord={})",
            self.reader, self.doc_base, self.ord
        )
    }
}
