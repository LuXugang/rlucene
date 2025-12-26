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
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::stored_fields::{StoredFields, StoredFieldsEnum2};
use crate::core::index::term::Term;
use crate::core::index::term_vectors::{TermVectors, TermVectorsEnum2};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

pub trait IndexReader: Display {
    type TermVectors<'a>: TermVectors
    where
        Self: 'a;
    fn term_vectors(&self) -> Result<Self::TermVectors<'_>>;

    fn max_doc(&self) -> Result<i32>;

    fn num_docs(&self) -> Result<i32>;

    fn num_deleted_docs(&self) -> Result<i32> {
        Ok(self.max_doc()? - self.num_docs()?)
    }

    fn inc_ref(&self) -> Result<()> {
        if !self.try_inc_ref() {
            self.ensure_open()?;
        }
        Ok(())
    }

    fn dec_ref(&self) -> Result<()> {
        // only check ref_count here (don't call ensure_open()),
        // so we can still close the reader if it was made invalid by a child.
        let base = self.base();
        let count = base.ref_count.load(Ordering::Acquire);
        if count <= 0 {
            return Err(LuceneError::already_closed(
                "this IndexReader is closed".to_string(),
            ));
        }

        let rc = base.ref_count.fetch_sub(1, Ordering::AcqRel) - 1;
        if rc == 0 {
            base.closed.store(true, Ordering::Release);
            // TODO: 这里要通知parent Rust Lucene 需要吗
            self.do_close()?;
        } else if rc < 0 {
            return Err(LuceneError::illegal_state(format!(
                "too many decRef calls: refCount is {} after decrement",
                rc
            )));
        }

        Ok(())
    }

    fn ensure_open(&self) -> Result<()> {
        let base = self.base();
        if base.ref_count.load(Ordering::Acquire) <= 0 {
            return Err(LuceneError::already_closed(
                "this IndexReader is closed".to_string(),
            ));
        }

        // The "happens-before" rule on reading ref_count after a fake write
        // ensures visibility of closed_by_child state.
        if base.closed_by_child.load(Ordering::Acquire) {
            return Err(LuceneError::already_closed(
                "this IndexReader cannot be used anymore as one of its child readers was closed"
                    .to_string(),
            ));
        }

        Ok(())
    }

    type StoredFields<'a>: StoredFields
    where
        Self: 'a;
    fn stored_fields(&self) -> Result<Self::StoredFields<'_>>;

    fn has_deletions(&self) -> Result<bool> {
        Ok(self.num_deleted_docs()? > 0)
    }
    fn close(&self) -> Result<()> {
        let base = self.base();
        if !base.closed.load(Ordering::Acquire) {
            self.dec_ref()?;
            base.closed.store(true, Ordering::Release);
        }
        Ok(())
    }

    fn do_close(&self) -> Result<()>;

    /// Optional method: Return a [`CacheHelper`] that can be used to cache based on the content of
    /// this reader. Two readers that have different data or different sets of deleted documents will
    /// be considered different.
    ///
    /// A return value of `None` indicates that this reader is not suited for caching, which
    /// is typically the case for short-lived wrappers that alter the content of the wrapped reader.
    ///
    /// # lucene.experimental
    type ReaderCacheHelper: CacheHelper;
    fn get_reader_cache_helper(&self) -> Result<Option<Self::ReaderCacheHelper>>;

    /// Returns the number of documents containing the `term`.
    /// This method returns `0` if the term or field does not exist.
    /// This method does not take into account deleted documents that
    /// have not yet been merged away.
    ///
    /// See [`TermsEnum::doc_freq`](crate::core::index::terms_enum::TermsEnum::doc_freq).
    fn doc_freq(&self, term: &Term) -> Result<i32>;

    /// Returns the total number of occurrences of `term` across all documents
    /// (the sum of the `freq()` for each doc that has this term).
    /// Note that, like other term measures, this measure does not take
    /// deleted documents into account.
    fn total_term_freq(&self, term: &Term) -> Result<i64>;
    /// Returns the sum of [`TermsEnum::doc_freq`](crate::core::index::terms_enum::TermsEnum::doc_freq) for all terms in this field.
    /// Note that, just like other term measures, this measure does not take
    /// deleted documents into account.
    ///
    /// See [`Terms::get_sum_doc_freq`](crate::core::index::terms::Terms::get_sum_doc_freq).
    fn get_sum_doc_freq(&self, field: &str) -> Result<i64>;
    /// Returns the number of documents that have at least one term for this field.
    /// Note that, just like other term measures, this measure does not take
    /// deleted documents into account.
    ///
    /// See [`Terms::get_doc_count`](crate::core::index::terms::Terms::get_doc_count).
    fn get_doc_count(&self, field: &str) -> Result<i32>;

