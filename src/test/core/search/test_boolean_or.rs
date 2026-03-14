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
use crate::core::document::field::Field;
use crate::core::document::field_type::FieldType;
use crate::core::document::text_field::text_field_type;
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::term::Term;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder;
use crate::core::search::boolean_scorer::BooleanScorer;
use crate::core::search::bulk_scorer::BulkScorer;
use crate::core::search::collector::Collector;
use crate::core::search::doc_id_set::DocIdSet;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::query::Query;
use crate::core::search::scorable::Scorable;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::simple_collector::SimpleCollector;
use crate::core::search::term_query::TermQuery;
use crate::core::search::weight::Weight;
use crate::core::util::TryIntoInt;
use crate::core::util::array_util::ArrayUtil;
use crate::core::util::bit_set::BitSet;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::core::util::int_array_doc_id_set::{IntArrayDocIdSet, IntArrayDocIdSetIterator};
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::DefaultIndexSearchCR;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
    at_least, new_directory_shared, new_index_writer_config_with_analyzer,
    new_searcher_with_reader, random,
};
use crate::test::core::util::test_util::TestUtil;
use once_cell::sync::Lazy;
use rand::Rng;
use rand::prelude::SliceRandom;
use std::fmt::{Display, Formatter};

#[allow(dead_code)]
pub struct TestBooleanOr;

const FIELD_T: &str = "T";
const FIELD_C: &str = "C";

static QUERYS: Lazy<(TermQuery, TermQuery, TermQuery, TermQuery)> = Lazy::new(|| {
    let t1 = TermQuery::new(Term::from_text(FIELD_T, "files"));
    let t2 = TermQuery::new(Term::from_text(FIELD_T, "deleting"));
    let c1 = TermQuery::new(Term::from_text(FIELD_C, "production"));
    let c2 = TermQuery::new(Term::from_text(FIELD_C, "optimize"));
    (t1, t2, c1, c2)
});
fn search<R: Rng + ?Sized>(
    _random: &mut R,
    searcher: &DefaultIndexSearchCR,
    q: impl Into<Query>,
) -> Result<usize> {
    let q = q.into();
    // TODO IMPORTANT  QueryUtils未实现
    // QueryUtils::check(random, q.clone(), searcher)?;
    let v = searcher.search(q, 1000)?.total_hits.value();
    Ok(v)
}
#[test]
fn test_elements() -> Result<()> {
    let mut random = random();
    let searcher = set_up(&mut random)?;

    assert_eq!(1, search(&mut random, &searcher, QUERYS.0.clone())?);
    assert_eq!(1, search(&mut random, &searcher, QUERYS.1.clone())?);
    assert_eq!(1, search(&mut random, &searcher, QUERYS.2.clone())?);
    assert_eq!(1, search(&mut random, &searcher, QUERYS.3.clone())?);

    Ok(())
}
#[test]
fn test_flat() -> Result<()> {
    let mut random = random();
    let searcher = set_up(&mut random)?;

    let mut q = Builder::new();
    q.add(QUERYS.0.clone(), Occur::Should)?;
    q.add(QUERYS.1.clone(), Occur::Should)?;
    q.add(QUERYS.2.clone(), Occur::Should)?;
    q.add(QUERYS.3.clone(), Occur::Should)?;
    assert_eq!(1, search(&mut random, &searcher, q.build())?);

    Ok(())
}

#[test]
fn test_parenthesis_must() -> Result<()> {
    let mut random = random();
    let searcher = set_up(&mut random)?;

    let mut q3 = Builder::new();
    q3.add(QUERYS.0.clone(), Occur::Should)?;
    q3.add(QUERYS.1.clone(), Occur::Should)?;

    let mut q4 = Builder::new();
    q4.add(QUERYS.2.clone(), Occur::Must)?;
    q4.add(QUERYS.3.clone(), Occur::Must)?;

    let mut q2 = Builder::new();
    q2.add(q3.build(), Occur::Should)?;
    q2.add(q4.build(), Occur::Should)?;
    assert_eq!(1, search(&mut random, &searcher, q2.build())?);

    Ok(())
}

