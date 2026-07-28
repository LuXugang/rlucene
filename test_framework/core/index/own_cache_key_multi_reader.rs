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
  BCRStoredFieldsImpl, BCRTermVectorsImpl, BaseCompositeReader,
};
use crate::core::index::composite_reader::CompositeReader;
use crate::core::index::index_reader::{
  CacheHelper, CacheKey, ClosedListener, ClosedListenerList, CompositeReaderContextKind,
  IndexReader, IndexReaderBase,
};
use crate::core::index::multi_reader::{MultiReader, MultiReaderKind};
use crate::core::index::term::Term;
use crate::core::util::IOUtils;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use parking_lot::Mutex;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

/// A [`MultiReader`] that has its own cache key, occasionally useful for testing purposes.
pub struct OwnCacheKeyMultiReader<R>
where
  R: IndexReader,
{
  base: MultiReader<R>,
  reader_closed_listeners: ClosedListenerList,
  cache_helper: CacheHelperImpl,
}

#[derive(Clone)]
pub struct CacheHelperImpl {
  cache_key: CacheKey,
  reader_closed_listeners: ClosedListenerList,
}

impl CacheHelperImpl {
  fn new(reader_closed_listeners: ClosedListenerList) -> Self {
    Self {
      cache_key: CacheKey::new(),
      reader_closed_listeners,
    }
  }
}

impl CacheHelper for CacheHelperImpl {
  fn get_key(&self) -> CacheKey {
    self.cache_key.clone()
  }

  fn add_closed_listener(&self, listener: Arc<dyn ClosedListener>) -> Result<()> {
    let mut reader_closed_listeners = self.reader_closed_listeners.lock();
    let Some(reader_closed_listeners) = reader_closed_listeners.as_mut() else {
      return Err(LuceneError::already_closed(
        "this IndexReader is closed".to_string(),
      ));
    };
    if !reader_closed_listeners
      .iter()
      .any(|existing| Arc::ptr_eq(existing, &listener))
    {
      reader_closed_listeners.push(listener);
    }
    Ok(())
  }
}

impl<R> OwnCacheKeyMultiReader<R>
where
  R: IndexReader,
  R::ContextKind: MultiReaderKind<R>,
{
  /// Sole constructor.
  pub fn new(sub_readers: Vec<R>) -> Result<Self> {
    let base = MultiReader::new(sub_readers)?;
    let reader_closed_listeners = Arc::new(Mutex::new(Some(Vec::new())));
    let cache_helper = CacheHelperImpl::new(reader_closed_listeners.clone());
    Ok(Self {
      base,
      reader_closed_listeners,
      cache_helper,
    })
  }
}

impl<R> IndexReader for OwnCacheKeyMultiReader<R>
where
  R: IndexReader,
  R::ContextKind: MultiReaderKind<R>,
{
  type ContextKind = CompositeReaderContextKind;

  type TermVectors = BCRTermVectorsImpl<R>;

  fn term_vectors(&self) -> Result<Self::TermVectors> {
    self.base.term_vectors()
  }

  fn max_doc(&self) -> Result<i32> {
    self.base.max_doc()
  }

  fn num_docs(&self) -> Result<i32> {
    self.base.num_docs()
  }

  type StoredFields = BCRStoredFieldsImpl<R>;

  fn stored_fields(&self) -> Result<Self::StoredFields> {
    self.base.stored_fields()
  }

  fn do_close(&self) -> Result<()> {
    <MultiReader<R> as IndexReader>::do_close(&self.base)
  }

  type ReaderCacheHelper = CacheHelperImpl;

  fn get_reader_cache_helper(&self) -> Result<Option<Self::ReaderCacheHelper>> {
    Ok(Some(self.cache_helper.clone()))
  }

  fn notify_reader_closed_listeners(&self) -> Result<()> {
    let mut reader_closed_listeners = self.reader_closed_listeners.lock();
    let listeners = reader_closed_listeners.take().unwrap_or_default();
    IOUtils::apply_to_all(&listeners, |listener| {
      listener.on_close(&self.cache_helper.get_key())
    })
  }

  fn doc_freq(&self, term: &Term) -> Result<i32> {
    self.base.doc_freq(term)
  }

  fn total_term_freq(&self, term: &Term) -> Result<i64> {
    self.base.total_term_freq(term)
  }

  fn get_sum_doc_freq(&self, field: &str) -> Result<i64> {
    self.base.get_sum_doc_freq(field)
  }

  fn get_doc_count(&self, field: &str) -> Result<i32> {
    self.base.get_doc_count(field)
  }

  fn get_sum_total_term_freq(&self, field: &str) -> Result<i64> {
    self.base.get_sum_total_term_freq(field)
  }

  fn index_base(&self) -> &IndexReaderBase {
    self.base.index_base()
  }
}

impl<R> Display for OwnCacheKeyMultiReader<R>
where
  R: IndexReader,
  R::ContextKind: MultiReaderKind<R>,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

impl<R> CompositeReader for OwnCacheKeyMultiReader<R>
where
  R: IndexReader,
  R::ContextKind: MultiReaderKind<R>,
{
  type LeafReader = <R::ContextKind as MultiReaderKind<R>>::LeafReader;
  type SubReader = R;

  fn get_sequential_sub_readers(&self) -> &[Self::SubReader] {
    self.base.get_sequential_sub_readers()
  }

  fn visit_leaves<F>(&self, visitor: &mut F) -> Result<()>
  where
    F: FnMut(&Self::LeafReader) -> Result<()>,
  {
    self.base.visit_leaves(visitor)
  }
}

impl<R> BaseCompositeReader for OwnCacheKeyMultiReader<R>
where
  R: IndexReader,
  R::ContextKind: MultiReaderKind<R>,
{
}
