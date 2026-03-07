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
use crate::core::index::index_reader_context::{IndexReaderContext, IndexReaderContextBase};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::{LeafReaderContext, TopParentMeta};
use crate::core::util::TryIntoInt;
use crate::core::util::error::lucene_error::{LuceneError, Result};

/// [`IndexReaderContext`] for CompositeReader instance.
pub struct CompositeReaderContext<CR>
where
    CR: CompositeReader,
{
    leaves: Vec<LeafReaderContext<CR::LeafReader>>,
    reader: CR,
    base: IndexReaderContextBase,
}
impl<CR> CompositeReaderContext<CR>
where
    CR: CompositeReader,
{
    pub(crate) fn reader(&self) -> &CR {
        &self.reader
    }
}
pub(crate) fn create<CR>(reader: CR) -> Result<CompositeReaderContext<CR>>
where
    CR: CompositeReader,
{
    let v = IndexReaderEnum::new(reader);
    let base = IndexReaderContextBase::new(true, 0, 0);
    let mut builder = Builder::<CR::LeafReader>::new();
    builder.build(&v, 0, 0)?;
    let max_doc = builder.max_doc;
    let leaves = builder.leaves;
    let reader = match v {
        IndexReaderEnum::Composite(composite_reader) => composite_reader,
        _ => {
            return Err(LuceneError::illegal_state(
                "CompositeReaderContext can only be created from CompositeReader",
            ));
        },
    };
    let mut ctx = CompositeReaderContext {
        leaves,
        reader,
        base,
    };
    let top_parent_meta = TopParentMeta {
        leaves_num: ctx.leaves.len(),
        max_doc,
        id: ctx.base.id().clone(),
    };
    ctx.leaves.iter_mut().for_each(|leaf| {
        leaf.top_parent = top_parent_meta.clone();
    });
    Ok(ctx)
}

impl<CR> IndexReaderContext for CompositeReaderContext<CR>
where
    CR: CompositeReader,
{
    type IndexReader = CR;

    fn reader(&self) -> &Self::IndexReader {
        &self.reader
    }

    type LeafReader = CR::LeafReader;

    fn leaves(&self) -> Result<&[LeafReaderContext<Self::LeafReader>]> {
        Ok(self.leaves.as_ref())
    }

    fn base(&self) -> &IndexReaderContextBase {
        &self.base
    }
}

struct Builder<LR>
where
    LR: LeafReader,
{
    pub(crate) leaves: Vec<LeafReaderContext<LR>>,
    pub(crate) leaf_doc_base: usize,
    pub(crate) max_doc: i32,
}
impl<LR> Builder<LR>
where
    LR: LeafReader,
{
    fn new() -> Self {
        Self {
            leaves: Vec::new(),
            leaf_doc_base: 0,
            max_doc: 0,
        }
    }
}
impl<LR> Builder<LR>
where
    LR: LeafReader + Clone,
{
    fn build<CR>(
        &mut self,
        reader: &IndexReaderEnum<LR, CR>,
        ord: i32,
        doc_base: usize,
    ) -> Result<()>
    where
        CR: CompositeReader<LeafReader = LR>,
    {
        match &reader {
            IndexReaderEnum::Leaf(ar) => {
                let leaves_size = self.leaves.len();
                let atomic = LeafReaderContext::new(
                    ar.clone(),
                    ord,
                    doc_base,
                    leaves_size,
                    self.leaf_doc_base,
                    TopParentMeta::default(),
                );
                self.leaves.push(atomic);
                let max_doc = ar.max_doc()?;
                self.leaf_doc_base += max_doc.try_convert()?;
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