#[test]
fn test_parenthesis_must2() -> Result<()> {
    let mut random = random();
    let searcher = set_up(&mut random)?;

    let mut q3 = Builder::new();
    q3.add(QUERYS.0.clone(), Occur::Should)?;
    q3.add(QUERYS.1.clone(), Occur::Should)?;

    let mut q4 = Builder::new();
    q4.add(QUERYS.2.clone(), Occur::Should)?;
    q4.add(QUERYS.3.clone(), Occur::Should)?;

    let mut q2 = Builder::new();
    q2.add(q3.build(), Occur::Should)?;
    q2.add(q4.build(), Occur::Must)?;
    assert_eq!(1, search(&mut random, &searcher, q2.build())?);

    Ok(())
}
#[test]
fn test_parenthesis_should() -> Result<()> {
    let mut random = random();
    let searcher = set_up(&mut random)?;

    let mut q3 = Builder::new();
    q3.add(QUERYS.0.clone(), Occur::Should)?;
    q3.add(QUERYS.1.clone(), Occur::Should)?;

    let mut q4 = Builder::new();
    q4.add(QUERYS.2.clone(), Occur::Should)?;
    q4.add(QUERYS.3.clone(), Occur::Should)?;

    let mut q2 = Builder::new();
    q2.add(q3.build(), Occur::Should)?;
    q2.add(q4.build(), Occur::Should)?;
    assert_eq!(1, search(&mut random, &searcher, q2.build())?);

    Ok(())
}
fn set_up<R: Rng + ?Sized>(random: &mut R) -> Result<DefaultIndexSearchCR> {
    let dir = new_directory_shared(random)?;
    let writer = RandomIndexWriter::new(random, dir.clone());

    let mut d = Document::new();
    d.add(Field::new(
        FIELD_T,
        "Optimize not deleting all files",
        FieldType::from_ref(&*text_field_type::TYPE_STORED)?,
    ));
    d.add(Field::new(
        FIELD_C,
        "Deleted When I run an optimize in our production environment.",
        FieldType::from_ref(&*text_field_type::TYPE_STORED)?,
    ));

    writer.add_document(d)?;

    let reader = writer.get_reader()?;
    let searcher = new_searcher_with_reader(reader)?;
    writer.close()?;
    Ok(searcher)
}
#[test]
fn test_boolean_scorer_max() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;
    let analyzer = MockAnalyzer::new(&mut random);
    let iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
    let riw = RandomIndexWriter::with_config(&mut random, dir.clone(), iwc);

    let doc_count: i32 = at_least(&mut random, 10_000);

    for _ in 0..doc_count {
        let mut doc = Document::new();
        doc.add(Field::new(
            "field",
            "a",
            FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?,
        ));
        riw.add_document(doc)?;
    }

    riw.force_merge(1)?;
    let r = riw.get_reader()?;
    riw.close()?;

    let s = new_searcher_with_reader(r)?;
    let mut bq = Builder::new();
    bq.add(TermQuery::new(Term::from_text("field", "a")), Occur::Should)?;
    bq.add(TermQuery::new(Term::from_text("field", "a")), Occur::Should)?;

    let query = s.rewrite(bq.build())?;
    let w = s.create_weight(query, ScoreMode::Complete, 1.0)?;

    assert_eq!(1, s.get_top_reader_context().leaves()?.len());
    let leaf = &s.get_top_reader_context().leaves()?[0];
    let mut scorer = w.bulk_scorer(leaf, &s)?.unwrap();

    let mut hits = FixedBitSet::new(doc_count as usize);
    let mut c = SimpleCollectorImpl::new(&mut hits);

    while c.end < doc_count {
        let min = c.end;
        let inc = TestUtil::next_int(&mut random, 1, 1000);
        let max = min + inc;
        c.end = max;
        scorer.score(&mut c, None::<&dyn Bits>, min, max)?;
    }

    assert_eq!(doc_count as usize, hits.cardinality());
    Ok(())
}

fn scorer(mut matches: Vec<i32>) -> Result<ScorerImpl> {
    let mut len = matches.len();
    ArrayUtil::grow_exact(&mut matches, len + 1)?;
    len = matches.len();
    matches[len - 1] = NO_MORE_DOCS;
    let it = IntArrayDocIdSet::new(matches, (len - 1).try_convert()?)?.iterator()?;
    Ok(ScorerImpl::new(it))
}

