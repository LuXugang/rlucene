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
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::util::error::lucene_error::Result;
use std::marker::PhantomData;
use std::sync::Arc;

/// [`IndexReaderContext`](crate::core::index::index_reader_context::IndexReaderContext) for CompositeReader instance.
pub struct CompositeReaderContext<CR>
where
    CR: CompositeReader,
{
    leaves: Vec<Arc<LeafReaderContext<CR::LeafReader>>>,
    reader: CR,
    base: IndexReaderContextBase,
}
pub(crate) fn create<CR>(reader: CR) -> Result<Arc<CompositeReaderContext<CR>>>
where
    CR: CompositeReader,
    CR: Clone,
    CR::LeafReader: LeafReader<ParentReader = CR>,
{
    let v = IndexReaderEnum::new(reader.clone());
    let base = IndexReaderContextBase::new(true, 0, 0);
    let mut builder = Builder::<CR::LeafReader, CR>::new();
    builder.build::<CR>(v, 0, 0)?;
    let mut leaves = builder.leaves.take().unwrap();
    let ctx_arc = Arc::new_cyclic(|weak_ctx| {
        for leaf in &mut leaves {
            if let Some(leaf_mut) = Arc::get_mut(leaf) {
                leaf_mut.top_parent = Some(weak_ctx.clone());
            } else {
                debug_assert!(false, "LeafReaderContext Arc is not unique");
            }
        }

        CompositeReaderContext {
            leaves,
            reader,
            base,
        }
    });
    Ok(ctx_arc)
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

    fn leaves(&self) -> Result<&[Arc<LeafReaderContext<Self::LeafReader>>]> {
        Ok(self.leaves.as_slice())
    }

    fn base(&self) -> &IndexReaderContextBase {
        &self.base
    }
}

impl<CR> CompositeReaderContext<CR>
where
    CR: CompositeReader,
{
    pub fn identity(&self) -> &Arc<()> {
        self.base.id()
    }

    pub fn contains_leaf(this: &Arc<Self>, leaf: &Arc<LeafReaderContext<CR::LeafReader>>) -> bool
    where
        CR::LeafReader: LeafReader<ParentReader = CR>,
    {
        leaf.top_parent
            .as_ref()
            .unwrap()
            .upgrade()
            .is_some_and(|parent| Arc::ptr_eq(&parent, this))
    }
}

struct Builder<LR, PR>
where
    LR: LeafReader<ParentReader = PR>,
    PR: CompositeReader,
{
    // for easy taken
    pub(crate) leaves: Option<Vec<Arc<LeafReaderContext<LR>>>>,
    pub(crate) leaf_doc_base: i32,
    _marker: PhantomData<PR>,
}
impl<LR, PR> Builder<LR, PR>
where
    LR: LeafReader<ParentReader = PR>,
    PR: CompositeReader,
{
    fn new() -> Self {
        Self {
            leaves: Some(Vec::new()),
            leaf_doc_base: 0,
            _marker: PhantomData,
        }
    }
}
impl<LR, PR> Builder<LR, PR>
where
    LR: LeafReader<ParentReader = PR> + Clone,
    PR: CompositeReader,
{
    fn build<CR>(&mut self, reader: IndexReaderEnum<LR, CR>, ord: i32, doc_base: i32) -> Result<()>
    where
        CR: CompositeReader<LeafReader = LR>,
    {
        match &reader {
            IndexReaderEnum::Leaf(ar) => {
                self.leaf_doc_base += ar.max_doc()?;
                let atomic = Arc::new(LeafReaderContext::new(
                    ar.clone(),
                    ord,
                    doc_base,
                    0,
                    0,
                    None,
                ));
                self.leaves.as_mut().unwrap().push(atomic);
            },
            IndexReaderEnum::Composite(composite_reader) => {
                let sequential_sub_readers = composite_reader.get_sequential_sub_readers();
                for sub_reader in sequential_sub_readers {
                    self.build::<CR::CompositeReader>(
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

    fn leaves(&self) -> Result<&[Arc<LeafReaderContext<Self::LeafReader>>]> {
        Ok(self.leaves.as_slice())
    }

    fn base(&self) -> &IndexReaderContextBase {
        &self.base
    }
}
