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
use crate::core::document::field::Store::Yes;
use crate::core::index::BytesRef;
use crate::core::index::composite_reader_context::CompositeReaderContext;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::log_merge_policy::LogMergePolicy;
use crate::core::index::multi_terms::get_terms;
use crate::core::index::standard_directory_reader::StandardDirectoryReaderType;
use crate::core::index::term::Term;
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::{SeekStatus, TermsEnum};
use crate::core::search::automaton_query::AutomatonQuery;
use crate::core::search::index_searcher::DefaultIndexSearcher;
use crate::core::search::query::IntoQuery;
use crate::core::store::directory::DirEnum;
use crate::core::util::automation::automata::Automata;
use crate::core::util::automation::automaton::Automaton;
use crate::core::util::automation::compiled_automaton::CompiledAutomaton;
use crate::core::util::automation::operations::Operations;
use crate::core::util::automation::reg_exp::RegExp;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::search::check_hits::CheckHits;
use crate::test::core::util::automaton::automaton_test_util::AutomatonTestUtil;
use crate::test::core::util::lucene_test_case::{
  at_least, new_directory_shared, new_index_writer_config_with_analyzer, new_searcher_with_reader,
  new_string_field, random,
};
use crate::test::core::util::test_util::TestUtil;
use rand::Rng;
use rand::RngExt;
use rand::prelude::SliceRandom;
use std::borrow::Cow;
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

#[allow(dead_code)] // for quick search
struct TestTermsEnum2;