    /// Returns the sum of [`TermsEnum::total_term_freq`](crate::core::index::terms_enum::TermsEnum::total_term_freq) for all terms in this field.
    /// Note that, just like other term measures, this measure does not take
    /// deleted documents into account.
    ///
    /// See [`Terms::get_sum_total_term_freq`](crate::core::index::terms::Terms::get_sum_total_term_freq).
    fn get_sum_total_term_freq(&self, field: &str) -> Result<i64>;

    fn base(&self) -> &IndexReaderBase;

    fn try_inc_ref(&self) -> bool {
        let base = self.base();
        loop {
            let count = base.ref_count.load(Ordering::Acquire);
            if count <= 0 {
                return false;
            }

            match base.ref_count.compare_exchange(
                count,
                count + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return true,
                Err(_) => continue,
            }
        }
    }

    fn get_ref_count(&self) -> i32 {
        let base = self.base();
        base.ref_count.load(Ordering::Acquire)
    }
}

pub struct IndexReaderBase {
    closed: AtomicBool,
    closed_by_child: AtomicBool,
    ref_count: AtomicI32,
}
impl IndexReaderBase {
    pub(crate) fn new() -> Self {
        Self {
            closed: AtomicBool::new(false),
            closed_by_child: AtomicBool::new(false),
            ref_count: AtomicI32::new(1),
        }
    }
}

pub trait CacheHelper {
    fn get_key(&self) -> CacheKey;
}
pub enum CacheHelperEnum2<A, B> {
    A(A),
    B(B),
}
impl<A, B> CacheHelper for CacheHelperEnum2<A, B>
where
    A: CacheHelper,
    B: CacheHelper,
{
    fn get_key(&self) -> CacheKey {
        match self {
            CacheHelperEnum2::A(a) => a.get_key(),
            CacheHelperEnum2::B(b) => b.get_key(),
        }
    }
}

pub type CacheKey = Identity;

pub type IRTermVectors<'a, LR, CR> =
    TermVectorsEnum2<<LR as IndexReader>::TermVectors<'a>, <CR as IndexReader>::TermVectors<'a>>;
pub type IRStoredFields<'a, LR, CR> =
    StoredFieldsEnum2<<LR as IndexReader>::StoredFields<'a>, <CR as IndexReader>::StoredFields<'a>>;

pub type IndexReaderEnumCacheHelperType<A, B> = CacheHelperEnum2<A, B>;

pub enum IndexReaderEnum<LR, CR>
where
    LR: LeafReader,
    CR: CompositeReader,
{
    Leaf(LR),
    Composite(CR),
}
impl<CR> IndexReaderEnum<CR::LeafReader, CR>
where
    CR: CompositeReader,
{
    pub(crate) fn new(reader: CR) -> Self {
        IndexReaderEnum::Composite(reader)
    }
}

impl<LR, CR> Display for IndexReaderEnum<LR, CR>
where
    CR: CompositeReader,
    LR: LeafReader,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexReaderEnum::Leaf(leaf) => write!(f, "LeafReader: {}", leaf),
            IndexReaderEnum::Composite(comp) => write!(f, "CompositeReader: {}", comp),
        }
    }
}

