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
use crate::core::index::BytesRef;
use crate::core::index::index_reader::Identity;
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::term::Term;
use crate::core::index::terms::Terms;
use crate::core::search::automaton_query::AutomatonQuery;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::multi_term_query::{
    ConstantScoreBlendedRewrite, MultiTermQuery, RewriteMethod, RewriteMethodEnum,
};
use crate::core::search::query::{Query, QueryBase, QueryWeight};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::util::HasIdentity;
use crate::core::util::automation::automaton::Automaton;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
/// A Query that matches documents containing terms with a specified prefix. A PrefixQuery is built
/// by QueryParser for input like `app*`.
///
/// This query uses the [`ConstantScoreBlendedRewrite`] rewrite method.
#[derive(Clone)]
pub struct PrefixQuery {
    base: AutomatonQuery,
    id: Identity,
}
impl PrefixQuery {
    /// Constructs a query for terms starting with `prefix`.
    ///
    /// Uses `CONSTANT_SCORE_BLENDED_REWRITE` as the default rewrite method.
    pub fn new(prefix: Term) -> Result<Self> {
        Self::with_rewrite(prefix, ConstantScoreBlendedRewrite)
    }

    /// Constructs a query for terms starting with `prefix` using a defined rewrite method.
    pub fn with_rewrite<R>(prefix: Term, rewrite_method: R) -> Result<Self>
    where
        R: Into<RewriteMethodEnum>,
    {
        let automaton = to_automaton(prefix.bytes())?;
        let base = AutomatonQuery::new(prefix.clone(), automaton, true, rewrite_method)?;
        Ok(Self {
            base,
            id: Identity::default(),
        })
    }
}
impl QueryBase for PrefixQuery {
    fn as_string(&self, field: &str) -> Result<String> {
        let mut buffer = String::new();

        if self.base.get_field() != field {
            buffer.push_str(self.base.get_field());
            buffer.push(':');
        }

        buffer.push_str(&self.base.term.text()?);
        buffer.push('*');
        Ok(buffer)
    }

    fn create_weight<IRC>(
        self,
        _searcher: &IndexSearcher<IRC>,
        _score_mode: &ScoreMode,
        _boost: f32,
    ) -> Result<QueryWeight<IRC>>
    where
        IRC: IndexReaderContext,
        Self: Sized,
    {
        Err(LuceneError::unsupported_operation(""))
    }

    fn rewrite<IRC>(self, searcher: &IndexSearcher<IRC>) -> Result<Query>
    where
        IRC: IndexReaderContext,
        Self: Sized,
    {
        let rewrite_method = self.base.rewrite_method.clone();
        rewrite_method.rewrite(searcher, self.into())
    }

    fn visit<QV>(&self, _visitor: &QV)
    where
        QV: QueryVisitor,
    {
        todo!()
    }
}

impl Debug for PrefixQuery {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.as_string("") {
            Ok(s) => write!(f, "{}", s),
            Err(_) => Err(std::fmt::Error),
        }
    }
}

impl HasIdentity for PrefixQuery {
    fn identity(&self) -> &Identity {
        &self.id
    }
}

impl MultiTermQuery for PrefixQuery {
    fn get_field(&self) -> &str {
        self.base.get_field()
    }

    type TermsEnum<T>
        = <AutomatonQuery as MultiTermQuery>::TermsEnum<T>
    where
        T: Terms;

    fn get_terms_enum<T>(&self, terms: T) -> Result<Self::TermsEnum<T>>
    where
        T: Terms + Clone,
    {
        self.base.compiled.get_terms_enum(terms)
    }

    fn as_query(&self) -> Query {
        self.clone().into()
    }
}
impl Eq for PrefixQuery {}
impl PartialEq for PrefixQuery {
    fn eq(&self, other: &Self) -> bool {
        self.base == other.base && self.base.term == other.base.term
    }
}
impl Hash for PrefixQuery {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.base.term.hash(state);
        self.base.hash(state);
    }
}
/// Build an automaton accepting all terms with the specified prefix.
pub fn to_automaton(prefix: &BytesRef<Vec<u8>>) -> Result<Automaton> {
    let num_states_and_transitions = prefix.length + 1;
    let mut automaton =
        Automaton::with_capacity(num_states_and_transitions, num_states_and_transitions);

    let mut last_state = automaton.create_state();
    for i in 0..prefix.length {
        let state = automaton.create_state();
        let b = prefix.bytes[prefix.offset + i];
        automaton.add_transition_label(last_state, state, b as i32)?;
        last_state = state;
    }

    automaton.set_accept(last_state, true);
    automaton.add_transition(last_state, last_state, 0, 255)?;
    automaton.finish_state()?;

    debug_assert!(automaton.is_deterministic());
    Ok(automaton)
}
#[cfg(test)]
mod tests {
    use crate::core::document::document::Document;
    use crate::core::document::field::Store;
    use crate::core::index::BytesRef;
    use crate::core::index::term::Term;
    use crate::core::search::prefix_query::PrefixQuery;
    use crate::core::util::StringHelper;
    use crate::core::util::error::lucene_error::Result;
    use crate::test::index::random_index_writer::RandomIndexWriter;
    use crate::test::util::lucene_test_case::lucene_test_case_util::{
        at_least, new_directory_shared, new_searcher_with_reader, new_string_field,
        new_string_field_binary, random,
    };
    use crate::test::util::test_util::TestUtil;
    use rand::Rng;
    use std::collections::HashMap;

