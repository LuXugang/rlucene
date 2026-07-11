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
use crate::core::util::IOUtils;
use crate::core::util::accountable::Accountable;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

/// Provides an interface for accessing a point-in-time view of an index.
///
/// Any changes made to the index via an
/// [`IndexWriter`](crate::core::index::index_writer::IndexWriter) will not be
/// visible until a new [`IndexReader`] is opened. If the
/// [`IndexWriter`](crate::core::index::index_writer::IndexWriter) is
/// in-process, it is best to obtain an [`IndexReader`] with
/// [`directory_reader::open_from_writer`](crate::core::index::directory_reader::open_from_writer).
/// When reopening is needed in order to see changes to the index, it is best to
/// use [`directory_reader::open_if_changed`](crate::core::index::directory_reader::open_if_changed),
/// since the new reader will share resources with the previous one when
/// possible. Searching an index is done entirely through this abstract
/// interface, so that any implementation is searchable.
///
/// There are two different types of index readers:
///
/// - [`LeafReader`]: atomic readers that do not consist of several sub-readers.
///   They support retrieval of stored fields, doc values, terms, and postings.
/// - [`CompositeReader`]: instances, such as
///   [`DirectoryReader`](crate::core::index::directory_reader::DirectoryReader),
///   can only be used to get stored fields from the underlying
///   [`LeafReader`]s. It is not possible to directly retrieve postings from a
///   composite reader; to do that, get the sub-readers via
///   [`CompositeReader::get_sequential_sub_readers`].
///
/// [`IndexReader`] instances for indexes on disk are usually constructed with a
/// call to one of the `DirectoryReader::open` methods, for example
/// [`directory_reader::open`](crate::core::index::directory_reader::open).
/// [`DirectoryReader`](crate::core::index::directory_reader::DirectoryReader)
/// implements the [`CompositeReader`] interface, so it is not possible to
/// directly get postings from it.
///
/// For efficiency, this API often refers to documents via document numbers:
/// non-negative integers that each name a unique document in the index. These
/// document numbers are ephemeral and may change as documents are added to and
/// deleted from an index. Clients should not rely on a document having the same
/// number between sessions.
///
/// NOTE: [`IndexReader`] instances are completely thread safe, meaning multiple
/// threads can call any of their methods concurrently. If your application
/// requires external synchronization, do not synchronize on the reader instance;
/// use your own non-Lucene objects instead.
pub trait IndexReader: Display {
  type TermVectors: TermVectors;

  /// Returns a [`TermVectors`] reader for the term vectors of this index.
  ///
  /// This call never returns `None`, even if no term vectors were indexed. The
  /// returned instance should only be used by a single thread.
  fn term_vectors(&self) -> Result<Self::TermVectors>;

  /// Returns one greater than the largest possible document number.
  ///
  /// This may be used, for example, to determine how big to allocate an array
  /// that will have an element for every document number in an index.
  fn max_doc(&self) -> Result<i32>;

  /// Returns the number of documents in this index.
  ///
  /// NOTE: This operation may run in `O(max_doc)`. Implementations that cannot
  /// return this number in constant time should cache it.
  fn num_docs(&self) -> Result<i32>;

  /// Returns the number of deleted documents.
  ///
  /// NOTE: This operation may run in `O(max_doc)`.
  fn num_deleted_docs(&self) -> Result<i32> {
    Ok(self.max_doc()? - self.num_docs()?)
  }

  /// Expert: increments the ref count of this [`IndexReader`] instance.
  ///
  /// Ref counts are used to determine when a reader can be closed safely, as
  /// soon as there are no more references. Be sure to always call a
  /// corresponding [`Self::dec_ref`], otherwise the reader may never be closed.
  /// [`Self::close`] simply calls [`Self::dec_ref`], which means the reader will
  /// not really be closed until [`Self::dec_ref`] has been called for all
  /// outstanding references.
  fn inc_ref(&self) -> Result<()> {
    if !self.try_inc_ref() {
      self.ensure_open()?;
    }
    Ok(())
  }

