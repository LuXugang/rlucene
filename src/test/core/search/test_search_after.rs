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
use crate::core::document::binary_doc_values_field::BinaryDocValuesField;
use crate::core::document::document::Document;
use crate::core::document::double_doc_values_field::DoubleDocValuesField;
use crate::core::document::field::Store;
use crate::core::document::fields::Fields;
use crate::core::document::float_doc_values_field::FloatDocValuesField;
use crate::core::document::numeric_doc_values_field::NumericDocValuesField;
use crate::core::document::sorted_doc_values_field::SortedDocValuesField;
use crate::core::document::stored_field::StoredField;
use crate::core::index::BytesRef;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::stored_fields::StoredFields;
use crate::core::index::term::Term;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder;
use crate::core::search::field_doc::FieldDoc;
use crate::core::search::field_value_hit_queue::TopFieldScoreDoc;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::match_all_docs_query::MatchAllDocsQuery;
use crate::core::search::query::Query;
use crate::core::search::score_doc::{ScoreDoc, ScoreDocLike};
use crate::core::search::sort::Sort;
use crate::core::search::sort_field::MissingValueEnum::{StringFirst, StringLast};
use crate::core::search::sort_field::{MissingValueEnum, SortField, SortFieldType, SortFiledBase};
use crate::core::search::term_query::TermQuery;
use crate::core::search::top_docs::{TopDocs, TopDocsLike};
use crate::core::search::top_field_collector::populate_scores;
use crate::core::search::top_field_collector_manager::TopFieldCollectorManager;
use crate::core::search::top_field_docs::TopFieldDocs;
use crate::core::search::top_score_doc_collector_manager::TopScoreDocCollectorManager;
use crate::core::search::total_hits::TotalHits;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::DefaultIndexSearchCR;
use crate::test::core::util::english::English;
use crate::test::core::util::lucene_test_case::{
  at_least, new_directory_shared, new_searcher_with_reader, new_text_field, random,
};
use crate::test::core::util::test_util::TestUtil;
use rand::{Rng, RngExt};
use std::collections::HashMap;

#[allow(dead_code)] // for quick search
pub struct TestSearchAfter;