struct SimpleCollectorImpl<'a> {
    hits: &'a mut FixedBitSet,
    end: i32,
}
impl<'a> SimpleCollectorImpl<'a> {
    fn new(hits: &'a mut FixedBitSet) -> Self {
        Self { hits, end: 0 }
    }
}

impl Collector for SimpleCollectorImpl<'_> {
    type LeafCollector<'a, IRC>
        = &'a mut Self
    where
        Self: 'a,
        IRC: IndexReaderContext;

    fn get_leaf_collector<'a, W, IRC>(
        &'a mut self,
        context: &LeafReaderContext<IRCLeafReader<IRC>>,
        weight: Option<&W>,
    ) -> Result<Self::LeafCollector<'a, IRC>>
    where
        IRC: IndexReaderContext,
        W: Weight<IRC> + ?Sized,
    {
        SimpleCollector::get_leaf_collector(self, context, weight)?;
        Ok(self)
    }

    fn score_mode(&self) -> ScoreMode {
        ScoreMode::CompleteNoScores
    }
}

impl LeafCollector for SimpleCollectorImpl<'_> {
    fn collect(&mut self, doc: i32, _scorer: &mut dyn Scorable) -> Result<()> {
        assert!(doc < self.end);
        self.hits.set(doc as usize);
        Ok(())
    }
}

impl Display for SimpleCollectorImpl<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", std::any::type_name::<Self>())
    }
}

impl SimpleCollector for SimpleCollectorImpl<'_> {}

#[test]
fn test_sub_scorer_next_is_not_match() -> Result<()> {
    let mut random = random();
    let mut optional_scorers = vec![
        scorer(vec![100000, 1000001, 9_999_999])?,
        scorer(vec![4000, 1000051])?,
        scorer(vec![5000, 100000, 9_999_998, 9_999_999])?,
    ];
    optional_scorers.shuffle(&mut random);

    let needs_scores = rand::random::<bool>();
    let mut bs = BooleanScorer::new(optional_scorers, 1, needs_scores)?;

    let matches = Vec::new();
    let mut collector = LeafCollectorImpl::new(matches);

    bs.score(&mut collector, None::<&dyn Bits>, 0, NO_MORE_DOCS)?;

    let expected = vec![4000, 5000, 100000, 1000001, 1000051, 9_999_998, 9_999_999];
    assert_eq!(collector.matches, expected);

    Ok(())
}

struct LeafCollectorImpl {
    matches: Vec<i32>,
}
impl LeafCollectorImpl {
    fn new(matches: Vec<i32>) -> LeafCollectorImpl {
        LeafCollectorImpl { matches }
    }
}

impl Display for LeafCollectorImpl {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", std::any::type_name::<Self>(),)
    }
}

impl LeafCollector for LeafCollectorImpl {
    fn collect(&mut self, doc: i32, _scorer: &mut dyn Scorable) -> Result<()> {
        self.matches.push(doc);
        Ok(())
    }
}

struct ScorerImpl {
    it: IntArrayDocIdSetIterator,
}
impl ScorerImpl {
    fn new(it: IntArrayDocIdSetIterator) -> ScorerImpl {
        Self { it }
    }
}

impl Scorable for ScorerImpl {
    fn score(&mut self) -> Result<f32> {
        Ok(0.0)
    }

    fn cost(&self) -> Result<i64> {
        self.iterator().cost()
    }
}

impl Scorer for ScorerImpl {
    fn doc_id(&mut self) -> Result<i32> {
        Ok(self.it.doc_id())
    }

    fn iterator(&self) -> Box<dyn DocIdSetIterator + '_> {
        Box::new(&self.it)
    }

    fn iterator_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
        Box::new(&mut self.it)
    }

    fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
        let ScorerImpl { it } = *self;
        Box::new(it)
    }

    fn get_max_score(&mut self, _up_to: i32) -> Result<f32> {
        Ok(f32::MAX)
    }

    fn has_two_phase_iterator(&self) -> TwoPhaseState {
        TwoPhaseState::No
    }

    fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
        Box::new(&self.it)
    }

    fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
        Box::new(&mut self.it)
    }
}
