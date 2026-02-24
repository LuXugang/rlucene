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
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::search::collector::Collector;
use crate::core::search::collector_manager::CollectorManager;
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::match_all_docs_query::MatchAllDocsQuery;
use crate::core::search::scorable::Scorable;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::simple_collector::SimpleCollector;
use crate::core::search::weight::Weight;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::index::random_index_writer::RandomIndexWriter;
use crate::test::util::lucene_test_case::lucene_test_case_util::{
    at_least, new_directory_shared, new_searcher_with_reader, random, rarely,
};
use rand::Rng;
use std::fmt::{Display, Formatter};
use std::rc::Rc;

#[allow(dead_code)] // for quick search
struct TestEarlyTermination;

#[test]
fn test_early_termination() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;
    let writer = RandomIndexWriter::new(&mut random, dir.clone());
    let num_docs = at_least(&mut random, 100);
    for _ in 0..num_docs {
        writer.add_document(Document::new())?;
        if rarely(&mut random) {
            writer.commit()?;
        }
    }
    let reader = Rc::new(writer.get_reader()?);
    let iter = at_least(&mut random, 5);
    for _ in 0..iter {
        let searcher = new_searcher_with_reader(reader.clone())?;
        searcher.search_with_collector_manager_states(
            MatchAllDocsQuery::new(),
            &CollectorManagerImpl,
            None,
        )?;
    }

    writer.close()?;
    Ok(())
}
#[derive(Default)]
struct CollectorManagerImpl;
impl CollectorManager for CollectorManagerImpl {
    type C = SimpleCollectorImpl;
    type T = ();

    fn new_collector(&self) -> Result<Self::C> {
        Ok(SimpleCollectorImpl::new())
    }

    fn reduce(&self, _collectors: Vec<Self::C>) -> Result<Self::T> {
        Ok(())
    }
}

struct SimpleCollectorImpl {
    collection_terminated: bool,
}
impl SimpleCollectorImpl {
    fn new() -> Self {
        Self {
            collection_terminated: true,
        }
    }
}

impl Collector for SimpleCollectorImpl {
    type LeafCollector<'a, LR>
        = &'a mut Self
    where
        Self: 'a,
        LR: LeafReader;

    fn get_leaf_collector<'a, W, LR>(
        &'a mut self,
        context: &LeafReaderContext<LR>,
        weight: Option<&W>,
    ) -> Result<Self::LeafCollector<'a, LR>>
    where
        LR: LeafReader,
        W: Weight<LeafReader = LR> + ?Sized,
    {
        SimpleCollector::get_leaf_collector(self, context, weight)?;
        Ok(self)
    }

    fn score_mode(&self) -> ScoreMode {
        ScoreMode::CompleteNoScores
    }
}

impl Display for SimpleCollectorImpl {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", std::any::type_name::<Self>())
    }
}

impl LeafCollector for SimpleCollectorImpl {
    fn collect(&mut self, _doc: i32, _scorer: &mut dyn Scorable) -> Result<()> {
        assert!(!self.collection_terminated);
        if rarely(&mut random()) {
            self.collection_terminated = true;
            return Err(LuceneError::collection_terminated(""));
        }
        Ok(())
    }
}

impl SimpleCollector for SimpleCollectorImpl {
    fn do_set_next_reader<LR>(&mut self, _context: &LeafReaderContext<LR>) -> Result<()>
    where
        LR: LeafReader,
    {
        let mut random = random();
        if random.random_bool(0.5) {
            self.collection_terminated = true;
            return Err(LuceneError::collection_terminated(""));
        } else {
            self.collection_terminated = false;
        }
        Ok(())
    }
}
