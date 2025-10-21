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
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::util::error::lucene_error::Result;

/// [`IndexReaderContext`](crate::core::index::index_reader_context::IndexReaderContext) for CompositeReader instance.
pub struct CompositeReaderContext<CR>
where
    CR: CompositeReader,
{
    leaves: Option<Vec<LeafReaderContext<CR::LeafReader>>>,
    reader: CR,
}
pub(crate) fn create<CR>(reader: CR) -> Result<CompositeReaderContext<CR>>
where
    CR: CompositeReader,
    CR: Clone,
{
    let v = IndexReaderEnum::new(reader.clone());
    let mut builder = Builder::new();
    builder.build(v, 0, 0)?;
    let leaves = builder.leaves.take().unwrap();
    Ok(CompositeReaderContext {
        leaves: Some(leaves),
        reader,
    })
}
// impl<LR> IndexReaderContext for CompositeReaderContext<LR> where LR: LeafReader {
//     type IndexReader = ();
//
//     fn reader(&self) -> &Self::IndexReader {
//         todo!()
//     }
//
//     type LeafReader = ();
//
//     fn leaves(&self) -> Result<&[LeafReaderContext<Self::LeafReader>]> {
//         todo!()
//     }
//
//     fn children(&self) -> Option<&[IndexReaderContextEnum<Self::LeafReader>]> {
//         todo!()
//     }
//
//     fn base(&self) -> &IndexReaderContextBase<Self::LeafReader> {
//         todo!()
//     }
//
//     fn base_mut(&mut self) -> &mut IndexReaderContextBase<Self::LeafReader> {
//         todo!()
//     }
// }

struct Builder<LR>
where
    LR: LeafReader,
{
    // for easy taken
    pub(crate) leaves: Option<Vec<LeafReaderContext<LR>>>,
    pub(crate) leaf_doc_base: i32,
}
impl<LR> Builder<LR>
where
    LR: LeafReader,
{
    fn new() -> Self {
        Self {
            leaves: Some(Vec::new()),
            leaf_doc_base: 0,
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
                self.leaf_doc_base += ar.max_doc()?;
                let atomic = LeafReaderContext::new(ar.clone(), ord, doc_base, 0, 0);
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
