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
use crate::core::index::doc_values::DocValues;
use crate::core::index::doc_values_type::DocValuesType;
use crate::core::index::field_info::FieldInfo;
use crate::core::index::index_options::IndexOptions;
use crate::core::index::index_reader::{Identity, IndexReader};
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader::{LRDisis, LRNormNumericDocValues, LeafReader};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::point_values::PointValues;
use crate::core::index::terms::Terms;
use crate::core::search::constant_score_scorer::ConstantScoreScorer;
use crate::core::search::constant_score_weight::ConstantScoreWeight;
use crate::core::search::doc_id_set_iterator::DocIdSetIteratorEnum3;
use crate::core::search::dummy::dummy_disi::DummyDISI;
use crate::core::search::explanation::Explanation;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::match_all_docs_query::MatchAllDocsQuery;
use crate::core::search::matches_utils::MatchWithNoTerms;
use crate::core::search::query::{Query, QueryBase, QueryWeight, QueryWeightSs};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::weight::{DefaultScorerSupplier, Weight};
use crate::core::util::core_helper::HasIdentity;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// A `Query` that matches documents that contain either a `KnnFloatVectorField`,
/// `KnnByteVectorField`, or a field that indexes norms
/// or doc values.
#[derive(Debug, Clone)]
pub struct FieldExistsQuery {
    id: Identity,
    field: String,
}
impl FieldExistsQuery {
    /// Create a query that will match that have a value for the given `field`.
    pub fn new<T>(field: T) -> Self
    where
        T: Into<String>,
    {
        let field = field.into();
        Self {
            id: Identity::new(),
            field,
        }
    }
    pub fn get_field(&self) -> &str {
        &self.field
    }
    fn build_error_msg(&self, field_info: &FieldInfo) -> String {
        format!(
            "FieldExistsQuery requires that the field indexes doc values, norms or vectors, but field '{}' exists and indexes neither of these data structures",
            field_info.name
        )
    }
    fn get_vector_values_size<LR>(&self, _fi: &FieldInfo, _reader: &LR) -> i32
    where
        LR: LeafReader,
    {
        todo!()
    }
}

impl PartialEq for FieldExistsQuery {
    fn eq(&self, other: &Self) -> bool {
        self.field == other.field
    }
}
impl Eq for FieldExistsQuery {}

impl Hash for FieldExistsQuery {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.field.hash(state);
    }
}

impl HasIdentity for FieldExistsQuery {
    fn identity(&self) -> &Identity {
        &self.id
    }
}

impl QueryBase for FieldExistsQuery {
    fn as_string(&self, _field: &str) -> Result<String> {
        Ok(format!("FieldExistsQuery [field={}]", self.field))
    }

    fn create_weight<IRC>(
        self,
        _searcher: &IndexSearcher<IRC>,
        score_mode: &ScoreMode,
        boost: f32,
    ) -> Result<QueryWeight<IRC>>
    where
        IRC: IndexReaderContext,
        Self: Sized,
    {
        Ok(Box::new(FieldExistsWeight::new(boost, self, *score_mode)))
    }

    fn rewrite<IRC>(self, searcher: &IndexSearcher<IRC>) -> Result<Query>
    where
        IRC: IndexReaderContext,
        Self: Sized,
    {
        let mut all_readers_rewritable = true;

        for context in searcher.get_leaf_contexts()? {
            let leaf = context.reader();
            let field_infos = leaf.get_field_infos()?;
            let field_info = field_infos.field_info_by_name(&self.field);

            let field_info = match field_info {
                Some(fi) => fi,
                None => {
                    all_readers_rewritable = false;
                    break;
                },
            };

            if field_info.has_norms() {
                // the field indexes norms
                if searcher.get_index_reader().get_doc_count(&self.field)?
                    != searcher.get_index_reader().max_doc()?
                {
                    all_readers_rewritable = false;
                    break;
                }
            } else if field_info.get_vector_dimension() != 0 {
                // TODO IMPORTANT
                todo!()
            } else if *field_info.get_doc_values_type() != DocValuesType::None {
                // This optimization is possible due to LUCENE-9334 enforcing a field to always uses the
                // same data structures (all or nothing). Since there's no index statistic to detect when
                // all documents have doc values for a specific field, FieldExistsQuery can only be
                // rewritten to MatchAllDocsQuery for doc values field, when that same field also indexes
                // terms or point values which do have index statistics, and those statistics confirm that
                // all documents in this segment have values terms or point values.
                let terms = leaf.terms(&self.field)?;
                let point_values = leaf.get_point_values(&self.field)?;

                let mut terms_bad = true;
                if let Some(t) = terms.as_ref()
                    && t.get_doc_count()? == leaf.max_doc()?
                {
                    terms_bad = false;
                }

                if terms_bad {
                    let mut points_bad = true;
                    if let Some(p) = point_values.as_ref()
                        && p.get_doc_count()? == leaf.max_doc()?
                    {
                        points_bad = false;
                    }

                    if points_bad {
                        all_readers_rewritable = false;
                        break;
                    }
                }
            } else {
                return Err(LuceneError::illegal_state(
                    self.build_error_msg(field_info.as_ref()),
                ));
            }
        }

        if all_readers_rewritable {
            return Ok(MatchAllDocsQuery::new().into());
        }

        Ok(self.into())
    }

    fn visit<QV>(&self, _visitor: &QV)
    where
        QV: QueryVisitor,
    {
        todo!()
    }
}

