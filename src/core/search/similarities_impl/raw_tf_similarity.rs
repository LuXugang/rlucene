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
use crate::core::search::collection_statistics::CollectionStatistics;
use crate::core::search::similarities_impl::similarities::{SimScorer, Similarity};
use crate::core::search::term_statistics::TermStatistics;
use crate::core::util::error::lucene_error::Result;
use std::fmt::{Display, Formatter};

/// Similarity that returns the raw TF as score.
#[derive(Debug, Clone)]
pub struct RawTFSimilarity {
    discount_overlaps: bool,
}

/// Default constructor: parameter-free
impl Default for RawTFSimilarity {
    fn default() -> Self {
        Self {
            discount_overlaps: true,
        }
    }
}

impl RawTFSimilarity {
    /// Primary constructor
    pub fn with_discount_overlaps(discount_overlaps: bool) -> Self {
        Self { discount_overlaps }
    }
}

impl Display for RawTFSimilarity {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "RawTFSimilarity")
    }
}

impl Similarity for RawTFSimilarity {
    type SimScorer = RawTFSimScorer;

    fn scorer(
        &self,
        boost: f32,
        _collection_stats: &CollectionStatistics,
        _term_stats: &[TermStatistics],
    ) -> Result<Self::SimScorer> {
        Ok(RawTFSimScorer { boost })
    }
}

#[derive(Debug, Clone)]
pub struct RawTFSimScorer {
    boost: f32,
}

impl SimScorer for RawTFSimScorer {
    fn score(&self, freq: f32, _norm: i64) -> f32 {
        self.boost * freq
    }
}

#[cfg(test)]
mod tests {
    use crate::core::document::document::Document;
    use crate::core::document::field::Store;
    use std::collections::HashMap;

    use crate::core::index::directory_reader::directory_reader_util;
    use crate::core::index::index_writer::IndexWriter;
    use crate::core::index::term::Term;
    use crate::core::search::boost_query::BoostQuery;
    use crate::core::search::similarities_impl::raw_tf_similarity::RawTFSimilarity;
    use crate::core::search::term_query::TermQuery;
    use crate::core::util::error::lucene_error::Result;
    use crate::test::search::similarities::base_similarity_test_case::BaseSimilarityTestCase;
    use crate::test::util::DefaultIndexSearch;
    use crate::test::util::lucene_test_case::lucene_test_case_util::{
        new_directory_shared, new_index_writer_config, new_searcher_with_reader, new_text_field,
        random,
    };
    use rand::Rng;

    #[allow(dead_code)]
    struct TestRawTFSimilarity;
    fn set_up<R: Rng + ?Sized>(random: &mut R) -> Result<DefaultIndexSearch> {
        let directory = new_directory_shared(random)?;

        {
            let index_writer =
                IndexWriter::new(directory.clone(), new_index_writer_config(random))?;
            let mut field_types = HashMap::new();
            let mut document1 = Document::new();
            let mut document2 = Document::new();
            let mut document3 = Document::new();

            document1.add(new_text_field("test", "one", Store::Yes, &mut field_types)?);
            document2.add(new_text_field(
                "test",
                "two two",
                Store::Yes,
                &mut field_types,
            )?);
            document3.add(new_text_field(
                "test",
                "three three three",
                Store::Yes,
                &mut field_types,
            )?);

            index_writer.add_document(document1)?;
            index_writer.add_document(document2)?;
            index_writer.add_document(document3)?;
            index_writer.commit()?;
        }

        let index_reader = directory_reader_util::open(directory)?;
        let mut index_searcher = new_searcher_with_reader(index_reader)?;
        index_searcher.set_similarity(RawTFSimilarity::default());

        Ok(index_searcher)
    }
    #[test]
    fn test_one() -> Result<()> {
        let mut random = random();
        let index_searcher = set_up(&mut random)?;
        impl_test(&index_searcher, "one", 1.0)?;
        Ok(())
    }

    #[test]
    fn test_two() -> Result<()> {
        let mut random = random();
        let index_searcher = set_up(&mut random)?;
        impl_test(&index_searcher, "two", 2.0)?;
        Ok(())
    }

    #[test]
    fn test_three() -> Result<()> {
        let mut random = random();
        let index_searcher = set_up(&mut random)?;
        impl_test(&index_searcher, "three", 3.0)?;
        Ok(())
    }

    fn impl_test(
        index_searcher: &DefaultIndexSearch,
        text: &str,
        expected_score: f32,
    ) -> Result<()> {
        let query = TermQuery::new(Term::from_text("test", text));
        let top_docs = index_searcher.search(query, 1)?;

        assert_eq!(1, top_docs.total_hits.value());
        assert_eq!(1, top_docs.score_docs.len());
        assert_eq!(expected_score, top_docs.score_docs[0].score);

        Ok(())
    }
    #[test]
    fn test_boost_query() -> Result<()> {
        let mut random = random();
        let index_searcher = set_up(&mut random)?;

        let query = TermQuery::new(Term::from_text("test", "three"));
        let boost = 14.0f32;

        let top_docs = index_searcher.search(BoostQuery::new(Box::new(query.into()), boost)?, 1)?;

        assert_eq!(1, top_docs.total_hits.value());
        assert_eq!(1, top_docs.score_docs.len());
        assert_eq!(42.0f32, top_docs.score_docs[0].score);

        Ok(())
    }

    impl BaseSimilarityTestCase for TestRawTFSimilarity {
        type Similarity = RawTFSimilarity;

        fn get_similarity<R: Rng + ?Sized>(&self, _random: &mut R) -> Result<Self::Similarity> {
            Ok(RawTFSimilarity::default())
        }
    }
    #[test]
    fn test_random_scoring() -> Result<()> {
        let mut random = random();
        let case = TestRawTFSimilarity;
        case.test_random_scoring(&mut random)
    }
}