  /// Expert: decreases the ref count of this [`IndexReader`] instance.
  ///
  /// If the ref count drops to `0`, then this reader is closed. If an error is
  /// hit, the ref count is unchanged.
  fn dec_ref(&self) -> Result<()> {
    // only check ref_count here (don't call ensure_open()),
    // so we can still close the reader if it was made invalid by a child.
    let base = self.index_base();
    let count = base.ref_count.load(Ordering::SeqCst);
    if count <= 0 {
      return Err(LuceneError::already_closed(
        "this IndexReader is closed".to_string(),
      ));
    }

    let rc = base.ref_count.fetch_sub(1, Ordering::SeqCst) - 1;
    if rc == 0 {
      base.closed.store(true, Ordering::SeqCst);
      let close_result = {
        let notify_result = self.notify_reader_closed_listeners();
        IOUtils::use_or_suppress_result(notify_result, self.report_close_to_parent_readers())
      };
      IOUtils::use_or_suppress_result(self.do_close(), close_result)?;
    } else if rc < 0 {
      return Err(LuceneError::illegal_state(format!(
        "too many decRef calls: refCount is {} after decrement",
        rc
      )));
    }

    Ok(())
  }

  /// Returns an error if this [`IndexReader`] or any of its child readers is
  /// closed; otherwise returns `Ok(())`.
  fn ensure_open(&self) -> Result<()> {
    let base = self.index_base();
    if base.ref_count.load(Ordering::SeqCst) <= 0 {
      return Err(LuceneError::already_closed(
        "this IndexReader is closed".to_string(),
      ));
    }

    // The "happens-before" rule on reading ref_count after a fake write
    // ensures visibility of closed_by_child state.
    if base.closed_by_child.load(Ordering::SeqCst) {
      return Err(LuceneError::already_closed(
        "this IndexReader cannot be used anymore as one of its child readers was closed"
          .to_string(),
      ));
    }

    Ok(())
  }

  type StoredFields: StoredFields;
  /// Returns a [`StoredFields`] reader for the stored fields of this index.
  ///
  /// This call never returns `None`, even if no stored fields were indexed. The
  /// returned instance should only be used by a single thread.
  fn stored_fields(&self) -> Result<Self::StoredFields>;

  /// Returns `true` if any documents have been deleted.
  ///
  /// Implementers should consider overriding this method if [`Self::max_doc`] or
  /// [`Self::num_docs`] are not constant-time operations.
  fn has_deletions(&self) -> Result<bool> {
    Ok(self.num_deleted_docs()? > 0)
  }

  /// Closes files associated with this index.
  ///
  /// No other methods should be called after this has been called.
  fn close(&self) -> Result<()> {
    let base = self.index_base();
    if !base.closed.load(Ordering::SeqCst) {
      self.dec_ref()?;
      base.closed.store(true, Ordering::SeqCst);
    }
    Ok(())
  }

  /// Implements close.
  fn do_close(&self) -> Result<()> {
    Ok(())
  }

  fn notify_reader_closed_listeners(&self) -> Result<()> {
    Ok(())
  }

  fn report_close_to_parent_readers(&self) -> Result<()> {
    // TODO IMPORTANT 未实现
    Ok(())
  }

  /// Cache helper type returned by [`Self::get_reader_cache_helper`].
  type ReaderCacheHelper: CacheHelper;

  /// Optional method: returns a [`CacheHelper`] that can be used to cache based
  /// on the content of this reader.
  ///
  /// Two readers that have different data or different sets of deleted
  /// documents will be considered different.
  ///
  /// A return value of `None` indicates that this reader is not suited for caching, which
  /// is typically the case for short-lived wrappers that alter the content of the wrapped reader.
  ///
  /// Experimental: this API follows the original Lucene experimental status.
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