impl<LR, CR> IndexReader for IndexReaderEnum<LR, CR>
where
    LR: LeafReader,
    CR: CompositeReader,
{
    type TermVectors<'a>
        = IRTermVectors<'a, LR, CR>
    where
        CR: 'a,
        LR: 'a;

    fn term_vectors(&self) -> Result<Self::TermVectors<'_>> {
        match self {
            IndexReaderEnum::Leaf(leaf) => Ok(TermVectorsEnum2::A(leaf.term_vectors()?)),
            IndexReaderEnum::Composite(comp) => Ok(TermVectorsEnum2::B(comp.term_vectors()?)),
        }
    }

    fn max_doc(&self) -> Result<i32> {
        match self {
            IndexReaderEnum::Leaf(leaf) => leaf.max_doc(),
            IndexReaderEnum::Composite(comp) => comp.max_doc(),
        }
    }

    fn num_docs(&self) -> Result<i32> {
        match self {
            IndexReaderEnum::Leaf(leaf) => leaf.num_docs(),
            IndexReaderEnum::Composite(comp) => comp.num_docs(),
        }
    }

    fn num_deleted_docs(&self) -> Result<i32> {
        match self {
            IndexReaderEnum::Leaf(leaf) => leaf.num_deleted_docs(),
            IndexReaderEnum::Composite(comp) => comp.num_deleted_docs(),
        }
    }

    fn inc_ref(&self) -> Result<()> {
        match self {
            IndexReaderEnum::Leaf(leaf) => leaf.inc_ref(),
            IndexReaderEnum::Composite(comp) => comp.inc_ref(),
        }
    }

    fn dec_ref(&self) -> Result<()> {
        match self {
            IndexReaderEnum::Leaf(leaf) => leaf.dec_ref(),
            IndexReaderEnum::Composite(comp) => comp.dec_ref(),
        }
    }

    fn ensure_open(&self) -> Result<()> {
        match self {
            IndexReaderEnum::Leaf(leaf) => leaf.ensure_open(),
            IndexReaderEnum::Composite(comp) => comp.ensure_open(),
        }
    }

    type StoredFields<'a>
        = IRStoredFields<'a, LR, CR>
    where
        CR: 'a,
        LR: 'a;

    fn stored_fields(&self) -> Result<Self::StoredFields<'_>> {
        match self {
            IndexReaderEnum::Leaf(leaf) => Ok(StoredFieldsEnum2::A(leaf.stored_fields()?)),
            IndexReaderEnum::Composite(comp) => Ok(StoredFieldsEnum2::B(comp.stored_fields()?)),
        }
    }

    fn has_deletions(&self) -> Result<bool> {
        match self {
            IndexReaderEnum::Leaf(leaf) => leaf.has_deletions(),
            IndexReaderEnum::Composite(comp) => comp.has_deletions(),
        }
    }

    fn do_close(&self) -> Result<()> {
        match self {
            IndexReaderEnum::Leaf(leaf) => leaf.do_close(),
            IndexReaderEnum::Composite(comp) => comp.do_close(),
        }
    }

    type ReaderCacheHelper =
        IndexReaderEnumCacheHelperType<LR::ReaderCacheHelper, CR::ReaderCacheHelper>;

    fn get_reader_cache_helper(&self) -> Result<Option<Self::ReaderCacheHelper>> {
        match self {
            IndexReaderEnum::Leaf(leaf) => {
                if let Some(helper) = leaf.get_reader_cache_helper()? {
                    Ok(Some(IndexReaderEnumCacheHelperType::A(helper)))
                } else {
                    Ok(None)
                }
            },
            IndexReaderEnum::Composite(comp) => {
                if let Some(helper) = comp.get_reader_cache_helper()? {
                    Ok(Some(IndexReaderEnumCacheHelperType::B(helper)))
                } else {
                    Ok(None)
                }
            },
        }
    }

    fn doc_freq(&self, term: &Term) -> Result<i32> {
        match self {
            IndexReaderEnum::Leaf(leaf) => <LR as IndexReader>::doc_freq(leaf, term),
            IndexReaderEnum::Composite(comp) => comp.doc_freq(term),
        }
    }

    fn total_term_freq(&self, term: &Term) -> Result<i64> {
        match self {
            IndexReaderEnum::Leaf(leaf) => <LR as IndexReader>::total_term_freq(leaf, term),
            IndexReaderEnum::Composite(comp) => comp.total_term_freq(term),
        }
    }

    fn get_sum_doc_freq(&self, field: &str) -> Result<i64> {
        match self {
            IndexReaderEnum::Leaf(leaf) => leaf.get_sum_doc_freq(field),
            IndexReaderEnum::Composite(comp) => comp.get_sum_doc_freq(field),
        }
    }

    fn get_doc_count(&self, field: &str) -> Result<i32> {
        match self {
            IndexReaderEnum::Leaf(leaf) => leaf.get_doc_count(field),
            IndexReaderEnum::Composite(comp) => comp.get_doc_count(field),
        }
    }

    fn get_sum_total_term_freq(&self, field: &str) -> Result<i64> {
        match self {
            IndexReaderEnum::Leaf(leaf) => leaf.get_sum_total_term_freq(field),
            IndexReaderEnum::Composite(comp) => comp.get_sum_total_term_freq(field),
        }
    }

    fn base(&self) -> &IndexReaderBase {
        match self {
            IndexReaderEnum::Leaf(leaf) => leaf.base(),
            IndexReaderEnum::Composite(comp) => comp.base(),
        }
    }
}
impl<IR> IndexReader for &IR
where
    IR: IndexReader + ?Sized,
{
    type TermVectors<'a>
        = IR::TermVectors<'a>
    where
        Self: 'a;

    fn term_vectors(&self) -> Result<Self::TermVectors<'_>> {
        (**self).term_vectors()
    }

    fn max_doc(&self) -> Result<i32> {
        (**self).max_doc()
    }

    fn num_docs(&self) -> Result<i32> {
        (**self).num_docs()
    }

    fn num_deleted_docs(&self) -> Result<i32> {
        (**self).num_deleted_docs()
    }

    fn inc_ref(&self) -> Result<()> {
        (**self).inc_ref()
    }

    fn dec_ref(&self) -> Result<()> {
        (**self).dec_ref()
    }

    fn ensure_open(&self) -> Result<()> {
        (**self).ensure_open()
    }

    type StoredFields<'a>
        = IR::StoredFields<'a>
    where
        Self: 'a;

    fn stored_fields(&self) -> Result<Self::StoredFields<'_>> {
        (**self).stored_fields()
    }

    fn has_deletions(&self) -> Result<bool> {
        (**self).has_deletions()
    }

    fn do_close(&self) -> Result<()> {
        (**self).do_close()
    }

    type ReaderCacheHelper = IR::ReaderCacheHelper;

    fn get_reader_cache_helper(&self) -> Result<Option<Self::ReaderCacheHelper>> {
        (**self).get_reader_cache_helper()
    }

    fn doc_freq(&self, term: &Term) -> Result<i32> {
        (**self).doc_freq(term)
    }

    fn total_term_freq(&self, term: &Term) -> Result<i64> {
        (**self).total_term_freq(term)
    }

    fn get_sum_doc_freq(&self, field: &str) -> Result<i64> {
        (**self).get_sum_doc_freq(field)
    }

    fn get_doc_count(&self, field: &str) -> Result<i32> {
        (**self).get_doc_count(field)
    }

    fn get_sum_total_term_freq(&self, field: &str) -> Result<i64> {
        (**self).get_sum_total_term_freq(field)
    }

    fn base(&self) -> &IndexReaderBase {
        (**self).base()
    }
}
impl<IR> IndexReader for Arc<IR>
where
    IR: IndexReader + ?Sized,
{
    type TermVectors<'a>
        = IR::TermVectors<'a>
    where
        Self: 'a;

    fn term_vectors(&self) -> Result<Self::TermVectors<'_>> {
        (**self).term_vectors()
    }

    fn max_doc(&self) -> Result<i32> {
        (**self).max_doc()
    }

    fn num_docs(&self) -> Result<i32> {
        (**self).num_docs()
    }

    fn num_deleted_docs(&self) -> Result<i32> {
        (**self).num_deleted_docs()
    }

    fn inc_ref(&self) -> Result<()> {
        (**self).inc_ref()
    }

    fn dec_ref(&self) -> Result<()> {
        (**self).dec_ref()
    }

    fn ensure_open(&self) -> Result<()> {
        (**self).ensure_open()
    }

    type StoredFields<'a>
        = IR::StoredFields<'a>
    where
        Self: 'a;

    fn stored_fields(&self) -> Result<Self::StoredFields<'_>> {
        (**self).stored_fields()
    }

    fn has_deletions(&self) -> Result<bool> {
        (**self).has_deletions()
    }

    fn do_close(&self) -> Result<()> {
        (**self).do_close()
    }

    type ReaderCacheHelper = IR::ReaderCacheHelper;

    fn get_reader_cache_helper(&self) -> Result<Option<Self::ReaderCacheHelper>> {
        (**self).get_reader_cache_helper()
    }

    fn doc_freq(&self, term: &Term) -> Result<i32> {
        (**self).doc_freq(term)
    }

    fn total_term_freq(&self, term: &Term) -> Result<i64> {
        (**self).total_term_freq(term)
    }

    fn get_sum_doc_freq(&self, field: &str) -> Result<i64> {
        (**self).get_sum_doc_freq(field)
    }

    fn get_doc_count(&self, field: &str) -> Result<i32> {
        (**self).get_doc_count(field)
    }

    fn get_sum_total_term_freq(&self, field: &str) -> Result<i64> {
        (**self).get_sum_total_term_freq(field)
    }

    fn base(&self) -> &IndexReaderBase {
        (**self).base()
    }
}

#[derive(Debug)]
struct IdentityTag(u8);
#[derive(Debug, Clone)]
pub struct Identity(Arc<IdentityTag>);

impl Identity {
    pub(crate) fn new() -> Self {
        Identity(Arc::new(IdentityTag(0)))
    }

    #[inline]
    fn ptr(&self) -> *const IdentityTag {
        Arc::as_ptr(&self.0)
    }
}

impl PartialEq for Identity {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.ptr(), other.ptr())
    }
}
impl Eq for Identity {}

impl Hash for Identity {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.ptr() as usize).hash(state);
    }
}