pub struct FieldExistsWeight {
    query: FieldExistsQuery,
    base: ConstantScoreWeight,
    parent_query: Arc<Query>,
    score_mode: ScoreMode,
    score: f32,
}
impl FieldExistsWeight {
    fn new(score: f32, query: FieldExistsQuery, score_mode: ScoreMode) -> Self {
        let query_clone = query.clone();
        let parent_query = Arc::new(query_clone.into());
        Self {
            base: ConstantScoreWeight::new(score),
            query,
            parent_query,
            score_mode,
            score,
        }
    }
}

impl<IRC> SegmentCacheable<IRC> for FieldExistsWeight
where
    IRC: IndexReaderContext,
{
    fn is_cacheable(&self, ctx: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<bool> {
        let field_infos = ctx.reader().get_field_infos()?;
        let field_info = field_infos.field_info_by_name(&self.query.field);

        if let Some(fi) = field_info
            && *fi.get_doc_values_type() != DocValuesType::None
        {
            let field = vec![self.query.field.clone()];
            return DocValues::is_cacheable(ctx, field.as_ref());
        }
        Ok(true)
    }
}
pub type Disi<LR> = DocIdSetIteratorEnum3<LRNormNumericDocValues<LR>, DummyDISI, LRDisis<LR>>;
impl<IRC> Weight<IRC> for FieldExistsWeight
where
    IRC: IndexReaderContext,
{
    type Matches = MatchWithNoTerms;

    fn matches(
        &self,
        context: &LeafReaderContext<IRCLeafReader<IRC>>,
        doc: i32,
        searcher: &IndexSearcher<IRC>,
    ) -> Result<Option<Self::Matches>> {
        self.default_matches(context, doc, searcher)
    }

    fn explain(
        &self,
        context: &LeafReaderContext<IRCLeafReader<IRC>>,
        doc: i32,
        searcher: &IndexSearcher<IRC>,
    ) -> Result<Explanation> {
        let scorer = self.scorer(context, searcher)?;
        self.base
            .explain(scorer, doc, self.parent_query.as_string("")?)
    }

    fn get_query(&self) -> Arc<Query> {
        self.parent_query.clone()
    }

    type ScorerSupplier = QueryWeightSs<IRC>;

    fn scorer_supplier(
        &self,
        context: &LeafReaderContext<IRCLeafReader<IRC>>,
        _searcher: &IndexSearcher<IRC>,
    ) -> Result<Option<Self::ScorerSupplier>> {
        let reader = context.reader();
        let field = self.query.get_field();
        let field_infos = reader.get_field_infos()?;
        let field_info = field_infos.field_info_by_name(field);

        let Some(fi) = field_info else {
            return Ok(None);
        };
        let disi_opt = if fi.has_norms() {
            // the field indexes norms
            reader
                .get_norm_values(field)?
                .map(Disi::<IRCLeafReader<IRC>>::A)
        } else if fi.get_vector_dimension() != 0 {
            // TODO IMPORTANT vector未实现
            unimplemented!();
        } else if *fi.get_doc_values_type() != DocValuesType::None {
            match *fi.get_doc_values_type() {
                DocValuesType::Numeric => reader.get_numeric_doc_values(field)?.map(|numeric| {
                    Disi::<IRCLeafReader<IRC>>::C(LRDisis::<IRCLeafReader<IRC>>::A(numeric))
                }),

                DocValuesType::Binary => reader.get_binary_doc_values(field)?.map(|binary| {
                    Disi::<IRCLeafReader<IRC>>::C(LRDisis::<IRCLeafReader<IRC>>::B(binary))
                }),

                DocValuesType::Sorted => reader.get_sorted_doc_values(field)?.map(|sorted| {
                    Disi::<IRCLeafReader<IRC>>::C(LRDisis::<IRCLeafReader<IRC>>::C(sorted))
                }),

                DocValuesType::SortedNumeric => {
                    reader
                        .get_sorted_numeric_doc_values(field)?
                        .map(|sorted_numeric| {
                            Disi::<IRCLeafReader<IRC>>::C(LRDisis::<IRCLeafReader<IRC>>::D(
                                sorted_numeric,
                            ))
                        })
                },

                DocValuesType::SortedSet => {
                    reader.get_sorted_set_doc_values(field)?.map(|sorted_set| {
                        Disi::<IRCLeafReader<IRC>>::C(LRDisis::<IRCLeafReader<IRC>>::E(sorted_set))
                    })
                },
                DocValuesType::None => None,
            }
        } else {
            return Err(LuceneError::illegal_argument(
                self.query.build_error_msg(fi.as_ref()),
            ));
        };
        match disi_opt {
            Some(disi) => Ok(Some(Box::new(DefaultScorerSupplier::new(
                ConstantScoreScorer::from_disi(self.score, self.score_mode, disi),
            )))),
            None => Ok(None),
        }
    }

    fn count(&self, ctx: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<i32> {
        let reader = ctx.reader();

        let field_infos = reader.get_field_infos()?;
        let field_info = field_infos.field_info_by_name(self.query.get_field());

        let Some(fi) = field_info else {
            return Ok(0);
        };

        if fi.has_norms() {
            // the field indexes norms
            // If every field has a value then we can shortcut
            let doc_count = LeafReader::get_doc_count(reader, self.query.get_field())?;
            if doc_count == reader.max_doc()? {
                return reader.num_docs();
            }
            return <Self as Weight<IRC>>::default_count(self, ctx);
        }

        if fi.has_vector_values() {
            // the field indexes vectors
            if !reader.has_deletions()? {
                return Ok(self.query.get_vector_values_size(fi.as_ref(), reader));
            }
            return <Self as Weight<IRC>>::default_count(self, ctx);
        }

        if *fi.get_doc_values_type() != DocValuesType::None {
            // the field indexes doc values
            if !reader.has_deletions()? {
                if fi.get_point_dimension_count() > 0 {
                    if let Some(point_values) = reader.get_point_values(self.query.get_field())? {
                        return point_values.get_doc_count();
                    } else {
                        return Ok(0);
                    }
                }

                if *fi.get_index_options() != IndexOptions::None {
                    if let Some(terms) = reader.terms(self.query.get_field())? {
                        return terms.get_doc_count();
                    } else {
                        return Ok(0);
                    }
                }
            }

            return <Self as Weight<IRC>>::default_count(self, ctx);
        }

        Err(LuceneError::illegal_argument(
            self.query.build_error_msg(fi.as_ref()),
        ))
    }
}

/// Returns a DocIdSetIterator from the given field or None if the field doesn't
/// exist in the reader or if the reader has no doc values for the field.
pub fn get_doc_values_doc_id_set_iterator<LR>(
    field: &str,
    reader: &LR,
) -> Result<Option<LRDisis<LR>>>
where
    LR: LeafReader,
{
    let field_info = reader.get_field_infos()?.field_info_by_name(field);

    let Some(fi) = field_info else {
        return Ok(None);
    };
    let doc_value_type = *fi.get_doc_values_type();
    match doc_value_type {
        DocValuesType::Numeric => match reader.get_numeric_doc_values(field)? {
            Some(numeric) => Ok(Some(LRDisis::<LR>::A(numeric))),
            None => Ok(None),
        },

        DocValuesType::Binary => match reader.get_binary_doc_values(field)? {
            Some(binary) => Ok(Some(LRDisis::<LR>::B(binary))),
            None => Ok(None),
        },

        DocValuesType::Sorted => match reader.get_sorted_doc_values(field)? {
            Some(sorted) => Ok(Some(LRDisis::<LR>::C(sorted))),
            None => Ok(None),
        },

        DocValuesType::SortedNumeric => match reader.get_sorted_numeric_doc_values(field)? {
            Some(sorted_numeric) => Ok(Some(LRDisis::<LR>::D(sorted_numeric))),
            None => Ok(None),
        },

        DocValuesType::SortedSet => match reader.get_sorted_set_doc_values(field)? {
            Some(sorted_set) => Ok(Some(LRDisis::<LR>::E(sorted_set))),
            None => Ok(None),
        },
        DocValuesType::None => Ok(None),
    }
}
#[cfg(test)]
mod test {
    use crate::core::document::document::Document;
    use crate::core::document::field::{Field, Store};
    use std::cmp::Ordering;

    use crate::core::document::numeric_doc_values_field::NumericDocValuesField;

    use crate::core::document::sorted_numeric_doc_values_field::SortedNumericDocValuesField;
    use crate::core::document::string_field::StringField;

    use crate::core::index::index_reader::IndexReader;
    use crate::core::index::index_reader_context::IndexReaderContext;
    use crate::core::index::term::Term;

    use crate::core::search::field_exists_query::FieldExistsQuery;
    use crate::core::search::index_searcher::IndexSearcher;
    use crate::core::search::query::{Query, QueryBase};
    use crate::core::search::score_doc::ScoreDocLike;
    use crate::core::search::sort::Sort;
    use crate::core::search::term_query::TermQuery;
    use crate::core::search::top_docs::TopDocsLike;
    use crate::core::util::error::lucene_error::Result;
    use crate::test::core::index::random_index_writer::RandomIndexWriter;
    use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
        at_least, new_directory_shared, new_index_writer_config, new_searcher_with_reader, random,
    };

    use crate::core::document::binary_point::BinaryPoint;
    use crate::core::document::double_doc_values_field::DoubleDocValuesField;
    use crate::core::document::field_type::FieldType;
    use crate::core::document::long_point::LongPoint;
    use crate::core::document::sorted_doc_values_field::SortedDocValuesField;
    use crate::core::document::text_field::TextField;
    use crate::core::index::BytesRef;
    use crate::core::index::composite_reader::get_context;
    use crate::core::index::index_options::IndexOptions;
    use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
    use crate::core::index::no_merge_policy::NoMergePolicy;
    use crate::core::search::boolean_clause::Occur;
    use crate::core::search::boolean_query::Builder;
    use crate::core::search::boost_query::BoostQuery;
    use crate::core::search::constant_score_query::ConstantScoreQuery;
    use crate::core::search::score_mode::ScoreMode;
    use crate::core::util::TryIntoInt;
    use crate::test::core::util::test_util::TestUtil;
    use rand::RngExt;
    use std::sync::Arc;
    use std::vec;

    #[allow(dead_code)] // for quick search
    struct TestFieldExistsQuery;

    #[test]
    fn test_doc_values_rewrite_with_terms_present() -> Result<()> {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;

        let config = new_index_writer_config(&mut random);
        let iw = RandomIndexWriter::with_config(&mut random, dir.clone(), config);
        let num_docs = at_least(&mut random, 100);

        for _ in 0..num_docs {
            let mut doc = Document::new();
            doc.add(DoubleDocValuesField::new("f", 2.0));
            doc.add(StringField::from_string(
                "f",
                if random.random_bool(0.5) { "yes" } else { "no" },
                Store::No,
            )?);
            iw.add_document(doc)?;
        }

        iw.commit()?;
        let reader = iw.get_reader()?;
        iw.close()?;

        let searcher = new_searcher_with_reader(reader)?;
        let query = FieldExistsQuery::new("f");
        let rewritten = query.rewrite(&searcher)?;

        assert!(matches!(rewritten, Query::MatchAllDocs(_)));

        Ok(())
    }
    #[test]
    fn test_doc_values_rewrite_with_point_values_present() -> Result<()> {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;

        let config = new_index_writer_config(&mut random);
        let iw = RandomIndexWriter::with_config(&mut random, dir.clone(), config);
        let num_docs = at_least(&mut random, 100);

        for _ in 0..num_docs {
            let mut doc = Document::new();
            doc.add(BinaryPoint::new("dim", [vec![0u8; 4], vec![0u8; 4]])?);
            doc.add(DoubleDocValuesField::new("dim", 2.0));
            iw.add_document(doc)?;
        }

        iw.commit()?;
        let reader = iw.get_reader()?;
        iw.close()?;

        let searcher = new_searcher_with_reader(reader)?;
        let query = FieldExistsQuery::new("dim");
        let rewritten = query.rewrite(&searcher)?;

        assert!(matches!(rewritten, Query::MatchAllDocs(_)));

        Ok(())
    }
    #[test]
    fn test_doc_values_no_rewrite() -> Result<()> {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;

        let config = new_index_writer_config(&mut random);
        let iw = RandomIndexWriter::with_config(&mut random, dir.clone(), config);
        let num_docs = at_least(&mut random, 100);

        for _ in 0..num_docs {
            let mut doc = Document::new();
            doc.add(DoubleDocValuesField::new("dim", 2.0));
            doc.add(BinaryPoint::new("dim", [vec![0u8; 4], vec![0u8; 4]])?);
            iw.add_document(doc)?;
        }

        for _ in 0..num_docs {
            let mut doc = Document::new();
            doc.add(DoubleDocValuesField::new("f", 2.0));
            doc.add(StringField::from_string(
                "f",
                if random.random_bool(0.5) { "yes" } else { "no" },
                Store::No,
            )?);
            iw.add_document(doc)?;
        }

        iw.commit()?;
        let reader = iw.get_reader()?;
        iw.close()?;

        let searcher = new_searcher_with_reader(reader)?;

        let rewritten_dim = FieldExistsQuery::new("dim").rewrite(&searcher)?;
        assert!(!matches!(rewritten_dim, Query::MatchAllDocs(_)));

        let rewritten_f = FieldExistsQuery::new("f").rewrite(&searcher)?;
        assert!(!matches!(rewritten_f, Query::MatchAllDocs(_)));

        Ok(())
    }

    #[test]
    fn test_doc_values_no_rewrite_with_doc_values() -> Result<()> {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;

        let config = new_index_writer_config(&mut random);
        let iw = RandomIndexWriter::with_config(&mut random, dir.clone(), config);
        let num_docs = at_least(&mut random, 100);

        for _ in 0..num_docs {
            let mut doc = Document::new();
            doc.add(NumericDocValuesField::new("dv1", 1));
            doc.add(SortedNumericDocValuesField::new("dv2", 1));
            doc.add(SortedNumericDocValuesField::new("dv2", 2));
            iw.add_document(doc)?;
        }

        iw.commit()?;
        let reader = iw.get_reader()?;
        iw.close()?;

        let searcher = new_searcher_with_reader(reader)?;

        let rewritten_dv1 = FieldExistsQuery::new("dv1").rewrite(&searcher)?;
        assert!(!matches!(rewritten_dv1, Query::MatchAllDocs(_)));

        let rewritten_dv2 = FieldExistsQuery::new("dv2").rewrite(&searcher)?;
        assert!(!matches!(rewritten_dv2, Query::MatchAllDocs(_)));

        let rewritten_dv3 = FieldExistsQuery::new("dv3").rewrite(&searcher)?;
        assert!(!matches!(rewritten_dv3, Query::MatchAllDocs(_)));

        Ok(())
    }

    #[test]
    fn test_doc_values_random() -> Result<()> {
        let mut random = random();

        let iters = at_least(&mut random, 10);
        for _ in 0..iters {
            let dir = new_directory_shared(&mut random)?;
            let iw = RandomIndexWriter::new(&mut random, dir.clone());
            let num_docs = at_least(&mut random, 100);

            for _ in 0..num_docs {
                let mut doc = Document::new();
                let has_value = random.random_bool(0.5);

                if has_value {
                    doc.add(NumericDocValuesField::new("dv1", 1));
                    doc.add(SortedNumericDocValuesField::new("dv2", 1));
                    doc.add(SortedNumericDocValuesField::new("dv2", 2));
                    doc.add(StringField::from_string("has_value", "yes", Store::No)?);
                }

                doc.add(StringField::from_string(
                    "f",
                    if random.random_bool(0.5) { "yes" } else { "no" },
                    Store::No,
                )?);

                iw.add_document(doc)?;
            }

            // TODO delete by query 未实现
            // if rng.random_bool(0.5) {
            // }

            iw.commit()?;
            let reader = iw.get_reader()?;
            let searcher = new_searcher_with_reader(reader)?;
            iw.close()?;

            assert_same_matches(
                &searcher,
                TermQuery::new(Term::from_text("has_value", "yes")),
                FieldExistsQuery::new("dv1"),
                false,
            )?;

            assert_same_matches(
                &searcher,
                TermQuery::new(Term::from_text("has_value", "yes")),
                FieldExistsQuery::new("dv2"),
                false,
            )?;
        }

        Ok(())
    }

    #[test]
    fn test_doc_values_approximation() -> Result<()> {
        let mut random = random();
        let iters = at_least(&mut random, 10);

        for _ in 0..iters {
            let dir = new_directory_shared(&mut random)?;
            let config = new_index_writer_config(&mut random);
            let iw = RandomIndexWriter::with_config(&mut random, dir.clone(), config);

            let num_docs = at_least(&mut random, 100);
            for _ in 0..num_docs {
                let mut doc = Document::new();
                let has_value = random.random_bool(0.5);
                if has_value {
                    doc.add(NumericDocValuesField::new("dv1", 1));
                    doc.add(SortedNumericDocValuesField::new("dv2", 1));
                    doc.add(SortedNumericDocValuesField::new("dv2", 2));
                    doc.add(StringField::from_string("has_value", "yes", Store::No)?);
                }
                doc.add(StringField::from_string(
                    "f",
                    if random.random_bool(0.5) { "yes" } else { "no" },
                    Store::No,
                )?);
                iw.add_document(doc)?;
            }
            // TODO: delete-by-query not implement yet
            // if random.random_bool(0.5) {
            //     iw.delete_documents(TermQuery::new(Term::from_text("f", "no")))?;
            // }

            iw.commit()?;
            let reader = iw.get_reader()?;
            let searcher = new_searcher_with_reader(reader)?;
            iw.close()?;

            let mut ref_builder = Builder::new();
            ref_builder
                .add(TermQuery::new(Term::from_text("f", "yes")), Occur::Must)?
                .add(
                    TermQuery::new(Term::from_text("has_value", "yes")),
                    Occur::Filter,
                )?;
            let ref_query = ref_builder.build();

            let mut bq1 = Builder::new();
            bq1.add(TermQuery::new(Term::from_text("f", "yes")), Occur::Must)?
                .add(FieldExistsQuery::new("dv1"), Occur::Filter)?;
            assert_same_matches(&searcher, ref_query.clone(), bq1.build(), true)?;

            let mut bq2 = Builder::new();
            bq2.add(TermQuery::new(Term::from_text("f", "yes")), Occur::Must)?
                .add(FieldExistsQuery::new("dv2"), Occur::Filter)?;
            assert_same_matches(&searcher, ref_query, bq2.build(), true)?;
        }

        Ok(())
    }

    #[test]
    fn test_doc_values_score() -> Result<()> {
        let mut random = random();
        let iters = at_least(&mut random, 10);

        for _ in 0..iters {
            let dir = new_directory_shared(&mut random)?;
            let config = new_index_writer_config(&mut random);
            let iw = RandomIndexWriter::with_config(&mut random, dir.clone(), config);

            let num_docs = at_least(&mut random, 100);
            for _ in 0..num_docs {
                let mut doc = Document::new();
                let has_value = random.random_bool(0.5);
                if has_value {
                    doc.add(NumericDocValuesField::new("dv1", 1));
                    doc.add(SortedNumericDocValuesField::new("dv2", 1));
                    doc.add(SortedNumericDocValuesField::new("dv2", 2));
                    doc.add(StringField::from_string("has_value", "yes", Store::No)?);
                }
                doc.add(StringField::from_string(
                    "f",
                    if random.random_bool(0.5) { "yes" } else { "no" },
                    Store::No,
                )?);
                iw.add_document(doc)?;
            }
            // TODO: delete-by-query not implement yet
            // if random.random_bool(0.5) {
            //     iw.delete_documents(TermQuery::new(Term::from_text("f", "no")))?;
            // }

            iw.commit()?;
            let reader = iw.get_reader()?;
            let searcher = new_searcher_with_reader(reader)?;
            iw.close()?;

            let boost = random.random::<f32>() * 10.0;

            let ref_query: Query = BoostQuery::new(
                ConstantScoreQuery::new(TermQuery::new(Term::from_text("has_value", "yes"))),
                boost,
            )?
            .into();

            let q1: Query = BoostQuery::new(FieldExistsQuery::new("dv1"), boost)?.into();
            assert_same_matches(&searcher, ref_query.clone(), q1, true)?;

            let q2: Query = BoostQuery::new(FieldExistsQuery::new("dv2"), boost)?.into();
            assert_same_matches(&searcher, ref_query, q2, true)?;
        }

        Ok(())
    }

    #[test]
    fn test_doc_values_missing_field() -> Result<()> {
        let mut random = random();

        let dir = new_directory_shared(&mut random)?;
        let iw = RandomIndexWriter::new(&mut random, dir.clone());

        iw.add_document(Document::new())?;
        iw.commit()?;

        let reader = iw.get_reader()?;
        let searcher = new_searcher_with_reader(reader)?;
        iw.close()?;

        assert_eq!(0, searcher.count(FieldExistsQuery::new("f"))?);

        Ok(())
    }
    #[test]
    fn test_doc_values_all_docs_have_field() -> Result<()> {
        let mut random = random();

        let dir = new_directory_shared(&mut random)?;
        let iw = RandomIndexWriter::new(&mut random, dir.clone());

        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("f", 1));
        iw.add_document(doc)?;
        iw.commit()?;

        let reader = iw.get_reader()?;
        let searcher = new_searcher_with_reader(reader)?;
        iw.close()?;

        assert_eq!(1, searcher.count(FieldExistsQuery::new("f"))?);

        Ok(())
    }
    #[test]
    fn test_doc_values_field_exists_but_no_docs_have_field() -> Result<()> {
        let mut random = random();

        let dir = new_directory_shared(&mut random)?;
        let iw = RandomIndexWriter::new(&mut random, dir.clone());

        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("f", 1));
        iw.add_document(doc)?;
        iw.commit()?;

        iw.add_document(Document::new())?;
        iw.commit()?;

        let reader = iw.get_reader()?;
        let searcher = new_searcher_with_reader(reader)?;
        iw.close()?;

        assert_eq!(1, searcher.count(FieldExistsQuery::new("f"))?);

        Ok(())
    }
    #[test]
    fn test_doc_values_query_matches_count() -> Result<()> {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;

        let config = new_index_writer_config(&mut random);
        let mut w = RandomIndexWriter::with_config(&mut random, dir.clone(), config);

        let random_num_docs = random.random_range(11..=100);
        let mut num_matching_docs = 0i32;

        for i in 0..random_num_docs {
            let mut doc = Document::new();
            // We select most documents randomly but keep two documents:
            //  * #0 ensures we will delete at least one document (with long between 0 and 9)
            //  * #10 ensures we will keep at least one document (with long greater than 9)
            if i == 0 || i == 10 || random.random_bool(0.5) {
                let v = i as i64;
                doc.add(LongPoint::new("long", [v])?);
                doc.add(NumericDocValuesField::new("long", v));
                doc.add(StringField::from_string("string", "value", Store::No)?);
                doc.add(SortedDocValuesField::new(
                    "string",
                    BytesRef::from_string("value"),
                ));
                num_matching_docs += 1;
            }
            w.add_document(doc)?;
        }
        w.force_merge(1)?;

        let reader = w.get_reader()?;
        let searcher = new_searcher_with_reader(reader)?;

        assert_same_count(&searcher, "long", num_matching_docs)?;
        assert_same_count(&searcher, "string", num_matching_docs)?;
        assert_same_count(&searcher, "doesNotExist", 0)?;

        // Test that we can't count in O(1) when there are deleted documents
        w.w.get_config_mut()
            .set_merge_policy(NoMergePolicy::default());
        // TODO: delete-by-query not implement yet
        // let v :Vec<Query>= vec![LongPoint::new_range_query("long", 0i64, 9i64)?.into()];
        // w.delete_documents_with_query(v)?;
        // let reader2 = w.get_reader()?;
        // let searcher2 = new_searcher_with_reader(reader2)?;
        //
        // let test_query: Query = FieldExistsQuery::new("long").into();
        // let weight2 = searcher2.create_weight(test_query, ScoreMode::Complete, 1.0)?;
        //
        // let leaf = &searcher2.get_leaf_contexts()?[0];
        // assert_eq!(weight2.count(leaf)?, -1);

        w.close()?;
        Ok(())
    }

    #[test]
    fn test_norms_random() -> Result<()> {
        let mut random = random();

        let iters = at_least(&mut random, 10);
        for _ in 0..iters {
            let dir = new_directory_shared(&mut random)?;
            let iw = RandomIndexWriter::new(&mut random, dir.clone());
            let num_docs = at_least(&mut random, 100);

            for _ in 0..num_docs {
                let mut doc = Document::new();
                let has_value = random.random_bool(0.5);

                if has_value {
                    doc.add(TextField::from_string("text1", "value", Store::No)?);
                    doc.add(StringField::from_string("has_value", "yes", Store::No)?);
                }

                doc.add(StringField::from_string(
                    "f",
                    if random.random_bool(0.5) { "yes" } else { "no" },
                    Store::No,
                )?);

                iw.add_document(doc)?;
            }

            // TODO: delete-by-query not implement yet
            // if random.random_bool(0.5) {
            //     iw.delete_documents(TermQuery::new(...));
            // }

            iw.commit()?;
            let reader = iw.get_reader()?;
            let searcher = new_searcher_with_reader(reader)?;
            iw.close()?;

            assert_same_matches(
                &searcher,
                TermQuery::new(Term::from_text("has_value", "yes")),
                FieldExistsQuery::new("text1"),
                false,
            )?;
        }

        Ok(())
    }
    #[test]
    fn test_norms_approximation() -> Result<()> {
        let mut random = random();
        let iters = at_least(&mut random, 10);

        for _ in 0..iters {
            let dir = new_directory_shared(&mut random)?;
            let config = new_index_writer_config(&mut random);
            let iw = RandomIndexWriter::with_config(&mut random, dir.clone(), config);

            let num_docs = at_least(&mut random, 100);
            for _ in 0..num_docs {
                let mut doc = Document::new();
                let has_value = random.random_bool(0.5);
                if has_value {
                    doc.add(TextField::from_string("text1", "value", Store::No)?);
                    doc.add(StringField::from_string("has_value", "yes", Store::No)?);
                }
                doc.add(StringField::from_string(
                    "f",
                    if random.random_bool(0.5) { "yes" } else { "no" },
                    Store::No,
                )?);
                iw.add_document(doc)?;
            }
            // TODO: delete-by-query not implement yet
            // if random.random_bool(0.5) {
            //     iw.delete_documents(TermQuery::new(Term::from_text("f", "no")))?;
            // }

            iw.commit()?;
            let reader = iw.get_reader()?;
            let searcher = new_searcher_with_reader(reader)?;
            iw.close()?;

            let ref_query: Query = {
                let mut b = Builder::new();
                b.add(TermQuery::new(Term::from_text("f", "yes")), Occur::Must)?
                    .add(
                        TermQuery::new(Term::from_text("has_value", "yes")),
                        Occur::Filter,
                    )?;
                b.build().into()
            };

            let q1: Query = {
                let mut bq1 = Builder::new();
                bq1.add(TermQuery::new(Term::from_text("f", "yes")), Occur::Must)?
                    .add(FieldExistsQuery::new("text1"), Occur::Filter)?;
                bq1.build().into()
            };

            assert_same_matches(&searcher, ref_query, q1, true)?;
        }

        Ok(())
    }

    #[test]
    fn test_norms_score() -> Result<()> {
        let mut random = random();
        let iters = at_least(&mut random, 10);

        for _ in 0..iters {
            let dir = new_directory_shared(&mut random)?;
            let config = new_index_writer_config(&mut random);
            let iw = RandomIndexWriter::with_config(&mut random, dir.clone(), config);

            let num_docs = at_least(&mut random, 100);
            for _ in 0..num_docs {
                let mut doc = Document::new();
                let has_value = random.random_bool(0.5);
                if has_value {
                    doc.add(TextField::from_string("text1", "value", Store::No)?);
                    doc.add(StringField::from_string("has_value", "yes", Store::No)?);
                }
                doc.add(StringField::from_string(
                    "f",
                    if random.random_bool(0.5) { "yes" } else { "no" },
                    Store::No,
                )?);
                iw.add_document(doc)?;
            }

            // TODO: delete-by-query not implemented yet
            // if random.random_bool(0.5) {
            //     iw.delete_documents(TermQuery::new(Term::from_text("f", "no")))?;
            // }

            iw.commit()?;
            let reader = iw.get_reader()?;
            let searcher = new_searcher_with_reader(reader)?;
            iw.close()?;

            let boost = random.random::<f32>() * 10.0;

            let ref_query: Query = BoostQuery::new(
                ConstantScoreQuery::new(TermQuery::new(Term::from_text("has_value", "yes"))),
                boost,
            )?
            .into();

            let q1: Query = BoostQuery::new(FieldExistsQuery::new("text1"), boost)?.into();

            assert_same_matches(&searcher, ref_query, q1, true)?;
        }

        Ok(())
    }

    #[test]
    fn test_norms_missing_field() -> Result<()> {
        let mut random = random();

        let dir = new_directory_shared(&mut random)?;
        let iw = RandomIndexWriter::new(&mut random, dir.clone());

        iw.add_document(Document::new())?;
        iw.commit()?;

        let reader = iw.get_reader()?;
        let searcher = new_searcher_with_reader(reader)?;
        iw.close()?;

        assert_eq!(0, searcher.count(FieldExistsQuery::new("f"))?);

        Ok(())
    }
    #[test]
    fn test_norms_all_docs_have_field() -> Result<()> {
        let mut random = random();

        let dir = new_directory_shared(&mut random)?;
        let iw = RandomIndexWriter::new(&mut random, dir.clone());

        let mut doc = Document::new();
        doc.add(TextField::from_string("f", "value", Store::No)?);
        iw.add_document(doc)?;
        iw.commit()?;

        let reader = iw.get_reader()?;
        let searcher = new_searcher_with_reader(reader)?;
        iw.close()?;

        assert_eq!(1, searcher.count(FieldExistsQuery::new("f"))?);

        Ok(())
    }
    #[test]
    fn test_norms_field_exists_but_no_docs_have_field() -> Result<()> {
        let mut random = random();

        let dir = new_directory_shared(&mut random)?;
        let iw = RandomIndexWriter::new(&mut random, dir.clone());

        let mut doc = Document::new();
        doc.add(TextField::from_string("f", "value", Store::No)?);
        iw.add_document(doc)?;
        iw.commit()?;

        iw.add_document(Document::new())?;
        iw.commit()?;

        let reader = iw.get_reader()?;
        let searcher = new_searcher_with_reader(reader)?;
        iw.close()?;

        assert_eq!(1, searcher.count(FieldExistsQuery::new("f"))?);

        Ok(())
    }
    #[test]
    fn test_norms_query_matches_count() -> Result<()> {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;
        let mut w = RandomIndexWriter::new(&mut random, dir.clone());

        let random_num_docs = TestUtil::next_int(&mut random, 10, 100);

        let mut no_norms_field_type = FieldType::default();
        no_norms_field_type.set_omit_norms(true)?;
        no_norms_field_type.set_index_options(IndexOptions::Docs)?;

        let mut doc = Document::new();
        doc.add(TextField::from_string("text", "always here", Store::No)?);
        doc.add(TextField::from_string("text_s", "", Store::No)?);
        doc.add(Field::new(
            "text_n",
            "always here",
            no_norms_field_type.clone(),
        ));
        w.add_document(doc)?;

        for _i in 1..random_num_docs {
            let mut doc = Document::new();
            doc.add(TextField::from_string("text", "some text", Store::No)?);
            doc.add(TextField::from_string("text_s", "some text", Store::No)?);
            doc.add(Field::new(
                "text_n",
                "some here",
                no_norms_field_type.clone(),
            ));
            w.add_document(doc)?;
        }
        w.force_merge(1)?;

        let reader = w.get_reader()?;
        let searcher = new_searcher_with_reader(reader)?;

        assert_norms_count_with_shortcut(&searcher, "text", random_num_docs)?;
        assert_norms_count_with_shortcut(&searcher, "doesNotExist", 0)?;

        let q = FieldExistsQuery::new("text_n");
        assert!(searcher.count(q).is_err());
        // docs that have a text field that analyzes to an empty token
        // stream still have a recorded norm value but don't show up in
        // Reader.getDocCount(field), so we can't use the shortcut for
        // these fields
        assert_norms_count_without_shortcut(&searcher, "text_s", random_num_docs)?;

        // We can still shortcut with deleted docs
        w.w.get_config_mut()
            .set_merge_policy(NoMergePolicy::default());
        w.delete_documents_with_terms(vec![Term::from_text("text", "text")])?; // deletes all but the first doc

        let reader2 = Arc::new(w.get_reader()?);
        let searcher2 = new_searcher_with_reader(reader2.clone())?;
        assert_norms_count_with_shortcut(&searcher2, "text", 1)?;

        Ok(())
    }
    fn assert_norms_count_without_shortcut<IRC: IndexReaderContext>(
        searcher: &IndexSearcher<IRC>,
        field: &str,
        expected_count: i32,
    ) -> Result<()> {
        let q = FieldExistsQuery::new(field);
        let weight = searcher.create_weight(q.clone(), ScoreMode::Complete, 1.0)?;

        let ctxs = searcher.get_leaf_contexts()?;
        assert_eq!(-1, weight.count(&ctxs[0])?);

        assert_eq!(expected_count, searcher.count(q)?);
        Ok(())
    }

    fn assert_norms_count_with_shortcut<IRC: IndexReaderContext>(
        searcher: &IndexSearcher<IRC>,
        field: &str,
        num_matching_docs: i32,
    ) -> Result<()> {
        let q = FieldExistsQuery::new(field);

        assert_eq!(num_matching_docs, searcher.count(q.clone())?);

        let weight = searcher.create_weight(q, ScoreMode::Complete, 1.0)?;
        let ctxs = searcher.get_leaf_contexts()?;
        assert_eq!(num_matching_docs, weight.count(&ctxs[0])?);
        Ok(())
    }
    fn test_knn_vector_random() -> Result<()> {
        // TODO knn 未实现
        Ok(())
    }
    fn test_knn_vector_missingfield() -> Result<()> {
        // TODO knn 未实现
        Ok(())
    }
    fn test_knn_vector_all_docs_have_field() -> Result<()> {
        // TODO knn 未实现
        Ok(())
    }
    fn test_delete_knn_vector() -> Result<()> {
        // TODO knn 未实现
        Ok(())
    }
    fn test_knn_vector_conjunction() -> Result<()> {
        // TODO knn 未实现
        Ok(())
    }
    fn test_knn_vector_field_exists_but_no_docs_have_field() -> Result<()> {
        // TODO knn 未实现
        Ok(())
    }
    #[test]
    fn test_delete_all_point_docs() -> Result<()> {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;
        let iw = RandomIndexWriter::new(&mut random, dir.clone());

        let mut doc = Document::new();
        doc.add(StringField::from_string("id", "0", Store::No)?);
        doc.add(LongPoint::new("long", vec![17])?);
        doc.add(NumericDocValuesField::new("long", 17));
        iw.add_document(doc)?;

        // add another document before the flush, otherwise the segment only has the document that
        // we are going to delete and the merge simply ignores the segment without carrying over its
        // field infos
        iw.add_document(Document::new())?;

        // make sure there are two segments or force merge will be a no-op
        iw.flush()?;
        iw.add_document(Document::new())?;
        iw.commit()?;

        iw.delete_documents_with_terms(vec![Term::from_text("id", "0")])?;
        iw.force_merge(1)?;

        let reader = iw.get_reader()?;
        assert!(!reader.has_deletions()?);
        let r = get_context(&reader)?;
        assert_eq!(1, r.leaves()?.len());

        let searcher = new_searcher_with_reader(reader)?;
        let q = FieldExistsQuery::new("long");
        assert_eq!(0, searcher.count(q)?);

        Ok(())
    }
    #[test]
    fn test_delete_all_term_docs() -> Result<()> {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;
        let iw = RandomIndexWriter::new(&mut random, dir.clone());

        let mut doc = Document::new();
        doc.add(StringField::from_string("id", "0", Store::No)?);
        doc.add(StringField::from_string("str", "foo", Store::No)?);
        doc.add(SortedDocValuesField::new(
            "str",
            BytesRef::from_bytes(b"foo".to_vec()),
        ));
        iw.add_document(doc)?;

        // add another document before the flush, otherwise the segment only has the document that
        // we are going to delete and the merge simply ignores the segment without carrying over its
        // field infos
        iw.add_document(Document::new())?;

        // make sure there are two segments or force merge will be a no-op
        iw.flush()?;
        iw.add_document(Document::new())?;
        iw.commit()?;

        iw.delete_documents_with_terms(vec![Term::from_text("id", "0")])?;
        iw.force_merge(1)?;

        let reader = iw.get_reader()?;
        assert!(!reader.has_deletions()?);
        let r = get_context(&reader)?;
        assert_eq!(1, r.leaves()?.len());

        let searcher = new_searcher_with_reader(reader)?;
        let q = FieldExistsQuery::new("str");
        assert_eq!(0, searcher.count(q)?);

        Ok(())
    }
    fn assert_same_matches<IRC, T1, T2>(
        searcher: &IndexSearcher<IRC>,
        q1: T1,
        q2: T2,
        scores: bool,
    ) -> Result<()>
    where
        IRC: IndexReaderContext,
        T1: Into<Query>,
        T2: Into<Query>,
    {
        let irc = searcher.get_top_reader_context();
        let max_doc = irc.reader().max_doc()?;

        let sort = if scores {
            Arc::new(Sort::get_relevance()?)
        } else {
            Arc::new(Sort::get_index_order()?)
        };

        let td1 = searcher.search_with_sort(q1, max_doc.try_convert()?, sort.clone())?;
        let td2 = searcher.search_with_sort(q2, max_doc.try_convert()?, sort)?;
        assert_eq!(td1.total_hits().value(), td2.total_hits().value());

        for i in 0..td1.score_docs().len() {
            let sd1 = &td1.score_docs()[i];
            let sd2 = &td2.score_docs()[i];

            assert_eq!(sd1.doc(), sd2.doc());

            if sd1.score().total_cmp(&sd2.score()) != Ordering::Equal {
                let diff = (sd1.score() - sd2.score()).abs();
                assert!(diff <= 1e-7, "score diff={} idx={}", diff, i);
            }
        }

        Ok(())
    }
    fn assert_same_count<IRC>(
        searcher: &IndexSearcher<IRC>,
        field: &str,
        num_matching_docs: i32,
    ) -> Result<()>
    where
        IRC: IndexReaderContext,
    {
        let test_query: Query = FieldExistsQuery::new(field).into();
        assert_eq!(searcher.count(test_query.clone())?, num_matching_docs);

        let weight = searcher.create_weight(test_query, ScoreMode::Complete, 1.0)?;
        assert_eq!(
            weight.count(&searcher.get_leaf_contexts()?[0])?,
            num_matching_docs
        );

        Ok(())
    }
}
