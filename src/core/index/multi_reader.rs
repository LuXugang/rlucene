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
use crate::core::index::base_composite_reader::{
    BCRStoredFieldsImpl, BCRTermVectorsImpl, BaseCompositeReader, BaseCompositeReaderBase,
};
use crate::core::index::composite_reader::CompositeReader;
use crate::core::index::dummy::dummy_composite_reader::DummyCompositeReader;
#[cfg(test)]
use crate::core::index::dummy::dummy_leaf_reader::DummyLeafReader;
use crate::core::index::index_reader::{
    IndexReader, IndexReaderBase, IndexReaderEnum, IndexReaderEnumCacheHelperType,
};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::term::Term;
use crate::core::util::dummy::dummy_comparator::DummyComparator;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::fmt::{Display, Formatter};

/// A [`CompositeReader`] which reads multiple indexes, appending their content.
/// It can be used to create a view on several sub-readers (like [`DirectoryReader`](crate::core::index::directory_reader::DirectoryReader))
/// and execute searches on it.
///
/// For efficiency, in this API documents are often referred to via *document numbers*,
/// non-negative integers which each name a unique document in the index.
/// These document numbers are ephemeral — they may change as documents are added
/// to and deleted from an index. Clients should thus not rely on a given document
/// having the same number between sessions.
///
/// **NOTE**: [`IndexReader`] instances are completely thread safe, meaning multiple
/// threads can call any of its methods concurrently. If your application requires
/// external synchronization, you should **not** synchronize on the `IndexReader`
/// instance; instead, use your own (non-Lucene) objects.
pub struct MultiReader<LR, CR>
where
    LR: LeafReader + Clone,
    CR: CompositeReader,
{
    base_composite_reader_base: BaseCompositeReaderBase<LR, CR>,
    index_reader_base: IndexReaderBase,
    close_sub_readers: bool,
}

#[cfg(test)]
impl MultiReader<DummyLeafReader, DummyCompositeReader<DummyLeafReader>> {
    pub fn empty() -> Result<Self> {
        Self::with_leaf_reader(vec![])
    }
}
impl<LR> MultiReader<LR, DummyCompositeReader<LR>>
where
    LR: LeafReader + Clone,
{
    pub fn with_leaf_reader(sub_readers: Vec<LR>) -> Result<Self> {
        let base_composite_reader_base =
            BaseCompositeReaderBase::with_leaf_readers::<DummyComparator>(sub_readers, &None)?;
        Self::new(base_composite_reader_base, IndexReaderBase::new(), true)
    }
}

impl<LR, CR> MultiReader<LR, CR>
where
    CR: CompositeReader<LeafReader = LR>,
    LR: LeafReader + Clone,
{
    pub fn with_composite_reader(sub_readers: Vec<CR>) -> Result<Self> {
        let base_composite_reader_base =
            BaseCompositeReaderBase::with_composite_readers::<DummyComparator>(sub_readers, &None)?;
        Self::new(base_composite_reader_base, IndexReaderBase::new(), true)
    }
    fn new(
        base_composite_reader_base: BaseCompositeReaderBase<LR, CR>,
        index_reader_base: IndexReaderBase,
        close_sub_readers: bool,
    ) -> Result<Self> {
        if !close_sub_readers {
            for index_reader in base_composite_reader_base.sub_reader.iter() {
                index_reader.inc_ref()?;
            }
        }
        Ok(Self {
            base_composite_reader_base,
            index_reader_base,
            close_sub_readers,
        })
    }
}
impl<LR, CR> IndexReader for MultiReader<LR, CR>
where
    CR: CompositeReader<LeafReader = LR>,
    LR: LeafReader + Clone,
{
    type TermVectors = BCRTermVectorsImpl<LR, CR>;

    fn term_vectors(&self) -> Result<Self::TermVectors> {
        self.base_composite_reader_base.term_vector(self)
    }

    fn max_doc(&self) -> Result<i32> {
        Ok(self.base_composite_reader_base.max_doc())
    }

    fn num_docs(&self) -> Result<i32> {
        self.base_composite_reader_base.num_docs()
    }

    type StoredFields = BCRStoredFieldsImpl<LR, CR>;

    fn stored_fields(&self) -> Result<Self::StoredFields> {
        self.base_composite_reader_base.stored_fields(self)
    }

    fn do_close(&self) -> Result<()> {
        let mut first_err: Option<LuceneError> = None;

        for r in self.base_composite_reader_base.get_sequential_sub_readers() {
            let result: Result<()> = (|| {
                if self.close_sub_readers {
                    r.close()?
                } else {
                    r.dec_ref()?
                }
                Ok(())
            })();

            if let Err(e) = result
                && first_err.is_none()
            {
                first_err = Some(e);
            }
        }
        if let Some(e) = first_err {
            Err(e)
        } else {
            Ok(())
        }
    }

    type ReaderCacheHelper =
        IndexReaderEnumCacheHelperType<LR::ReaderCacheHelper, CR::ReaderCacheHelper>;

    fn get_reader_cache_helper(&self) -> Result<Option<Self::ReaderCacheHelper>> {
        let readers = self.get_sequential_sub_readers();
        if readers.len() == 1 {
            readers[0].get_reader_cache_helper()
        } else {
            Ok(None)
        }
    }

    fn doc_freq(&self, term: &Term) -> Result<i32> {
        self.base_composite_reader_base.doc_freq(term, self)
    }

    fn total_term_freq(&self, term: &Term) -> Result<i64> {
        self.base_composite_reader_base.total_term_freq(term, self)
    }

    fn get_sum_doc_freq(&self, field: &str) -> Result<i64> {
        self.base_composite_reader_base
            .get_sum_doc_freq(field, self)
    }

    fn get_doc_count(&self, field: &str) -> Result<i32> {
        self.base_composite_reader_base.get_doc_count(field, self)
    }

    fn get_sum_total_term_freq(&self, field: &str) -> Result<i64> {
        self.base_composite_reader_base
            .get_sum_total_term_freq(field, self)
    }

    fn base(&self) -> &IndexReaderBase {
        &self.index_reader_base
    }
}

impl<LR, CR> Display for MultiReader<LR, CR>
where
    CR: CompositeReader<LeafReader = LR>,
    LR: LeafReader + Clone,
{
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl<LR, CR> CompositeReader for MultiReader<LR, CR>
where
    LR: LeafReader + Clone,
    CR: CompositeReader<LeafReader = LR>,
{
    type LeafReader = LR;
    type SubCompositeReader = CR;

    fn get_sequential_sub_readers(
        &self,
    ) -> &[IndexReaderEnum<Self::LeafReader, Self::SubCompositeReader>] {
        self.base_composite_reader_base.get_sequential_sub_readers()
    }

    fn to_string(&self) -> String {
        todo!()
    }
}
impl<LR, CR> BaseCompositeReader for MultiReader<LR, CR>
where
    LR: LeafReader + Clone,
    CR: CompositeReader<LeafReader = LR>,
{
}
