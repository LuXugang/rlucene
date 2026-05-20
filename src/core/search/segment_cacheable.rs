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
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::util::error::lucene_error::Result;
use std::sync::Arc;

/// Interface defining whether or not an object can be cached against a `LeafReader`
///
/// Objects that depend only on segment-immutable structures such as Points or postings lists can
/// just return `Ok(true)` from [`SegmentCacheable::is_cacheable`].
///
/// Objects that depend on doc values should return
/// [`DocValues::is_cacheable`](crate::core::index::doc_values::DocValues::is_cacheable), which will check to see if the doc values
/// fields have been updated. Updated doc values fields are not suitable for cacheing.
///
/// Objects that are not segment-immutable, such as those that rely on global statistics or
/// scores, should return `false`.
pub trait SegmentCacheable<IRC: IndexReaderContext> {
  /// Returns `Ok(true)` if the object can be cached against a given leaf.
  fn is_cacheable(&self, ctx: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<bool>;
}

impl<IRC, T> SegmentCacheable<IRC> for Arc<T>
where
  IRC: IndexReaderContext,
  T: SegmentCacheable<IRC>,
{
  fn is_cacheable(&self, ctx: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<bool> {
    self.as_ref().is_cacheable(ctx)
  }
}
impl<IRC, T> SegmentCacheable<IRC> for Box<T>
where
  IRC: IndexReaderContext,
  T: SegmentCacheable<IRC> + ?Sized,
{
  fn is_cacheable(&self, ctx: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<bool> {
    self.as_ref().is_cacheable(ctx)
  }
}
#[cfg(test)]
mod tests {
  use crate::core::document::document::Document;
  use crate::core::document::field::Store;
  use crate::core::document::numeric_doc_values_field::NumericDocValuesField;
  use crate::core::index::composite_reader::get_context;
  use crate::core::index::directory_reader;
  use crate::core::index::doc_values::DocValues;
  use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
  use crate::core::index::index_writer::IndexWriter;
  use crate::core::index::leaf_reader::LeafReader;
  use crate::core::index::leaf_reader_context::LeafReaderContext;
  use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
  use crate::core::index::no_merge_policy::NoMergePolicy;
  use crate::core::index::term::Term;
  use crate::core::index::two_phase_commit::TwoPhaseCommit;
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
    new_directory_shared, new_index_writer_config, new_text_field, random,
  };
  use std::collections::HashMap;
  use std::rc::Rc;

  use super::SegmentCacheable;

  #[allow(dead_code)] // for quick search
  struct TestSegmentCacheables;

  enum TestCacheable {
    Fixed(bool),
    DocValues(Vec<String>),
    All(Vec<Rc<TestCacheable>>),
  }

  impl TestCacheable {
    fn fixed(cacheable: bool) -> Rc<Self> {
      Rc::new(Self::Fixed(cacheable))
    }

    fn doc_values(names: &[&str]) -> Rc<Self> {
      Rc::new(Self::DocValues(fields(names)))
    }

    fn all(children: Vec<Rc<Self>>) -> Rc<Self> {
      Rc::new(Self::All(children))
    }

    fn is_cacheable<LR>(&self, ctx: &LeafReaderContext<LR>) -> Result<bool>
    where
      LR: LeafReader,
    {
      match self {
        Self::Fixed(cacheable) => Ok(*cacheable),
        Self::DocValues(fields) => DocValues::is_cacheable(ctx, fields),
        Self::All(children) => {
          for child in children {
            if !child.is_cacheable(ctx)? {
              return Ok(false);
            }
          }
          Ok(true)
        },
      }
    }
  }

  impl<IRC> SegmentCacheable<IRC> for TestCacheable
  where
    IRC: IndexReaderContext,
  {
    fn is_cacheable(
      &self,
      ctx: &crate::core::index::leaf_reader_context::LeafReaderContext<IRCLeafReader<IRC>>,
    ) -> Result<bool> {
      TestCacheable::is_cacheable(self, ctx)
    }
  }

  fn fields(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| name.to_string()).collect()
  }

  fn is_cacheable<LR>(cacheable: &Rc<TestCacheable>, ctx: &LeafReaderContext<LR>) -> Result<bool>
  where
    LR: LeafReader,
  {
    cacheable.as_ref().is_cacheable(ctx)
  }

  #[test]
  fn test_multiple_doc_values_delegates() -> Result<()> {
    let seg = TestCacheable::fixed(true);
    let non = TestCacheable::fixed(false);
    let dv1 = TestCacheable::doc_values(&["field1"]);
    let dv2 = TestCacheable::doc_values(&["field2"]);
    let dv3 = TestCacheable::doc_values(&["field3"]);
    let dv34 = TestCacheable::doc_values(&["field3", "field4"]);
    let dv12 = TestCacheable::doc_values(&["field1", "field2"]);

    let seg_dv1 = TestCacheable::all(vec![seg.clone(), dv1.clone()]);
    let dv2_dv34 = TestCacheable::all(vec![dv2.clone(), dv34.clone()]);
    let dv2_non = TestCacheable::all(vec![dv2.clone(), non.clone()]);

    let seg_dv1_dv2_dv34 = TestCacheable::all(vec![seg_dv1.clone(), dv2_dv34.clone()]);

    let dv1_dv3 = TestCacheable::all(vec![dv1.clone(), dv3.clone()]);
    let dv12_dv1_dv3 = TestCacheable::all(vec![dv12.clone(), dv1_dv3.clone()]);

    let mut random = random();
    let dir = new_directory_shared(&mut random)?;
    let mut iwc = new_index_writer_config(&mut random);
    iwc.set_merge_policy(NoMergePolicy::default());
    let writer = IndexWriter::new(dir.clone(), iwc)?;

    let mut doc = Document::new();
    doc.add(NumericDocValuesField::new("field3", 1));
    doc.add(new_text_field(
      &mut random,
      "text",
      "text",
      Store::No,
      &mut HashMap::new(),
    )?);
    writer.add_document(doc)?;
    writer.commit()?;

    let reader = directory_reader::open_from_writer(&writer)?;
    let reader_context = get_context(reader)?;
    let ctx = &reader_context.leaves()?[0];

    assert!(is_cacheable(&seg_dv1, ctx)?);
    assert!(is_cacheable(&dv2_dv34, ctx)?);
    assert!(is_cacheable(&seg_dv1_dv2_dv34, ctx)?);
    assert!(!is_cacheable(&dv2_non, ctx)?);

    writer.update_numeric_doc_value(Term::from_text("text", "text"), "field3", 2)?;
    writer.commit()?;
    drop(reader_context);
    let reader = directory_reader::open(dir.clone())?;

    let reader_context = get_context(reader)?;
    let ctx = &reader_context.leaves()?[0];
    assert!(is_cacheable(&seg_dv1, ctx)?);
    assert!(!is_cacheable(&dv34, ctx)?);
    assert!(!is_cacheable(&dv2_dv34, ctx)?);
    assert!(!is_cacheable(&dv1_dv3, ctx)?);
    assert!(!is_cacheable(&seg_dv1_dv2_dv34, ctx)?);
    assert!(!is_cacheable(&dv12_dv1_dv3, ctx)?);

    drop(reader_context);
    writer.close()?;
    Ok(())
  }
}
