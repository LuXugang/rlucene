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
use crate::core::index::index_reader::Identity;
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::term::Term;
use crate::core::index::terms::Terms;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::multi_term_query::{
    ConstantScoreBlendedRewrite, MultiTermQuery, RewriteMethod, RewriteMethodEnum,
};
use crate::core::search::query::{Query, QueryBase, QueryWeight};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::util::HasIdentity;
use crate::core::util::automation::automaton::Automaton;
use crate::core::util::automation::compiled_automaton::{CompiledAutomaton, CompiledAutomatonTE};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
/// A [`Query`] that will match terms against a finite-state machine.
///
/// This query will match documents that contain terms accepted by a given finite-state machine.
/// The automaton can be constructed with the automaton API.
/// Alternatively, it can be created from a regular expression with [`RegexpQuery`](crate::core::search::regexp_query::RegexpQuery) or from the
/// standard Lucene wildcard syntax with [`WildcardQuery`](crate::core::search::regexp_query::RegexpQuery).
///
/// When the query is executed, it will will enumerate the term dictionary in an intelligent way
/// to reduce the number of comparisons. For example: the regular expression of `[dl]og?`
/// will make approximately four comparisons: do, dog, lo, and log.
///
/// @lucene.experimental
#[derive(Clone)]
pub struct AutomatonQuery {
    pub(crate) compiled: CompiledAutomaton,
    pub(crate) term: Term,
    #[allow(dead_code)]
    automaton_is_binary: bool,
    ram_bytes_used: i64,
    id: Identity,
    pub(crate) rewrite_method: RewriteMethodEnum,
}
impl AutomatonQuery {
    /// Create a new `AutomatonQuery` from an [`Automaton`].
    ///
    /// - `term`: [`Term`] containing field and possibly some pattern structure. The term text is
    ///   ignored.
    /// - `automaton`: [`Automaton`] to run, terms that are accepted are considered a match.
    pub fn from_automaton(term: Term, automaton: Automaton) -> Result<Self> {
        Self::from_automaton_with_binary(term, automaton, false)
    }

    /// Create a new `AutomatonQuery` from an [`Automaton`].
    ///
    /// - `term`: [`Term`] containing field and possibly some pattern structure. The term text is
    ///   ignored.
    /// - `automaton`: [`Automaton`] to run, terms that are accepted are considered a match.
    /// - `is_binary`: if `true`, this automaton is already binary and will not go through the
    ///   UTF32ToUTF8 conversion.
    pub fn from_automaton_with_binary(
        term: Term,
        automaton: Automaton,
        is_binary: bool,
    ) -> Result<Self> {
        Self::new(term, automaton, is_binary, ConstantScoreBlendedRewrite)
    }
    /// Create a new `AutomatonQuery` from an [`Automaton`].
    ///
    /// - `term`: [`Term`] containing field and possibly some pattern structure. The term text is
    ///   ignored.
    /// - `automaton`: [`Automaton`] to run, terms that are accepted are considered a match.
    /// - `is_binary`: unused.
    /// - `rewrite_method`: the rewrite method to use to build the final query from the automaton.
    pub fn new<T>(
        term: Term,
        automaton: Automaton,
        is_binary: bool,
        rewrite_method: T,
    ) -> Result<Self>
    where
        T: Into<RewriteMethodEnum>,
    {
        let rewrite_method = rewrite_method.into();
        let compiled = CompiledAutomaton::with_binary(automaton, false, true, is_binary)?;
        // TODO: memory calculation not implement
        let ram_bytes_used = 0;

        Ok(Self {
            compiled,
            term,
            #[allow(dead_code)]
            automaton_is_binary: is_binary,
            ram_bytes_used,
            id: Identity::new(),
            rewrite_method,
        })
    }
    #[cfg(test)]
    pub(crate) fn get_compiled(&self) -> &CompiledAutomaton {
        &self.compiled
    }
}

impl QueryBase for AutomatonQuery {
    fn as_string(&self, field: &str) -> Result<String> {
        let mut buffer = String::new();

        if self.term.field() != field {
            buffer.push_str(self.term.field());
            buffer.push(':');
        }

        buffer.push_str("AutomatonQuery");
        buffer.push_str(" {");
        buffer.push('\n');
        buffer.push('}');

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
        IRCLeafReader<IRC>: 'static,
    {
        Err(LuceneError::unsupported_operation(""))
    }

    fn rewrite<IRC>(self, searcher: &IndexSearcher<IRC>) -> Result<Query>
    where
        IRC: IndexReaderContext,
        Self: Sized,
    {
        let rewrite_method = self.rewrite_method.clone();
        rewrite_method.rewrite(searcher, self.into())
    }

    fn visit<QV>(&self, _visitor: &QV)
    where
        QV: QueryVisitor,
    {
        todo!()
    }
}

impl Debug for AutomatonQuery {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.as_string("") {
            Ok(s) => write!(f, "{}", s),
            Err(_) => Err(std::fmt::Error),
        }
    }
}

