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
use crate::core::document::field::Store;
use crate::core::document::string_field::StringField;
use crate::core::document::text_field::TextField;
use crate::core::index::directory_reader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::term::Term;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder;
use crate::core::search::similarities_impl::raw_tf_similarity::RawTFSimilarity;
use crate::core::search::term_query::TermQuery;
use crate::core::search::top_docs::TopDocsLike;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test_framework::core::util::DefaultIndexSearchCR;
use crate::test_framework::core::util::lucene_test_case::{
  new_directory_shared, new_index_writer_config, new_index_writer_config_with_analyzer,
  new_log_merge_policy, new_searcher_with_reader, new_text_field, random,
};
use rand::Rng;

#[allow(dead_code)] // for quick search
pub struct TestConjunctions;

const F1: &str = "title";
const F2: &str = "body";

fn set_up<R>(random: &mut R) -> Result<DefaultIndexSearchCR>
where
  R: Rng + ?Sized,
{
  let dir = new_directory_shared(random)?;
  let mock = MockAnalyzer::new(random);
  let mut config = new_index_writer_config_with_analyzer(random, mock)?;

  config.set_merge_policy(new_log_merge_policy(random)?);
  let w = IndexWriter::new(dir.clone(), config)?;

  w.add_document(doc(
    "lucene",
    "lucene is a very popular search engine library",
  )?)?;
  w.add_document(doc(
    "solr",
    "solr is a very popular search server and is using lucene",
  )?)?;
  w.add_document(doc(
    "nutch",
    "nutch is an internet search engine with web crawler and is using lucene and hadoop",
  )?)?;

  let reader = directory_reader::open_from_writer(&w)?;
  w.close()?;

  let mut searcher = new_searcher_with_reader(reader)?;
  searcher.set_similarity(RawTFSimilarity::default());

  Ok(searcher)
}
fn doc(v1: &str, v2: &str) -> Result<Document> {
  let mut doc = Document::new();
  doc.add(StringField::from_string(F1, v1, Store::Yes)?);
  doc.add(TextField::from_string(F2, v2, Store::Yes)?);
  Ok(doc)
}
#[test]
fn test_term_conjunctions_with_omit_tf() -> Result<()> {
  let mut random = random();
  let searcher = set_up(&mut random)?;

  let mut builder = Builder::new();
  builder
    .add(TermQuery::new(Term::from_text(F1, "nutch")), Occur::Must)?
    .add(TermQuery::new(Term::from_text(F2, "is")), Occur::Must)?;
  let query = builder.build();

  let top_docs = searcher.search(query, 3)?;
  assert_eq!(1, top_docs.total_hits().value());
  assert!(
    (top_docs.score_docs()[0].score - 3.0).abs() < 0.001,
    "expected score 3.0, got {}",
    top_docs.score_docs()[0].score
  );

  Ok(())
}
#[test]
#[ignore = "Scorable::get_children is not implemented for conjunction scorers"]
fn test_scorer_get_children() -> Result<()> {
  let mut random = random();

  let dir = new_directory_shared(&mut random)?;
  let iwc = new_index_writer_config(&mut random)?;
  let w = IndexWriter::new(dir.clone(), iwc)?;

  let mut field_to_type = HashMap::new();

  let mut doc = Document::new();
  doc.add(new_text_field(
    &mut random,
    "field",
    "a b",
    Store::No,
    &mut field_to_type,
  )?);

  w.add_document(doc)?;

  let r = directory_reader::open_from_writer(&w)?;

  let mut b = Builder::new();
  b.add(TermQuery::new(Term::from_text("field", "a")), Occur::Must)?;
  b.add(TermQuery::new(Term::from_text("field", "b")), Occur::Filter)?;
  let q = b.build();

  let s = new_searcher_with_reader(r)?;

  let manager = CollectorManagerImpl;

  s.search_with_collector_manager(q, &manager)?;

  w.close()?;

  Ok(())
}
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::search::collector::Collector;
use crate::core::search::collector_manager::CollectorManager;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::query::Query;
use crate::core::search::scorable::Scorable;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::simple_collector::SimpleCollector;
use crate::core::search::weight::Weight;
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::sync::{
  Arc,
  atomic::{AtomicBool, Ordering},
};

struct CollectorManagerImpl;

impl CollectorManager for CollectorManagerImpl {
  type C = TestCollector;
  type T = ();

  fn new_collector(&self) -> Result<Self::C> {
    Ok(TestCollector::new())
  }

  fn reduce(&self, collectors: Vec<Self::C>) -> Result<Self::T> {
    for collector in collectors {
      assert!(collector.set_scorer_called.load(Ordering::SeqCst));
    }
    Ok(())
  }
}

struct TestCollector {
  set_scorer_called: Arc<AtomicBool>,
}

impl TestCollector {
  fn new() -> Self {
    Self {
      set_scorer_called: Arc::new(AtomicBool::new(false)),
    }
  }
}

impl Collector for TestCollector {
  type LeafCollector<'a, IRC>
    = &'a mut Self
  where
    Self: 'a,
    IRC: IndexReaderContext + 'a;

  fn get_leaf_collector<'a, W, IRC>(
    &'a mut self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    weight: Option<&W>,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<Self::LeafCollector<'a, IRC>>
  where
    IRC: IndexReaderContext,
    W: Weight<IRC> + ?Sized,
  {
    SimpleCollector::get_leaf_collector(self, context, weight)?;
    Ok(self)
  }

  fn score_mode(&self) -> ScoreMode {
    ScoreMode::Complete
  }

  fn set_weight<W, IRC>(&self, weight: Option<&W>) -> Result<()>
  where
    IRC: IndexReaderContext,
    W: Weight<IRC> + ?Sized,
  {
    let weight = weight
      .as_ref()
      .ok_or_else(|| LuceneError::illegal_state("weight should not None"))?;
    let query = weight.get_query();

    let bq = match query.as_ref() {
      Query::Boolean(q) => q,
      _ => unreachable!("expected BooleanQuery"),
    };

    let clauses = bq.clauses();
    assert_eq!(2, clauses.len());

    let mut terms = HashSet::new();

    for clause in clauses {
      let tq = match &clause.query {
        Query::Term(q) => q,
        _ => unreachable!("expected TermQuery"),
      };

      let term = tq.get_term();
      assert_eq!("field", term.field());
      terms.insert(term.text()?.clone());
    }

    assert_eq!(2, terms.len());
    assert!(terms.contains("a"));
    assert!(terms.contains("b"));

    Ok(())
  }
}

impl LeafCollector for TestCollector {
  fn set_scorer(&mut self, scorer: &mut dyn Scorable) -> Result<()> {
    let _children = scorer.get_children()?;
    self.set_scorer_called.store(true, Ordering::SeqCst);
    // TODO IMPORTANT Restore the child-count assertion after ConjunctionScorer::get_children is
    // implemented.
    // The current owned ChildScorable API cannot borrow the live child scorers.
    // assert_eq!(2, children.len());
    Ok(())
  }

  fn collect(&mut self, _doc: i32, _scorer: &mut dyn Scorable) -> Result<()> {
    Ok(())
  }
}

impl Display for TestCollector {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

impl SimpleCollector for TestCollector {}