fn set_up<R>(random: &mut R) -> Result<(Vec<SortField>, DefaultIndexSearchCR)>
where
  R: Rng + ?Sized,
{
  let mut all_sort_fields = vec![
    SortField::with_reverse(Some("int"), SortFieldType::Int, false)?,
    SortField::with_reverse(Some("long"), SortFieldType::Long, false)?,
    SortField::with_reverse(Some("float"), SortFieldType::Float, false)?,
    SortField::with_reverse(Some("double"), SortFieldType::Double, false)?,
    SortField::with_reverse(Some("bytes"), SortFieldType::String, false)?,
    SortField::with_reverse(Some("bytesval"), SortFieldType::StringVal, false)?,
    SortField::with_reverse(Some("int"), SortFieldType::Int, true)?,
    SortField::with_reverse(Some("long"), SortFieldType::Long, true)?,
    SortField::with_reverse(Some("float"), SortFieldType::Float, true)?,
    SortField::with_reverse(Some("double"), SortFieldType::Double, true)?,
    SortField::with_reverse(Some("bytes"), SortFieldType::String, true)?,
    SortField::with_reverse(Some("bytesval"), SortFieldType::StringVal, true)?,
    SortField::get_field_score()?,
    SortField::get_field_doc()?,
  ];

  for field in ["bytes", "sortedbytesdocvalues"] {
    for rev in 0..2 {
      let reversed = rev == 0;

      let mut sf = SortField::with_reverse(Some(field), SortFieldType::String, reversed)?;
      sf.set_missing_value(StringFirst)?;
      all_sort_fields.push(sf);

      let mut sf = SortField::with_reverse(Some(field), SortFieldType::String, reversed)?;
      sf.set_missing_value(StringLast)?;
      all_sort_fields.push(sf);
    }
  }

  for field in ["sortedbytesdocvaluesval", "straightbytesdocvalues"] {
    for rev in 0..2 {
      let reversed = rev == 0;

      let mut sf = SortField::with_reverse(Some(field), SortFieldType::StringVal, reversed)?;
      sf.set_missing_value(StringFirst)?;
      all_sort_fields.push(sf);

      let mut sf = SortField::with_reverse(Some(field), SortFieldType::StringVal, reversed)?;
      sf.set_missing_value(StringLast)?;
      all_sort_fields.push(sf);
    }
  }

  let limit = all_sort_fields.len();
  for i in 0..limit {
    let sf = &all_sort_fields[i];
    if sf.get_type() == SortFieldType::Int {
      let mut sf2 = SortField::with_reverse(sf.get_field(), SortFieldType::Int, sf.get_reverse())?;
      sf2.set_missing_value(MissingValueEnum::Int(random.random::<i32>()))?;
      all_sort_fields.push(sf2);
    } else if sf.get_type() == SortFieldType::Long {
      let mut sf2 = SortField::with_reverse(sf.get_field(), SortFieldType::Long, sf.get_reverse())?;
      sf2.set_missing_value(MissingValueEnum::Long(random.random::<i64>()))?;
      all_sort_fields.push(sf2);
    } else if sf.get_type() == SortFieldType::Float {
      let mut sf2 =
        SortField::with_reverse(sf.get_field(), SortFieldType::Float, sf.get_reverse())?;
      sf2.set_missing_value(MissingValueEnum::Float(random.random::<f32>()))?;
      all_sort_fields.push(sf2);
    } else if sf.get_type() == SortFieldType::Double {
      let mut sf2 =
        SortField::with_reverse(sf.get_field(), SortFieldType::Double, sf.get_reverse())?;
      sf2.set_missing_value(MissingValueEnum::Double(random.random::<f64>()))?;
      all_sort_fields.push(sf2);
    }
  }

  let dir = new_directory_shared(random)?;
  let iw = RandomIndexWriter::new(random, dir.clone());
  let mut field_to_type = HashMap::new();

  let num_docs = at_least(random, 200);
  for i in 0..num_docs {
    let mut fields: Vec<Fields> = Vec::new();
    fields.push(
      new_text_field(
        random,
        "english",
        English::int_to_english(i),
        Store::No,
        &mut field_to_type,
      )?
      .into(),
    );
    fields.push(
      new_text_field(
        random,
        "oddeven",
        if i % 2 == 0 { "even" } else { "odd" },
        Store::No,
        &mut field_to_type,
      )?
      .into(),
    );
    fields.push(NumericDocValuesField::new("byte", random.random::<i8>() as i64).into());
    fields.push(NumericDocValuesField::new("short", random.random::<i16>() as i64).into());
    fields.push(NumericDocValuesField::new("int", random.random::<i32>() as i64).into());
    fields.push(NumericDocValuesField::new("long", random.random::<i64>()).into());
    fields.push(FloatDocValuesField::new("float", random.random::<f32>()).into());
    fields.push(DoubleDocValuesField::new("double", random.random::<f64>()).into());

    let bytes_value = TestUtil::random_realistic_unicode_string(random);
    fields.push(SortedDocValuesField::new("bytes", BytesRef::from_string(&bytes_value)).into());

    let bytesval_value = TestUtil::random_realistic_unicode_string(random);
    fields
      .push(BinaryDocValuesField::new("bytesval", BytesRef::from_string(&bytesval_value)).into());

    let mut document = Document::new();
    document.add(StoredField::from_string("id", i.to_string())?);

    if cfg!(feature = "test_log_verbose") {
      println!("  add doc id={}", i);
    }

    for field in fields {
      if random.random_range(0..5) != 4 {
        document.add(field);
      }
    }

    iw.add_document(random, document)?;

    if random.random_range(0..50) == 17 {
      iw.commit(random)?;
    }
  }

  let reader = iw.get_reader(random)?;
  iw.close(random)?;
  let searcher = new_searcher_with_reader(reader)?;

  Ok((all_sort_fields, searcher))
}

#[test]
fn test_queries() -> Result<()> {
  let mut random = random();
  let (_all_sort_fields, searcher) = set_up(&mut random)?;

  let n = at_least(&mut random, 20);

  for _ in 0..n {
    assert_query(
      &mut random,
      &searcher,
      MatchAllDocsQuery::new().into(),
      None,
      false,
    )?;

    assert_query(
      &mut random,
      &searcher,
      TermQuery::new(Term::from_text("english", "one")).into(),
      None,
      false,
    )?;

    let mut bq = Builder::new();
    bq.add(
      TermQuery::new(Term::from_text("english", "one")),
      Occur::Should,
    )?;
    bq.add(
      TermQuery::new(Term::from_text("oddeven", "even")),
      Occur::Should,
    )?;
    assert_query(&mut random, &searcher, bq.build().into(), None, false)?;
  }

  Ok(())
}

