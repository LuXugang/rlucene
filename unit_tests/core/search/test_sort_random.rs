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
use crate::core::document::document::Document;
use crate::core::document::numeric_doc_values_field::NumericDocValuesField;
use crate::core::document::sorted_doc_values_field::SortedDocValuesField;
use crate::core::document::stored_field::StoredField;
use crate::core::index::BytesRef;
use crate::core::index::index_reader::Identity;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::search::constant_score_scorer::ConstantScoreScorer;
use crate::core::search::constant_score_weight::ConstantScoreWeight;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::explanation::Explanation;
use crate::core::search::field_comparator::FieldComparatorValue;
use crate::core::search::field_value_hit_queue::TopFieldScoreDoc;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::matches_utils::MatchWithNoTerms;
use crate::core::search::query::{Query, QueryBase, QueryWeight, QueryWeightSs};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::sort::Sort;
use crate::core::search::sort_field::{MissingValueEnum, SortField, SortFieldType, SortFiledBase};
use crate::core::search::top_docs::TopDocsLike;
use crate::core::search::weight::{DefaultScorerSupplier, Weight};
use crate::core::util::HasIdentity;
use crate::core::util::bit_set::BitSet;
use crate::core::util::bit_set_iterator::BitSetIterator;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::test::support::core::index::random_index_writer::RandomIndexWriter;
pub use crate::test::support::core::search::query::RandomQuery;
use crate::test::support::core::util::lucene_test_case::{
  at_least, new_directory_shared, new_searcher_with_wrap, random,
};
use crate::test::support::core::util::test_util::TestUtil;
use parking_lot::Mutex;
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

#[allow(dead_code)] // for quick search
pub struct TestSortRandom;

#[test]
fn test_random_string_sort() -> Result<()> {
  test_random_string_sort_for_type(SortFieldType::String)
}

fn test_random_string_sort_for_type(type_: SortFieldType) -> Result<()> {
  let mut random = random();
  let num_docs = at_least(&mut random, 100) as usize;
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  let allow_dups = random.random_bool(0.5);
  let mut seen = HashSet::new();
  let max_length = TestUtil::next_int(&mut random, 5, 100) as usize;

  let mut num_docs_indexed = 0usize;
  let mut doc_values: Vec<Option<BytesRef<Vec<u8>>>> = Vec::with_capacity(num_docs);
  while num_docs_indexed < num_docs {
    let mut doc = Document::new();

    let br = if random.random_range(0..10) != 7 {
      let s = if random.random_bool(0.5) {
        TestUtil::random_simple_string_with_len(&mut random, max_length)
      } else {
        TestUtil::random_unicode_string_with_len(&mut random, max_length)
      };

      if !allow_dups && !seen.insert(s.clone()) {
        continue;
      }

      let br = BytesRef::from_string(&s);
      doc.add(SortedDocValuesField::new("stringdv", br.clone()));
      Some(br)
    } else {
      None
    };

    doc_values.push(br.clone());
    doc.add(NumericDocValuesField::new("id", num_docs_indexed as i64));
    doc.add(StoredField::from_i32("id", num_docs_indexed as i32)?);
    writer.add_document(&mut random, doc)?;
    num_docs_indexed += 1;

    if random.random_range(0..40) == 17 {
      drop(writer.get_reader(&mut random)?);
    }
  }

  let reader = writer.get_reader(&mut random)?;
  let max_doc = reader.max_doc()?;
  writer.close(&mut random)?;

  let searcher = new_searcher_with_wrap(&mut random, reader, false)?;
  let iters = at_least(&mut random, 100);
  let doc_values = Arc::new(doc_values);

  for _iter in 0..iters {
    let reverse = random.random_bool(0.5);
    let mut sf = SortField::with_reverse(Some("stringdv"), type_, reverse)?;
    let sort_missing_last = random.random_bool(0.5);
    if sort_missing_last {
      sf.set_missing_value(MissingValueEnum::StringLast)?;
    }

    let sort = if random.random_bool(0.5) {
      Sort::with_fields(vec![sf])?
    } else {
      Sort::with_fields(vec![sf, SortField::get_field_doc()?])?
    };

    let hit_count = TestUtil::next_int(&mut random, 1, max_doc + 20) as usize;
    let seed = random.random();
    let density = random.random::<f32>();
    let filter = RandomQuery::new(seed, density, doc_values.clone());
    let hits = searcher.search_with_sort_score(filter.clone(), hit_count, sort, false)?;

    let mut expected = filter.match_values.lock().clone();
    expected.sort_by(|a, b| compare_optional_bytes_ref(a.as_ref(), b.as_ref(), sort_missing_last));
    if reverse {
      expected.reverse();
    }

    assert_eq!(hits.total_hits().value, expected.len());
    for (hit_idx, score_doc) in hits.score_docs().iter().enumerate() {
      let fd = match score_doc {
        TopFieldScoreDoc::Field(fd) => fd,
        _ => {
          return Err(LuceneError::illegal_state(
            "expected FieldDoc in TopFieldDocs",
          ));
        },
      };
      let actual = field_doc_sort_value(fd.fields.first())?;
      let expected_value = expected.get(hit_idx).cloned().unwrap();
      assert_eq!(expected_value, actual);
    }
  }

  Ok(())
}

fn compare_optional_bytes_ref(
  a: Option<&BytesRef<Vec<u8>>>,
  b: Option<&BytesRef<Vec<u8>>>,
  sort_missing_last: bool,
) -> Ordering {
  match (a, b) {
    (None, None) => Ordering::Equal,
    (None, Some(_)) => {
      if sort_missing_last {
        Ordering::Greater
      } else {
        Ordering::Less
      }
    },
    (Some(_), None) => {
      if sort_missing_last {
        Ordering::Less
      } else {
        Ordering::Greater
      }
    },
    (Some(a), Some(b)) => a.cmp(b),
  }
}

fn field_doc_sort_value(value: Option<&FieldComparatorValue>) -> Result<Option<BytesRef<Vec<u8>>>> {
  match value {
    Some(FieldComparatorValue::Missing) | None => Ok(None),
    Some(FieldComparatorValue::TermVal(bytes)) => Ok(Some(BytesRef::deep_copy_of(bytes))),
    Some(other) => Err(LuceneError::illegal_state(format!(
      "expected string sort value, got {other:?}"
    ))),
  }
}
