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
use crate::core::index::composite_reader::CompositeReader;
use crate::core::index::index_reader::IndexReaderEnum;
use crate::core::index::index_reader_context::{
    IndexReaderContext, IndexReaderContextBase, IndexReaderContextSealed,
};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::{LeafReaderContext, TopParentMeta};
use crate::core::util::error::lucene_error::Result;
use std::sync::Arc;

/// [`IndexReaderContext`](crate::core::index::index_reader_context::IndexReaderContext) for CompositeReader instance.
pub struct CompositeReaderContext<CR>
where
    CR: CompositeReader,
{
    leaves: Vec<Arc<LeafReaderContext<CR::LeafReader>>>,
    pub(crate) reader: CR,
    base: IndexReaderContextBase,
}
pub(crate) fn create<CR>(reader: CR) -> Result<CompositeReaderContext<Arc<CR>>>
where
    CR: CompositeReader,
{
    let reader = Arc::new(reader);
    let v = IndexReaderEnum::new(reader.clone());
    let base = IndexReaderContextBase::new(true, 0, 0);
    let mut builder = Builder::<CR::LeafReader>::new();
    builder.build(v, 0, 0)?;
    let leaves = builder.leaves.take().unwrap();
    let mut ctx = CompositeReaderContext {
        leaves,
        reader,
        base,
    };
    let top_parent_meta = TopParentMeta {
        leaves_num: ctx.leaves.len(),
        max_doc: builder.max_doc,
        id: ctx.base.id().clone(),
    };
    ctx.leaves.iter_mut().for_each(|leaf| {
        debug_assert!(Arc::strong_count(leaf) == 1);
        Arc::get_mut(leaf).unwrap().top_parent = top_parent_meta.clone();
    });
    Ok(ctx)
}

impl<CR> IndexReaderContextSealed for CompositeReaderContext<CR> where CR: CompositeReader {}

impl<CR> IndexReaderContext for CompositeReaderContext<CR>
where
    CR: CompositeReader,
{
    type IndexReader = CR;

    fn reader(&self) -> &Self::IndexReader {
        &self.reader
    }

    type LeafReader = CR::LeafReader;

    fn leaves(&self) -> Result<Vec<Arc<LeafReaderContext<Self::LeafReader>>>> {
        Ok(self.leaves.clone())
    }

    fn base(&self) -> &IndexReaderContextBase {
        &self.base
    }
}

struct Builder<LR>
where
    LR: LeafReader,
{
    // for easy taken
    pub(crate) leaves: Option<Vec<Arc<LeafReaderContext<LR>>>>,
    pub(crate) leaf_doc_base: i32,
    pub(crate) max_doc: i32,
}
impl<LR> Builder<LR>
where
    LR: LeafReader,
{
    fn new() -> Self {
        Self {
            leaves: Some(Vec::new()),
            leaf_doc_base: 0,
            max_doc: 0,
        }
    }
}
impl<LR> Builder<LR>
where
    LR: LeafReader + Clone,
{
    fn build<CR>(&mut self, reader: IndexReaderEnum<LR, CR>, ord: i32, doc_base: i32) -> Result<()>
    where
        CR: CompositeReader<LeafReader = LR>,
    {
        match &reader {
            IndexReaderEnum::Leaf(ar) => {
                let leaves_size = self.leaves.as_ref().unwrap().len();
                let atomic = Arc::new(LeafReaderContext::new(
                    ar.clone(),
                    ord,
                    doc_base,
                    leaves_size,
                    self.leaf_doc_base,
                    TopParentMeta::default(),
                ));
                self.leaves.as_mut().unwrap().push(atomic);
                let max_doc = ar.max_doc()?;
                self.leaf_doc_base += max_doc;
                self.max_doc += max_doc;
            },
            IndexReaderEnum::Composite(composite_reader) => {
                let sequential_sub_readers = composite_reader.get_sequential_sub_readers();
                for sub_reader in sequential_sub_readers {
                    self.build::<CR::SubCompositeReader>(
                        sub_reader,
                        ord,
                        doc_base + self.leaf_doc_base,
                    )?;
                }
            },
        }
        Ok(())
    }
}

impl<CR> IndexReaderContextSealed for Arc<CompositeReaderContext<CR>> where CR: CompositeReader {}

impl<CR> IndexReaderContext for Arc<CompositeReaderContext<CR>>
where
    CR: CompositeReader,
{
    type IndexReader = CR;

    fn reader(&self) -> &Self::IndexReader {
        &self.reader
    }

    type LeafReader = CR::LeafReader;

    fn leaves(&self) -> Result<Vec<Arc<LeafReaderContext<Self::LeafReader>>>> {
        Ok(self.leaves.clone())
    }

    fn base(&self) -> &IndexReaderContextBase {
        &self.base
    }
}