fn assert_query<IRC>(
  random: &mut impl Rng,
  searcher: &IndexSearcher<IRC>,
  query: Query,
  sort: Option<Sort>,
  is_relevance: bool,
) -> Result<()>
where
  IRC: IndexReaderContext + Sync,
{
  let max_doc = searcher.get_index_reader().max_doc()? as usize;
  let page_size = TestUtil::next_usize(random, 1, max_doc * 2);

  let do_scores;

  let mut all = match sort {
    None => {
      let all_manager = TopScoreDocCollectorManager::with_after(max_doc, None, i32::MAX as usize)?;
      do_scores = false;
      let v = searcher.search_with_collector_manager(query.clone(), &all_manager)?;
      TopDocEnum::Score(v)
    },
    Some(ref sort) => {
      let all_manager =
        TopFieldCollectorManager::with_after(sort.clone(), max_doc, None, i32::MAX as usize)?;
      if is_relevance {
        do_scores = true;
      } else {
        do_scores = random.random_bool(0.5);
      }
      let v = searcher.search_with_collector_manager(query.clone(), &all_manager)?;
      TopDocEnum::Field(v)
    },
  };

  if do_scores {
    match all {
      TopDocEnum::Field(ref mut v) => {
        populate_scores(v.score_docs_mut(), searcher, query.clone())?;
      },
      TopDocEnum::Score(ref mut v) => {
        populate_scores(v.score_docs_mut(), searcher, query.clone())?;
      },
    }
  }

  let mut page_start = 0usize;
  let mut last_bottom: Option<ScoreDocEnum> = None;

  while page_start < all.total_hits().value() {
    let mut paged = match sort.clone() {
      None => {
        let after = match last_bottom.take() {
          Some(ScoreDocEnum::Score(v)) => Some(v),
          None => None,
          _ => unreachable!(),
        };
        let paged_manager =
          TopScoreDocCollectorManager::with_after(page_size, after, i32::MAX as usize)?;
        let v = searcher.search_with_collector_manager(query.clone(), &paged_manager)?;
        TopDocEnum::Score(v)
      },
      Some(sort) => {
        let after = match last_bottom.take() {
          Some(ScoreDocEnum::Field(v)) => Some(v),
          None => None,
          _ => unreachable!(),
        };
        let paged_manager =
          TopFieldCollectorManager::with_after(sort, page_size, after, i32::MAX as usize)?;
        let v = searcher.search_with_collector_manager(query.clone(), &paged_manager)?;
        TopDocEnum::Field(v)
      },
    };

    if do_scores {
      match paged {
        TopDocEnum::Field(ref mut v) => {
          populate_scores(v.score_docs_mut(), searcher, query.clone())?;
        },
        TopDocEnum::Score(ref mut v) => {
          populate_scores(v.score_docs_mut(), searcher, query.clone())?;
        },
      }
    }

    if paged.score_docs().is_empty() {
      break;
    }

    assert_page(searcher, page_start, &all, &paged)?;
    let len = paged.score_docs().len();
    page_start += len;
    last_bottom = match paged {
      TopDocEnum::Field(v) => match v.score_docs()[len - 1].clone() {
        TopFieldScoreDoc::Field(v) => Some(ScoreDocEnum::Field(v)),
        _ => unreachable!("expected field doc"),
      },
      TopDocEnum::Score(v) => Some(ScoreDocEnum::Score(v.score_docs()[len - 1].clone())),
    };
  }

  assert_eq!(all.score_docs().len(), page_start);
  Ok(())
}
fn assert_page<IRC>(
  searcher: &IndexSearcher<IRC>,
  page_start: usize,
  all: &TopDocEnum,
  paged: &TopDocEnum,
) -> Result<()>
where
  IRC: IndexReaderContext,
{
  assert_eq!(all.total_hits().value(), paged.total_hits().value());

  let mut stored_fields = searcher.stored_fields()?;
  for i in 0..paged.score_docs().len() {
    let sd1 = &all.score_docs()[page_start + i];
    let sd2 = &paged.score_docs()[i];

    if cfg!(feature = "test_log_verbose") {
      println!("    hit {}", page_start + i);
      println!(
        "      expected id={:?} {:?}",
        stored_fields.document(sd1.doc())?.get("id")?.unwrap(),
        sd1
      );
      println!(
        "        actual id={:?} {:?}",
        stored_fields.document(sd2.doc())?.get("id")?.unwrap(),
        sd2
      );
    }

    assert_eq!(sd1.doc(), sd2.doc());
    assert_eq!(sd1.score(), sd2.score());
    all.same_field(paged)
  }

  Ok(())
}
enum TopDocEnum {
  Field(TopFieldDocs),
  Score(TopDocs<ScoreDoc>),
}
impl TopDocEnum {
  fn total_hits(&self) -> &TotalHits {
    match self {
      TopDocEnum::Field(field_docs) => field_docs.total_hits(),
      TopDocEnum::Score(score_docs) => score_docs.total_hits(),
    }
  }
  fn score_docs(&self) -> Vec<ScoreDoc> {
    match self {
      TopDocEnum::Field(field_docs) => {
        let mut r = Vec::new();

        for v in field_docs.base.score_docs.iter() {
          r.push(v.score_doc().clone());
        }
        r
      },
      TopDocEnum::Score(score_docs) => score_docs.score_docs.clone(),
    }
  }
  fn same_field(&self, other: &Self) {
    match (self, other) {
      (TopDocEnum::Field(field_docs1), TopDocEnum::Field(field_docs2)) => {
        assert!(field_docs1.fields == field_docs2.fields);
      },
      (TopDocEnum::Score(_), TopDocEnum::Score(_)) => {},
      _ => unreachable!("expected same type"),
    }
  }
}
enum ScoreDocEnum {
  Field(FieldDoc),
  Score(ScoreDoc),
}
impl ScoreDocEnum {
  fn score_docs(&self) -> &ScoreDoc {
    match self {
      ScoreDocEnum::Field(field_doc) => &field_doc.base,
      ScoreDocEnum::Score(score_doc) => score_doc,
    }
  }
}