impl HasIdentity for AutomatonQuery {
    fn identity(&self) -> &Identity {
        &self.id
    }
}
impl PartialEq for AutomatonQuery {
    fn eq(&self, other: &Self) -> bool {
        if std::ptr::eq(self, other) {
            return true;
        }
        self.compiled == other.compiled && self.term == other.term
    }
}
impl Eq for AutomatonQuery {}
impl Hash for AutomatonQuery {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.compiled.hash(state);
        self.term.hash(state);
    }
}
impl MultiTermQuery for AutomatonQuery {
    fn get_field(&self) -> &str {
        self.term.field()
    }

    type TermsEnum<T>
        = CompiledAutomatonTE<T>
    where
        T: Terms;

    fn get_terms_enum<T>(&self, terms: T) -> Result<Self::TermsEnum<T>>
    where
        T: Terms + Clone,
    {
        self.compiled.get_terms_enum(terms)
    }

    fn as_query(&self) -> Query {
        Query::Automaton(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use crate::core::document::document::Document;
    use crate::core::document::field::Store;
    use crate::core::index::index_reader_context::IndexReaderContext;

    use crate::core::index::BytesRef;
    use crate::core::index::multi_terms::get_terms;
    use crate::core::index::term::Term;
    use crate::core::search::automaton_query::AutomatonQuery;
    use crate::core::search::index_searcher::IndexSearcher;
    use crate::core::search::multi_term_query::{
        ConstantScoreBlendedRewrite, ConstantScoreRewrite, MultiTermQuery,
    };
    use crate::core::search::top_docs::TopDocsLike;
    use crate::core::util::automation::automata::Automata;
    use crate::core::util::automation::automaton::Automaton;
    use crate::core::util::automation::compiled_automaton::CompiledAutomatonTE;
    use crate::core::util::automation::operations::Operations;
    use crate::core::util::error::lucene_error::Result;
    use crate::test::index::random_index_writer::RandomIndexWriter;
    use crate::test::util::DefaultIndexSearch;
    use crate::test::util::lucene_test_case::lucene_test_case_util::{
        is_night_mode, new_directory_shared, new_searcher_with_reader, new_text_field, random,
    };
    use crate::test::util::test_util::TestUtil;
    use rand::Rng;
    use std::collections::HashMap;
    use std::rc::Rc;

    #[allow(dead_code)] // for quick search
    struct TestAutomatonQuery;
    const FN: &str = "field";

    fn set_up<R: Rng + ?Sized>(random: &mut R) -> Result<DefaultIndexSearch> {
        let directory = new_directory_shared(random)?;
        let mut field_to_type = HashMap::new();

        let writer = RandomIndexWriter::new(random, directory);

        let mut doc = Document::new();
        let title_field = new_text_field("title", "some title", Store::No, &mut field_to_type)?;
        let mut field = new_text_field(
            FN,
            "this is document one 2345",
            Store::No,
            &mut field_to_type,
        )?;
        let footer_field = new_text_field("footer", "a footer", Store::No, &mut field_to_type)?;

        doc.add(title_field.clone());
        doc.add(field.clone());
        doc.add(footer_field.clone());
        writer.add_document(doc)?;

        doc = Document::new();
        field.set_string_value("some text from doc two a short piece 5678.91")?;
        doc.add(title_field.clone());
        doc.add(field.clone());
        doc.add(footer_field.clone());
        writer.add_document(doc.clone())?;

        doc = Document::new();
        field.set_string_value(
            "doc three has some different stuff with numbers 1234 5678.9 and letter b",
        )?;
        doc.add(title_field.clone());
        doc.add(field.clone());
        doc.add(footer_field.clone());
        writer.add_document(doc.clone())?;

        let reader = writer.get_reader()?;
        let searcher = new_searcher_with_reader(reader)?;

        writer.close()?;
        Ok(searcher)
    }
    fn new_term(value: &str) -> Term {
        Term::from_text(FN, value)
    }
    fn automaton_query_nr_hits<IRC>(
        searcher: &IndexSearcher<IRC>,
        query: AutomatonQuery,
    ) -> Result<usize>
    where
        IRC: IndexReaderContext,
    {
        let top_docs = searcher.search(query, 5)?;
        Ok(top_docs.total_hits().value())
    }
    fn assert_automaton_hits<IRC>(
        expected: usize,
        automaton: Automaton,
        searcher: &IndexSearcher<IRC>,
    ) -> Result<()>
    where
        IRC: IndexReaderContext,
    {
        // TODO
        // assert_eq!(
        //     expected ,
        //     automaton_query_nr_hits(
        //         searcher,
        //         AutomatonQuery::new(
        //             new_term("bogus"),
        //             automaton.clone(),
        //             false,
        //             ScoringBooleanRewrite,
        //         )?,
        //     )?
        // );

        assert_eq!(
            expected,
            automaton_query_nr_hits(
                searcher,
                AutomatonQuery::new(
                    new_term("bogus"),
                    automaton.clone(),
                    false,
                    ConstantScoreRewrite,
                )?,
            )?
        );

        assert_eq!(
            expected,
            automaton_query_nr_hits(
                searcher,
                AutomatonQuery::new(
                    new_term("bogus"),
                    automaton.clone(),
                    false,
                    ConstantScoreBlendedRewrite,
                )?,
            )?
        );

        assert_eq!(
            expected,
            automaton_query_nr_hits(
                searcher,
                AutomatonQuery::new(
                    new_term("bogus"),
                    automaton,
                    false,
                    ConstantScoreBlendedRewrite,
                )?,
            )?
        );

        Ok(())
    }
    #[test]
    fn test_automata() -> Result<()> {
        let mut random = random();
        let searcher = set_up(&mut random)?;

        assert_automaton_hits(0, Automata::make_empty()?, &searcher)?;
        assert_automaton_hits(0, Automata::make_empty_string()?, &searcher)?;
        assert_automaton_hits(2, Automata::make_any_char()?, &searcher)?;
        assert_automaton_hits(3, Automata::make_any_string()?, &searcher)?;
        assert_automaton_hits(2, Automata::make_string("doc")?, &searcher)?;
        assert_automaton_hits(1, Automata::make_char('a' as i32)?, &searcher)?;
        assert_automaton_hits(
            2,
            Automata::make_char_range('a' as i32, 'b' as i32)?,
            &searcher,
        )?;
        assert_automaton_hits(
            2,
            Automata::make_decimal_interval(1233, 2346, 0)?,
            &searcher,
        )?;

        assert_automaton_hits(
            1,
            Operations::determinize(
                &Automata::make_decimal_interval(0, 2000, 0)?,
                Operations::DEFAULT_DETERMINIZE_WORK_LIMIT,
            )?
            .into_owned(),
            &searcher,
        )?;

        assert_automaton_hits(
            2,
            Operations::union(
                &Automata::make_char('a' as i32)?,
                &Automata::make_char('b' as i32)?,
            )?,
            &searcher,
        )?;

        assert_automaton_hits(
            0,
            Operations::intersection(
                &Automata::make_char('a' as i32)?,
                &Automata::make_char('b' as i32)?,
            )?
            .into_owned(),
            &searcher,
        )?;

        assert_automaton_hits(
            1,
            Operations::minus(
                &Automata::make_char_range('a' as i32, 'b' as i32)?,
                &Automata::make_char('a' as i32)?,
                Operations::DEFAULT_DETERMINIZE_WORK_LIMIT,
            )?
            .into_owned(),
            &searcher,
        )?;

        Ok(())
    }
    fn test_equals() -> Result<()> {
        // TODO WildcardQuery RegexpQuery未实现
        Ok(())
    }
    #[test]
    fn test_rewrite_single_term() -> Result<()> {
        let mut random = random();
        let searcher = set_up(&mut random)?;

        let aq =
            AutomatonQuery::from_automaton(new_term("bogus"), Automata::make_string("piece")?)?;

        let _r = searcher.get_index_reader();
        let terms = Rc::new(get_terms(searcher.get_index_reader(), FN)?.unwrap());

        let te = aq.get_terms_enum(terms)?;
        assert!(matches!(te, CompiledAutomatonTE::Single(_)));

        assert_eq!(1, automaton_query_nr_hits(&searcher, aq)?);
        Ok(())
    }
    /// Test that rewriting to a prefix query works as expected, preserves MultiTermQuery semantics.
    #[test]
    fn test_rewrite_prefix() -> Result<()> {
        let mut random = random();
        let searcher = set_up(&mut random)?;

        let pfx = Automata::make_string("do")?;
        let prefix_automaton = Operations::concatenate(&pfx, &Automata::make_any_string()?)?;

        let aq = AutomatonQuery::from_automaton(new_term("bogus"), prefix_automaton)?;
        assert_eq!(3, automaton_query_nr_hits(&searcher, aq)?);

        Ok(())
    }

    /// Test handling of the empty language
    #[test]
    fn test_empty_optimization() -> Result<()> {
        let mut random = random();
        let searcher = set_up(&mut random)?;

        let aq = AutomatonQuery::from_automaton(new_term("bogus"), Automata::make_empty()?)?;

        let terms = Rc::new(get_terms(searcher.get_index_reader(), FN)?.unwrap());
        let te = aq.get_terms_enum(terms)?;
        assert!(matches!(te, CompiledAutomatonTE::Empty(_)));

        assert_eq!(0, automaton_query_nr_hits(&searcher, aq)?);
        Ok(())
    }
    fn test_hash_code_with_threads() -> Result<()> {
        // TODO IMPORTANT
        Ok(())
    }
    #[test]
    fn test_biggish_automaton() -> Result<()> {
        let mut random = random();

        let num_terms: usize = if is_night_mode() { 3000 } else { 500 };

        let mut terms = Vec::new();
        while terms.len() < num_terms {
            let s = TestUtil::random_unicode_string(&mut random);
            terms.push(BytesRef::from_string(&s));
        }

        terms.sort();

        let automaton = Automata::make_string_union(terms.as_ref())?;
        let _aq = AutomatonQuery::from_automaton(Term::from_text("foo", "bar"), automaton)?;

        Ok(())
    }
}