  fn index_base(&self) -> &IndexReaderBase;

  /// Expert: increments the ref count only if this [`IndexReader`] has not been
  /// closed yet.
  ///
  /// Returns `true` if the ref count was successfully incremented, otherwise
  /// `false`. If this method returns `false`, the reader is either already
  /// closed or is currently being closed, and should not be used by an
  /// application.
  ///
  /// Ref counts are used to determine when a reader can be closed safely. Be
  /// sure to always call a corresponding [`Self::dec_ref`] when this method
  /// returns `true`.
  fn try_inc_ref(&self) -> bool {
    let base = self.index_base();
    loop {
      let count = base.ref_count.load(Ordering::SeqCst);
      if count <= 0 {
        return false;
      }

      match base
        .ref_count
        .compare_exchange(count, count + 1, Ordering::SeqCst, Ordering::SeqCst)
      {
        Ok(_) => return true,
        Err(_) => continue,
      }
    }
  }

  /// Expert: returns the current ref count for this reader.
  fn get_ref_count(&self) -> i32 {
    let base = self.index_base();
    base.ref_count.load(Ordering::SeqCst)
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

/// Utility hooks for building caches based on data contained in an index.
///
/// For example, a query-count cache can use a reader cache key to store the
/// number of documents that match a query per reader.
///
/// Experimental: this API follows the original Lucene experimental status.
pub trait CacheHelper {
  /// Gets a key that the resource can be cached on.
  ///
  /// The returned key can be compared by identity: equality is implemented as
  /// identity equality and hashing is implemented from the identity hash.
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

/// A cache key identifying a resource that is being cached on.
pub type CacheKey = Identity;

pub type IRTermVectors<LR, CR> =
  TermVectorsEnum2<<LR as IndexReader>::TermVectors, <CR as IndexReader>::TermVectors>;
pub type IRStoredFields<LR, CR> =
  StoredFieldsEnum2<<LR as IndexReader>::StoredFields, <CR as IndexReader>::StoredFields>;

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
  type TermVectors = IRTermVectors<LR, CR>;

  fn term_vectors(&self) -> Result<Self::TermVectors> {
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

  type StoredFields = IRStoredFields<LR, CR>;

  fn stored_fields(&self) -> Result<Self::StoredFields> {
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
      IndexReaderEnum::Leaf(leaf) => LeafReader::get_sum_doc_freq(leaf, field),
      IndexReaderEnum::Composite(comp) => comp.get_sum_doc_freq(field),
    }
  }

  fn get_doc_count(&self, field: &str) -> Result<i32> {
    match self {
      IndexReaderEnum::Leaf(leaf) => LeafReader::get_doc_count(leaf, field),
      IndexReaderEnum::Composite(comp) => comp.get_doc_count(field),
    }
  }

  fn get_sum_total_term_freq(&self, field: &str) -> Result<i64> {
    match self {
      IndexReaderEnum::Leaf(leaf) => LeafReader::get_sum_total_term_freq(leaf, field),
      IndexReaderEnum::Composite(comp) => comp.get_sum_total_term_freq(field),
    }
  }

  fn index_base(&self) -> &IndexReaderBase {
    match self {
      IndexReaderEnum::Leaf(leaf) => leaf.index_base(),
      IndexReaderEnum::Composite(comp) => comp.index_base(),
    }
  }
}
impl<IR> IndexReader for &IR
where
  IR: IndexReader,
{
  type TermVectors = IR::TermVectors;

  fn term_vectors(&self) -> Result<Self::TermVectors> {
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

  type StoredFields = IR::StoredFields;

  fn stored_fields(&self) -> Result<Self::StoredFields> {
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

  fn index_base(&self) -> &IndexReaderBase {
    (**self).index_base()
  }
}
impl<IR> IndexReader for Arc<IR>
where
  IR: IndexReader,
{
  type TermVectors = IR::TermVectors;

  fn term_vectors(&self) -> Result<Self::TermVectors> {
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

  type StoredFields = IR::StoredFields;

  fn stored_fields(&self) -> Result<Self::StoredFields> {
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

  fn index_base(&self) -> &IndexReaderBase {
    (**self).index_base()
  }
}
impl<IR> IndexReader for Rc<IR>
where
  IR: IndexReader,
{
  type TermVectors = IR::TermVectors;

  fn term_vectors(&self) -> Result<Self::TermVectors> {
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

  type StoredFields = IR::StoredFields;

  fn stored_fields(&self) -> Result<Self::StoredFields> {
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

  fn index_base(&self) -> &IndexReaderBase {
    (**self).index_base()
  }
}
/// A lightweight identity marker used to distinguish instances by identity,
/// not by value.
///
/// This type itself carries no semantic data. Its sole purpose is to provide
/// a stable allocation whose address can be used as an identity token.
#[derive(Debug)]
struct IdentityTag(u8);
/// An identity wrapper whose equality and hashing are based on pointer identity.
///
/// Two `Identity` values are considered equal **if and only if** they point to
/// the same underlying allocation (i.e. they represent the same instance),
/// regardless of any external semantics.
///
/// This is commonly used to model Lucene-style *instance identity*:
/// - whether two objects originate from the same underlying reader
/// - whether two wrappers refer to the same logical component
/// - whether two enum variants wrap the same inner instance
#[derive(Debug, Clone)]
pub struct Identity(Arc<IdentityTag>);

impl Identity {
  pub fn new() -> Self {
    Identity(Arc::new(IdentityTag(0)))
  }
  /// Returns the raw pointer to the underlying identity allocation.
  ///
  /// This pointer is used exclusively for identity comparison and hashing.
  /// Its value must never be dereferenced.
  #[inline]
  fn ptr(&self) -> *const IdentityTag {
    Arc::as_ptr(&self.0)
  }
}
impl Default for Identity {
  fn default() -> Self {
    Self::new()
  }
}

impl Accountable for Identity {
  fn ram_bytes_used(&self) -> Result<i64> {
    Ok(std::mem::size_of_val(self.0.as_ref()) as i64)
  }
}

impl PartialEq for Identity {
  fn eq(&self, other: &Self) -> bool {
    std::ptr::eq(self.ptr(), other.ptr())
  }
}
impl Eq for Identity {}

impl Hash for Identity {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    (self.ptr() as usize).hash(state);
  }
}

macro_rules! either_index_reader_type {
    (term_vectors; $A:ident, $B:ident) => {
        TermVectorsEnum2<<$A as IndexReader>::TermVectors, <$B as IndexReader>::TermVectors>
    };
    (term_vectors; $A:ident, $B:ident, $C:ident) => {
        TermVectorsEnum2<
            <$A as IndexReader>::TermVectors,
            TermVectorsEnum2<<$B as IndexReader>::TermVectors, <$C as IndexReader>::TermVectors>,
        >
    };
    (stored_fields; $A:ident, $B:ident) => {
        StoredFieldsEnum2<<$A as IndexReader>::StoredFields, <$B as IndexReader>::StoredFields>
    };
    (stored_fields; $A:ident, $B:ident, $C:ident) => {
        StoredFieldsEnum2<
            <$A as IndexReader>::StoredFields,
            StoredFieldsEnum2<<$B as IndexReader>::StoredFields, <$C as IndexReader>::StoredFields>,
        >
    };
    (cache_helper; $A:ident, $B:ident) => {
        CacheHelperEnum2<<$A as IndexReader>::ReaderCacheHelper, <$B as IndexReader>::ReaderCacheHelper>
    };
    (cache_helper; $A:ident, $B:ident, $C:ident) => {
        CacheHelperEnum2<
            <$A as IndexReader>::ReaderCacheHelper,
            CacheHelperEnum2<
                <$B as IndexReader>::ReaderCacheHelper,
                <$C as IndexReader>::ReaderCacheHelper,
            >,
        >
    };
}

macro_rules! either_index_reader_wrap {
  (TermVectorsEnum2; $expr:expr; A; [A: $A:ident, B: $B:ident]) => {
    TermVectorsEnum2::A($expr)
  };
  (TermVectorsEnum2; $expr:expr; B; [A: $A:ident, B: $B:ident]) => {
    TermVectorsEnum2::B($expr)
  };
  (TermVectorsEnum2; $expr:expr; A; [A: $A:ident, B: $B:ident, C: $C:ident]) => {
    TermVectorsEnum2::A($expr)
  };
  (TermVectorsEnum2; $expr:expr; B; [A: $A:ident, B: $B:ident, C: $C:ident]) => {
    TermVectorsEnum2::B(TermVectorsEnum2::A($expr))
  };
  (TermVectorsEnum2; $expr:expr; C; [A: $A:ident, B: $B:ident, C: $C:ident]) => {
    TermVectorsEnum2::B(TermVectorsEnum2::B($expr))
  };
  (StoredFieldsEnum2; $expr:expr; A; [A: $A:ident, B: $B:ident]) => {
    StoredFieldsEnum2::A($expr)
  };
  (StoredFieldsEnum2; $expr:expr; B; [A: $A:ident, B: $B:ident]) => {
    StoredFieldsEnum2::B($expr)
  };
  (StoredFieldsEnum2; $expr:expr; A; [A: $A:ident, B: $B:ident, C: $C:ident]) => {
    StoredFieldsEnum2::A($expr)
  };
  (StoredFieldsEnum2; $expr:expr; B; [A: $A:ident, B: $B:ident, C: $C:ident]) => {
    StoredFieldsEnum2::B(StoredFieldsEnum2::A($expr))
  };
  (StoredFieldsEnum2; $expr:expr; C; [A: $A:ident, B: $B:ident, C: $C:ident]) => {
    StoredFieldsEnum2::B(StoredFieldsEnum2::B($expr))
  };
  (CacheHelperEnum2; $expr:expr; A; [A: $A:ident, B: $B:ident]) => {
    CacheHelperEnum2::A($expr)
  };
  (CacheHelperEnum2; $expr:expr; B; [A: $A:ident, B: $B:ident]) => {
    CacheHelperEnum2::B($expr)
  };
  (CacheHelperEnum2; $expr:expr; A; [A: $A:ident, B: $B:ident, C: $C:ident]) => {
    CacheHelperEnum2::A($expr)
  };
  (CacheHelperEnum2; $expr:expr; B; [A: $A:ident, B: $B:ident, C: $C:ident]) => {
    CacheHelperEnum2::B(CacheHelperEnum2::A($expr))
  };
  (CacheHelperEnum2; $expr:expr; C; [A: $A:ident, B: $B:ident, C: $C:ident]) => {
    CacheHelperEnum2::B(CacheHelperEnum2::B($expr))
  };
}

macro_rules! either_index_reader {
    (
        $vis:vis $name:ident { $( $Variant:ident : $T:ident ),+ $(,)? }
    ) => {
        either_index_reader!(@impl $vis $name { $( $Variant : $T ),+ } [ $( $Variant : $T ),+ ]);
    };
    (
        @impl
        $vis:vis $name:ident
        { $( $Variant:ident : $T:ident ),+ }
        $all:tt
    ) => {
        $vis enum $name<$( $T ),+> {
            $( $Variant($T), )+
        }

        impl<$( $T ),+> Display for $name<$( $T ),+>
        where
            $( $T: IndexReader ),+
        {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                match self {
                    $( Self::$Variant(inner) => write!(f, "{}", inner), )+
                }
            }
        }

        impl<$( $T ),+> IndexReader for $name<$( $T ),+>
        where
            $( $T: IndexReader ),+
        {
            type TermVectors = either_index_reader_type!(term_vectors; $( $T ),+);

            fn term_vectors(&self) -> Result<Self::TermVectors> {
                match self {
                    $(
                        Self::$Variant(inner) => {
                            Ok(either_index_reader_wrap!(
                                TermVectorsEnum2;
                                inner.term_vectors()?;
                                $Variant;
                                $all
                            ))
                        }
                    ),+
                }
            }

            fn max_doc(&self) -> Result<i32> {
                match self {
                    $( Self::$Variant(inner) => inner.max_doc(), )+
                }
            }

            fn num_docs(&self) -> Result<i32> {
                match self {
                    $( Self::$Variant(inner) => inner.num_docs(), )+
                }
            }

            fn num_deleted_docs(&self) -> Result<i32> {
                match self {
                    $( Self::$Variant(inner) => inner.num_deleted_docs(), )+
                }
            }

            fn inc_ref(&self) -> Result<()> {
                match self {
                    $( Self::$Variant(inner) => inner.inc_ref(), )+
                }
            }

            fn dec_ref(&self) -> Result<()> {
                match self {
                    $( Self::$Variant(inner) => inner.dec_ref(), )+
                }
            }

            fn ensure_open(&self) -> Result<()> {
                match self {
                    $( Self::$Variant(inner) => inner.ensure_open(), )+
                }
            }

            type StoredFields = either_index_reader_type!(stored_fields; $( $T ),+);

            fn stored_fields(&self) -> Result<Self::StoredFields> {
                match self {
                    $(
                        Self::$Variant(inner) => {
                            Ok(either_index_reader_wrap!(
                                StoredFieldsEnum2;
                                inner.stored_fields()?;
                                $Variant;
                                $all
                            ))
                        }
                    ),+
                }
            }

            fn has_deletions(&self) -> Result<bool> {
                match self {
                    $( Self::$Variant(inner) => inner.has_deletions(), )+
                }
            }

            fn do_close(&self) -> Result<()> {
                match self {
                    $( Self::$Variant(inner) => inner.do_close(), )+
                }
            }

            type ReaderCacheHelper = either_index_reader_type!(cache_helper; $( $T ),+);

            fn get_reader_cache_helper(&self) -> Result<Option<Self::ReaderCacheHelper>> {
                match self {
                    $(
                        Self::$Variant(inner) => Ok(inner.get_reader_cache_helper()?.map(|helper| {
                            either_index_reader_wrap!(
                                CacheHelperEnum2;
                                helper;
                                $Variant;
                                $all
                            )
                        })),
                    )+
                }
            }

            fn doc_freq(&self, term: &Term) -> Result<i32> {
                match self {
                    $( Self::$Variant(inner) => inner.doc_freq(term), )+
                }
            }

            fn total_term_freq(&self, term: &Term) -> Result<i64> {
                match self {
                    $( Self::$Variant(inner) => inner.total_term_freq(term), )+
                }
            }

            fn get_sum_doc_freq(&self, field: &str) -> Result<i64> {
                match self {
                    $( Self::$Variant(inner) => inner.get_sum_doc_freq(field), )+
                }
            }

            fn get_doc_count(&self, field: &str) -> Result<i32> {
                match self {
                    $( Self::$Variant(inner) => inner.get_doc_count(field), )+
                }
            }

            fn get_sum_total_term_freq(&self, field: &str) -> Result<i64> {
                match self {
                    $( Self::$Variant(inner) => inner.get_sum_total_term_freq(field), )+
                }
            }

            fn index_base(&self) -> &IndexReaderBase {
                match self {
                    $( Self::$Variant(inner) => inner.index_base(), )+
                }
            }
        }
    };
}

either_index_reader!(pub IndexReaderEnum2 { A: A, B: B });
either_index_reader!(pub IndexReaderEnum3 { A: A, B: B, C: C });