    #[allow(dead_code)] // for quick search
    struct TestPrefixQuery;
    #[test]
    fn test_prefix_query() -> Result<()> {
        let mut random = random();
        let directory = new_directory_shared(&mut random)?;

        let categories = ["/Computers", "/Computers/Mac", "/Computers/Windows"];
        let writer = RandomIndexWriter::new(&mut random, directory.clone());
        let mut field_to_type = HashMap::new();

        for cat in categories {
            let mut doc = Document::new();
            doc.add(new_string_field(
                &mut random,
                "category",
                cat,
                Store::Yes,
                &mut field_to_type,
            )?);
            writer.add_document(doc)?;
        }

        let reader = writer.get_reader()?;
        let searcher = new_searcher_with_reader(reader)?;

        let query = PrefixQuery::new(Term::from_text("category", "/Computers"))?;
        let hits = searcher.search(query.clone(), 1000)?.score_docs;
        assert_eq!(
            3,
            hits.len(),
            "All documents in /Computers category and below"
        );

        let query = PrefixQuery::new(Term::from_text("category", "/Computers/Mac"))?;
        let hits = searcher.search(query.clone(), 1000)?.score_docs;
        assert_eq!(1, hits.len(), "One in /Computers/Mac");

        let query = PrefixQuery::new(Term::from_text("category", ""))?;
        let hits = searcher.search(query.clone(), 1000)?.score_docs;
        assert_eq!(3, hits.len(), "everything");

        writer.close()?;
        Ok(())
    }
    #[test]
    fn test_match_all() -> Result<()> {
        let mut random = random();
        let directory = new_directory_shared(&mut random)?;

        let writer = RandomIndexWriter::new(&mut random, directory.clone());
        let mut field_to_type = HashMap::new();

        let mut doc = Document::new();
        doc.add(new_string_field(
            &mut random,
            "field",
            "field",
            Store::Yes,
            &mut field_to_type,
        )?);
        writer.add_document(doc)?;

        let reader = writer.get_reader()?;
        let searcher = new_searcher_with_reader(reader)?;

        let query = PrefixQuery::new(Term::from_text("field", ""))?;
        let top_docs = searcher.search(query, 1000)?;
        assert_eq!(1, top_docs.total_hits.value());

        writer.close()?;
        Ok(())
    }
    #[test]
    fn test_random_binary_prefix() -> Result<()> {
        use rand::seq::SliceRandom;
        use std::collections::HashSet;

        let mut random = random();
        let dir = new_directory_shared(&mut random)?;
        let w = RandomIndexWriter::new(&mut random, dir.clone());
        let mut field_to_type = HashMap::new();

        let num_terms = at_least(&mut random, 1000);
        let mut terms = HashSet::new();
        while terms.len() < num_terms as usize {
            let len = TestUtil::next_int(&mut random, 1, 10) as usize;
            let mut bytes = vec![0u8; len];
            random.fill_bytes(&mut bytes);
            terms.insert(BytesRef::from_bytes(bytes));
        }

        let mut terms_list: Vec<_> = terms.into_iter().collect();
        terms_list.shuffle(&mut random);

        for term in &terms_list {
            let mut doc = Document::new();
            doc.add(new_string_field_binary(
                &mut random,
                "field",
                term.clone(),
                Store::No,
                &mut field_to_type,
            )?);
            w.add_document(doc)?;
        }

        let reader = w.get_reader()?;
        let searcher = new_searcher_with_reader(reader)?;

        let iters = at_least(&mut random, 100);
        for _ in 0..iters {
            let len = (random.next_u32() % 3) as usize;
            let mut bytes = vec![0u8; len];
            random.fill_bytes(&mut bytes);
            let prefix = BytesRef::from_bytes(bytes);

            let q = PrefixQuery::new(Term::new("field", prefix.clone()))?;

            let mut count = 0;
            for term in &terms_list {
                if StringHelper::starts_with_byte_ref(term, &prefix) {
                    count += 1;
                }
            }

            assert_eq!(count, searcher.count(q)?);
        }

        w.close()?;
        Ok(())
    }
}
