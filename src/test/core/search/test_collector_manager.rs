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
use crate::core::index::composite_reader::get_context;
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::search::collector::Collector;
use crate::core::search::collector_manager::CollectorManager;
use crate::core::search::dummy::dummy_weight::DummyWeight;
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::multi_collector::{OneOrMultiCollector, wrap};
use crate::core::search::scorable::Scorable;
use crate::core::search::score::Score;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::weight::Weight;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::lucene_test_case::{new_directory_shared, random};
use crate::test::core::util::test_util::TestUtil;
use rand::{Rng, RngExt};
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};
use std::rc::Rc;

#[allow(dead_code)] // for quick search
struct TestCollectorManager;

#[test]
fn test_collection() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone());
  writer.add_document(&mut random, Document::new())?;
  writer.commit(&mut random)?;
  let reader = Rc::new(writer.get_reader(&mut random)?);
  let ctx = get_context(reader)?;
  let leaves = ctx.leaves()?;

  // Setup two collectors, one that will only collect even doc ids and one that
  // only collects odd. Create some random doc ids and keep track of the ones that we
  // expect each collector manager to collect:
  let even_predicate = Predicate::Even;
  let odd_predicate = Predicate::Odd;

  let cm = CompositeCollectorManager::new(vec![even_predicate, odd_predicate]);

  for _ in 0..100 {
    let docs = TestUtil::next_int(&mut random, 1000, 10000);
    let expected = generate_doc_ids(docs, &mut random);
    let expected_even = expected
      .iter()
      .copied()
      .filter(|doc| even_predicate.test(*doc));
    let expected_odd = expected
      .iter()
      .copied()
      .filter(|doc| odd_predicate.test(*doc));

    // Test only wrapping one of the collector managers:
    let mut result = collect_all(&mut random, &leaves[0], &expected, &cm)?;
    result.sort_unstable();
    let mut expected_result: Vec<i32> = expected_even.chain(expected_odd).collect();
    expected_result.sort_unstable();
    assert_eq!(expected_result, result);
  }

  Ok(())
}

#[test]
fn test_empty_collectors() {
  match CompositeCollectorManager::new(Vec::new()).new_collector() {
    Ok(_) => panic!("expected empty collector manager to fail"),
    Err(err) => assert!(matches!(err, LuceneError::IllegalArgument(_))),
  }
}

fn collect_all<R, CM, LR>(
  random: &mut R,
  ctx: &LeafReaderContext<LR>,
  values: &BTreeSet<i32>,
  collector_manager: &CM,
) -> Result<CM::T>
where
  R: Rng + ?Sized,
  CM: CollectorManager,
  LR: LeafReader + Clone,
{
  let dummy_weight = DummyWeight::<LeafReaderContext<LR>>::new(ctx.reader().clone());
  let mut collectors = Vec::new();
  let mut collector = collector_manager.new_collector()?;
  for v in values {
    if random.random_range(0..10) == 1 {
      collectors.push(collector);
      collector = collector_manager.new_collector()?;
    }
    let mut scorer = Score::new(0.0);
    let mut leaf_collector =
      collector.get_leaf_collector::<_, LeafReaderContext<LR>>(ctx, Some(&dummy_weight))?;
    leaf_collector.collect(*v, &mut scorer)?;
  }
  collectors.push(collector);
  collector_manager.reduce(collectors)
}

/// Generate test doc ids. This will de-dupe and create a sorted collection to be more realistic
/// with real-world use-cases. Note that it's possible this will generate fewer than 'count'
/// entries because of de-duping, but that should be quite rare and probably isn't worth worrying
/// about for these testing purposes.
fn generate_doc_ids(count: i32, random: &mut impl RngExt) -> BTreeSet<i32> {
  let mut generated = BTreeSet::new();
  for _ in 0..count {
    generated.insert(random.random());
  }
  generated
}

#[derive(Clone, Copy)]
enum Predicate {
  Even,
  Odd,
}

impl Predicate {
  fn test(&self, doc: i32) -> bool {
    match self {
      Self::Even => doc % 2 == 0,
      Self::Odd => doc % 2 != 0,
    }
  }
}

struct CompositeCollectorManager {
  predicates: Vec<Predicate>,
}

impl CompositeCollectorManager {
  fn new(predicates: Vec<Predicate>) -> Self {
    Self { predicates }
  }
}

impl CollectorManager for CompositeCollectorManager {
  type C = OneOrMultiCollector<SimpleListCollector>;
  type T = Vec<i32>;

  fn new_collector(&self) -> Result<Self::C> {
    wrap(
      self
        .predicates
        .iter()
        .map(|predicate| Some(SimpleListCollector::new(*predicate))),
    )
  }

  fn reduce(&self, collectors: Vec<Self::C>) -> Result<Self::T> {
    let mut all = Vec::new();
    for collector in collectors {
      match collector {
        OneOrMultiCollector::One(mut collector) => {
          all.append(&mut collector.collected);
        },
        OneOrMultiCollector::Multi(collector) => {
          for mut collector in collector.into_collectors() {
            all.append(&mut collector.collected);
          }
        },
      }
    }

    Ok(all)
  }
}

struct SimpleListCollector {
  predicate: Predicate,
  collected: Vec<i32>,
}

impl SimpleListCollector {
  fn new(predicate: Predicate) -> Self {
    Self {
      predicate,
      collected: Vec::new(),
    }
  }
}

impl Collector for SimpleListCollector {
  type LeafCollector<'a, IRC>
    = &'a mut Self
  where
    Self: 'a,
    IRC: IndexReaderContext;

  fn get_leaf_collector<'a, W, IRC>(
    &'a mut self,
    _context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _weight: Option<&W>,
  ) -> Result<Self::LeafCollector<'a, IRC>>
  where
    IRC: IndexReaderContext,
    W: Weight<IRC> + ?Sized,
  {
    Ok(self)
  }

  fn score_mode(&self) -> ScoreMode {
    ScoreMode::Complete
  }
}

impl Display for SimpleListCollector {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "SimpleListCollector")
  }
}

impl LeafCollector for SimpleListCollector {
  fn collect(&mut self, doc: i32, _scorer: &mut dyn Scorable) -> Result<()> {
    if self.predicate.test(doc) {
      self.collected.push(doc);
    }
    Ok(())
  }
}