#[allow(clippy::type_complexity)]
fn set_up<R>(
  random: &mut R,
) -> Result<(
  i32,
  Arc<DirEnum>,
  BTreeSet<BytesRef<Vec<u8>>>,
  Automaton,
  Arc<StandardDirectoryReaderType<DirEnum>>,
  DefaultIndexSearcher<CompositeReaderContext<Arc<StandardDirectoryReaderType<DirEnum>>>>,
)>
where
  R: Rng + ?Sized,
{
  let num_iterations = at_least(random, 50);

  let dir = new_directory_shared(random)?;

  let mock = MockAnalyzer::new(random);
  let mut iwc = new_index_writer_config_with_analyzer(random, mock);
  iwc.set_merge_policy(LogMergePolicy::log_doc());
  iwc.set_max_buffered_docs(TestUtil::next_int(random, 50, 1000));
  let writer = RandomIndexWriter::with_config(random, dir.clone(), iwc);

  let mut doc = Document::new();
  let mut field = new_string_field(random, "field", "", Yes, &mut HashMap::new())?;
  doc.add(field.clone());

  let mut terms: BTreeSet<BytesRef<Vec<u8>>> = BTreeSet::new();

  let num = at_least(random, 200);
  for _i in 0..num {
    let s = TestUtil::random_unicode_string(random);
    field.set_string_value(&s)?;
    terms.insert(BytesRef::from_string(&s));
    let mut doc = Document::new();
    doc.add(field.clone());
    writer.add_document(random, doc)?;
  }
  let v: Vec<BytesRef<Vec<u8>>> = terms.iter().cloned().collect();
  let terms_automaton = Automata::make_string_union(v.as_slice())?;

  let reader = Arc::new(writer.get_reader(random)?);
  let searcher = new_searcher_with_reader(reader.clone())?;
  writer.close(random)?;

  Ok((
    num_iterations,
    dir.clone(),
    terms,
    terms_automaton,
    reader,
    searcher,
  ))
}
#[test]
fn test_finite_versus_infinite() -> Result<()> {
  let mut random = random();
  let (num_iterations, _dir, terms, _terms_automaton, _reader, searcher) = set_up(&mut random)?;

  for _ in 0..num_iterations {
    let reg = AutomatonTestUtil::random_regexp(&mut random)?;
    let at = RegExp::from_str_with_flags(&reg, RegExp::NONE)?.to_automaton()?;
    let automaton = Operations::determinize(&at, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;

    let mut matched_terms = Vec::new();
    for t in &terms {
      if Operations::run_str(&automaton, &t.utf8_to_string()?) {
        matched_terms.push(t.clone());
      }
    }
    let v = match automaton {
      Cow::Borrowed(_) => at,
      Cow::Owned(a) => a,
    };

    let alternate = Automata::make_string_union(&matched_terms)?;
    let a1 = AutomatonQuery::from_automaton(Term::from_text("field", ""), v)?;
    let a2 = AutomatonQuery::from_automaton(Term::from_text("field", ""), alternate)?;

    let orig_hits = searcher.search(a1.clone(), 25)?.score_docs;
    let new_hits = searcher.search(a2.clone(), 25)?.score_docs;
    CheckHits::check_equal(&a1.into_query(), &orig_hits, &new_hits)?;
  }

  Ok(())
}
#[test]
fn test_seeking() -> Result<()> {
  let mut random = random();
  let (num_iterations, _dir, terms, _terms_automaton, reader, _searcher) = set_up(&mut random)?;

  for _ in 0..num_iterations {
    let reg = AutomatonTestUtil::random_regexp(&mut random)?;
    let automaton = RegExp::from_str_with_flags(&reg, RegExp::NONE)?.to_automaton()?;
    let vv = Operations::determinize(&automaton, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;
    let v = match vv {
      Cow::Borrowed(_) => automaton,
      Cow::Owned(v1) => v1,
    };

    let mut te = get_terms(&reader, "field")?.unwrap().iterator()?;

    let mut unsorted_terms: Vec<&BytesRef<Vec<u8>>> = terms.iter().collect();
    unsorted_terms.shuffle(&mut random);

    for term in unsorted_terms {
      if Operations::run_str(&v, &term.utf8_to_string()?) {
        if random.random_bool(0.5) {
          assert!(te.seek_exact(term)?);
        } else {
          let status = te.seek_ceil(term)?;
          assert_eq!(SeekStatus::Found, status);
          assert_eq!(term, te.term()?.as_ref());
        }
      }
    }
  }

  Ok(())
}
#[test]
fn test_seeking_and_nexting() -> Result<()> {
  let mut random = random();
  let (num_iterations, _dir, terms, _terms_automaton, reader, _searcher) = set_up(&mut random)?;

  for _ in 0..num_iterations {
    let mut te = get_terms(&reader, "field")?.unwrap().iterator()?;

    for term in terms.iter() {
      let c = random.random_range(0..3);
      if c == 0 {
        assert_eq!(term, te.next()?.unwrap().as_ref());
      } else if c == 1 {
        let status = te.seek_ceil(term)?;
        assert_eq!(SeekStatus::Found, status);
        assert_eq!(term, te.term()?.as_ref());
      } else {
        assert!(te.seek_exact(term)?);
      }
    }
  }

  Ok(())
}
#[test]
fn test_intersect() -> Result<()> {
  let mut random = random();
  let (num_iterations, _dir, _terms, terms_automaton, reader, _searcher) = set_up(&mut random)?;

  for _ in 0..num_iterations {
    let reg = AutomatonTestUtil::random_regexp(&mut random)?;
    let automaton = RegExp::from_str_with_flags(&reg, RegExp::NONE)?.to_automaton()?;
    let automaton =
      Operations::determinize(&automaton, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?.into_owned();

    let ca = CompiledAutomaton::new(automaton.clone(), false, false)?;

    let mut te = get_terms(&reader, "field")?.unwrap().intersect(&ca, None)?;
    let v = Operations::intersection(&terms_automaton, &automaton)?.into_owned();
    let expected = match Operations::determinize(&v, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)? {
      Cow::Borrowed(_) => v,
      Cow::Owned(v1) => v1,
    };
    let mut found: BTreeSet<BytesRef<Vec<u8>>> = BTreeSet::new();
    while let Some(term) = te.next()? {
      found.insert(BytesRef::deep_copy_of(&term));
    }

    let v: Vec<BytesRef<Vec<u8>>> = found.iter().cloned().collect();
    let actual = Operations::determinize(
      &Automata::make_string_union(v.as_slice())?,
      Operations::DEFAULT_DETERMINIZE_WORK_LIMIT,
    )?
    .into_owned();

    assert!(AutomatonTestUtil::same_language(&expected, &actual)?);
  }

  Ok(())
}
