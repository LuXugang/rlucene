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
use crate::core::util::automation::automaton_provider::{AutomatonProvider, DefaultProvider};
use crate::core::util::automation::operations::Operations;
use crate::core::util::automation::reg_exp::RegExp;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::borrow::Cow;
use std::fmt::{Debug, Formatter};
use std::hash::Hash;

/// A fast regular expression query based on the [`Automaton`] package.
///
/// - Comparisons are [fast](http://tusker.org/regex/regex_benchmark.html)
/// - The term dictionary is enumerated in an intelligent way, to avoid comparisons. See
///   [`AutomatonQuery`] for more details.
///
/// The supported syntax is documented in the [`RegExp`] class. Note this might be different
/// than other regular expression implementations. For some alternatives with different syntax, look
/// under the sandbox.
///
/// Note this query can be slow, as it needs to iterate over many terms. In order to prevent
/// extremely slow RegexpQueries, a Regexp term should not start with the expression `.*`
///
/// See [`RegExp`].
#[derive(Clone)]
pub struct RegexpQuery {
    syntax_flags: i32,
    match_flags: i32,
    determinize_work_limit: i32,
    do_determinization: bool,
    base: AutomatonQuery,
    id: Identity,
}

impl RegexpQuery {
    /// A provider that provides no named automata
    /// Constructs a query for terms matching `term`.
    ///
    /// By default, all regular expression features are enabled.
    pub fn new(term: Term) -> Result<Self> {
        Self::with_flags(term, RegExp::ALL)
    }

    /// Constructs a query for terms matching `term`.
    ///
    /// - `term`: regular expression.
    /// - `flags`: optional RegExp features from [`RegExp`]
    pub fn with_flags(term: Term, flags: i32) -> Result<Self> {
        Self::with_provider(
            term,
            flags,
            &DefaultProvider,
            Operations::DEFAULT_DETERMINIZE_WORK_LIMIT as i32,
        )
    }

    /// Constructs a query for terms matching `term`.
    ///
    /// - `term`: regular expression.
    /// - `flags`: optional RegExp syntax features from [`RegExp`]
    /// - `determinize_work_limit`: maximum effort to spend while compiling the automaton from this
    ///   regexp. Set higher to allow more complex queries and lower to prevent memory exhaustion.
    ///   Use [`Operations::DEFAULT_DETERMINIZE_WORK_LIMIT`] as a decent default if you don't
    ///   otherwise know what to specify.
    pub fn with_flags_and_work_limit(
        term: Term,
        flags: i32,
        determinize_work_limit: i32,
    ) -> Result<Self> {
        Self::with_provider(term, flags, &DefaultProvider, determinize_work_limit)
    }

    /// Constructs a query for terms matching `term`.
    ///
    /// - `term`: regular expression.
    /// - `syntax_flags`: optional RegExp syntax features from [`RegExp`]. The automaton for the regexp
    ///   can result in. Set higher to allow more complex queries and lower to prevent memory
    ///   exhaustion.
    /// - `match_flags`: boolean 'or' of match behavior options such as case insensitivity
    /// - `determinize_work_limit`: maximum effort to spend while compiling the automaton from this
    ///   regexp. Set higher to allow more complex queries and lower to prevent memory exhaustion.
    ///   Use [`Operations::DEFAULT_DETERMINIZE_WORK_LIMIT`] as a decent default if you don't
    ///   otherwise know what to specify.
    pub fn with_syntax_and_match_flags(
        term: Term,
        syntax_flags: i32,
        match_flags: i32,
        determinize_work_limit: i32,
    ) -> Result<Self> {
        Self::with_all(
            term,
            syntax_flags,
            match_flags,
            &DefaultProvider,
            determinize_work_limit,
            ConstantScoreBlendedRewrite,
        )
    }

    /// Constructs a query for terms matching `term`.
    ///
    /// - `term`: regular expression.
    /// - `syntax_flags`: optional RegExp features from [`RegExp`]
    /// - `provider`: custom AutomatonProvider for named automata
    /// - `determinize_work_limit`: maximum effort to spend while compiling the automaton from this
    ///   regexp. Set higher to allow more complex queries and lower to prevent memory exhaustion.
    ///   Use [`Operations::DEFAULT_DETERMINIZE_WORK_LIMIT`] as a decent default if you don't
    ///   otherwise know what to specify.
    pub fn with_provider<T>(
        term: Term,
        syntax_flags: i32,
        provider: &T,
        determinize_work_limit: i32,
    ) -> Result<Self>
    where
        T: AutomatonProvider,
    {
        Self::with_all(
            term,
            syntax_flags,
            0,
            provider,
            determinize_work_limit,
            ConstantScoreBlendedRewrite,
        )
    }

    /// Constructs a query for terms matching `term`.
    ///
    /// - `term`: regular expression.
    /// - `syntax_flags`: optional RegExp features from [`RegExp`]
    /// - `match_flags`: boolean 'or' of match behavior options such as case insensitivity
    /// - `provider`: custom AutomatonProvider for named automata
    /// - `determinize_work_limit`: maximum effort to spend while compiling the automaton from this
    ///   regexp. Set higher to allow more complex queries and lower to prevent memory exhaustion.
    ///   Use [`Operations::DEFAULT_DETERMINIZE_WORK_LIMIT`] as a decent default if you don't
    ///   otherwise know what to specify.
    /// - `rewrite_method`: the rewrite method to use to build the final query
    pub fn with_all<R, T>(
        term: Term,
        syntax_flags: i32,
        match_flags: i32,
        provider: &T,
        determinize_work_limit: i32,
        rewrite_method: R,
    ) -> Result<Self>
    where
        R: Into<RewriteMethodEnum>,
        T: AutomatonProvider,
    {
        Self::with_all_and_determinization(
            term,
            syntax_flags,
            match_flags,
            provider,
            determinize_work_limit,
            rewrite_method,
            true,
        )
    }

    /// Constructs a query for terms matching `term`.
    ///
    /// - `term`: regular expression.
    /// - `syntax_flags`: optional RegExp features from [`RegExp`]
    /// - `match_flags`: boolean 'or' of match behavior options such as case insensitivity
    /// - `provider`: custom AutomatonProvider for named automata
    /// - `determinize_work_limit`: maximum effort to spend while compiling the automaton from this
    ///   regexp. Set higher to allow more complex queries and lower to prevent memory exhaustion.
    ///   Use [`Operations::DEFAULT_DETERMINIZE_WORK_LIMIT`] as a decent default if you don't
    ///   otherwise know what to specify.
    /// - `rewrite_method`: the rewrite method to use to build the final query
    /// - `do_determinization`: whether do determinization to force the query to use DFA as
    ///   run_automaton, if false, the query will not try to determinize the generated automaton from
    ///   regexp such that it might or might not be a DFA. In case it is an NFA, the query will
    ///   eventually use [`NFARunAutomaton`](crate::core::util::automation::nfa_run_automaton::NFARunAutomaton) to execute. Notice
    ///   that [`NFARunAutomaton`](crate::core::util::automation::nfa_run_automaton::NFARunAutomaton) is not thread-safe, so better
    ///   to avoid rewritten method like [`ConstantScoreBlendedRewrite`] when searcher is
    ///   configured with an executor service
    pub fn with_all_and_determinization<R, T>(
        term: Term,
        syntax_flags: i32,
        match_flags: i32,
        provider: &T,
        determinize_work_limit: i32,
        rewrite_method: R,
        do_determinization: bool,
    ) -> Result<Self>
    where
        R: Into<RewriteMethodEnum>,
        T: AutomatonProvider,
    {
        let re = RegExp::parse(&term.text()?, syntax_flags, match_flags)?;
        let automaton = to_automaton(&re, determinize_work_limit, provider, do_determinization)?;
        let base = AutomatonQuery::new(term, automaton, false, rewrite_method)?;

        Ok(Self {
            syntax_flags,
            match_flags,
            determinize_work_limit,
            do_determinization,
            base,
            id: Identity::default(),
        })
    }
}

impl QueryBase for RegexpQuery {
    fn as_string(&self, field: &str) -> Result<String> {
        let mut buffer = String::new();

        if self.base.term.field() != field {
            buffer.push_str(self.base.term.field());
            buffer.push(':');
        }

        buffer.push('/');
        buffer.push_str(&self.base.term.text()?);
        buffer.push('/');
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

impl Debug for RegexpQuery {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.as_string("") {
            Ok(s) => write!(f, "{}", s),
            Err(_) => Err(std::fmt::Error),
        }
    }
}

impl HasIdentity for RegexpQuery {
    fn identity(&self) -> &Identity {
        &self.id
    }
}

impl MultiTermQuery for RegexpQuery {
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

impl Hash for RegexpQuery {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.base.hash(state);
        self.syntax_flags.hash(state);
        self.match_flags.hash(state);
        self.determinize_work_limit.hash(state);
        self.do_determinization.hash(state);
    }
}

impl Eq for RegexpQuery {}

impl PartialEq for RegexpQuery {
    fn eq(&self, other: &Self) -> bool {
        self.base == other.base
            && self.syntax_flags == other.syntax_flags
            && self.match_flags == other.match_flags
            && self.determinize_work_limit == other.determinize_work_limit
            && self.do_determinization == other.do_determinization
    }
}

fn to_automaton<T>(
    regexp: &RegExp,
    determinize_work_limit: i32,
    provider: &T,
    do_determinization: bool,
) -> Result<Automaton>
where
    T: AutomatonProvider,
{
    let a = regexp.to_automaton_from_provider(provider)?;
    if do_determinization {
        match Operations::determinize(&a, determinize_work_limit as usize)? {
            Cow::Owned(o) => Ok(o),
            Cow::Borrowed(_) => Ok(a),
        }
    } else {
        Ok(a)
    }
}

#[cfg(test)]
mod tests {
    use crate::core::document::document::Document;
    use crate::core::document::field::Store;
    use crate::core::index::index_reader_context::IndexReaderContext;
    use crate::core::index::term::Term;
    use crate::core::search::index_searcher::IndexSearcher;
    use crate::core::search::multi_term_query::ConstantScoreBlendedRewrite;
    use crate::core::search::regexp_query::RegexpQuery;
    use crate::core::util::automation::automata::Automata;
    use crate::core::util::automation::automaton::Automaton;
    use crate::core::util::automation::automaton_provider::{AutomatonProvider, DefaultProvider};
    use crate::core::util::automation::operations::Operations;
    use crate::core::util::automation::reg_exp::RegExp;
    use crate::core::util::error::lucene_error::{LuceneError, Result};
    use crate::test::index::random_index_writer::RandomIndexWriter;
    use crate::test::util::DefaultIndexSearch;
    use crate::test::util::lucene_test_case::lucene_test_case_util::{
        new_directory_shared, new_searcher_with_reader, new_text_field, random,
    };
    use rand::Rng;
    use rand::RngExt;
    use std::collections::HashMap;

    #[allow(dead_code)] // for quick search
    struct TestRegexpQuery;
    const FN: &str = "field";
    fn set_up<R: Rng + ?Sized>(random: &mut R) -> Result<DefaultIndexSearch> {
        let directory = new_directory_shared(random)?;

        let writer = RandomIndexWriter::new(random, directory.clone());
        let mut doc = Document::new();
        let mut field_to_type = HashMap::new();
        doc.add(new_text_field(
            random,
            FN,
            "the quick brown fox jumps over the lazy ??? dog 493432 49344 [foo] 12.3 \\",
            Store::No,
            &mut field_to_type,
        )?);
        writer.add_document(doc)?;

        let reader = writer.get_reader()?;
        writer.close()?;

        let searcher = new_searcher_with_reader(reader)?;

        Ok(searcher)
    }
    fn new_term(value: &str) -> Term {
        Term::from_text(FN, value)
    }

    fn regex_query_nr_hits<IRC>(searcher: &IndexSearcher<IRC>, regex: &str) -> Result<i64>
    where
        IRC: IndexReaderContext,
    {
        let query = RegexpQuery::new(new_term(regex))?;
        Ok(searcher.count(query)? as i64)
    }
    fn case_insensitive_regex_query_nr_hits<IRC, R: Rng + ?Sized>(
        random: &mut R,
        searcher: &IndexSearcher<IRC>,
        regex: &str,
    ) -> Result<i64>
    where
        IRC: IndexReaderContext,
    {
        let query = RegexpQuery::with_all_and_determinization(
            new_term(regex),
            RegExp::ALL,
            RegExp::ASCII_CASE_INSENSITIVE,
            &DefaultProvider,
            Operations::DEFAULT_DETERMINIZE_WORK_LIMIT as i32,
            ConstantScoreBlendedRewrite,
            random.random_bool(0.5),
        )?;
        Ok(searcher.count(query)? as i64)
    }
    #[test]
    fn test_regex1() -> Result<()> {
        let mut random = random();
        let searcher = set_up(&mut random)?;
        assert_eq!(1, regex_query_nr_hits(&searcher, "q.[aeiou]c.*")?);
        Ok(())
    }
    #[test]
    fn test_regex2() -> Result<()> {
        let mut random = random();
        let searcher = set_up(&mut random)?;
        assert_eq!(0, regex_query_nr_hits(&searcher, ".[aeiou]c.*")?);
        Ok(())
    }

    #[test]
    fn test_regex3() -> Result<()> {
        let mut random = random();
        let searcher = set_up(&mut random)?;
        assert_eq!(0, regex_query_nr_hits(&searcher, "q.[aeiou]c")?);
        Ok(())
    }

    #[test]
    fn test_numeric_range() -> Result<()> {
        let mut random = random();
        let searcher = set_up(&mut random)?;
        assert_eq!(1, regex_query_nr_hits(&searcher, "<420000-600000>")?);
        assert_eq!(0, regex_query_nr_hits(&searcher, "<493433-600000>")?);
        Ok(())
    }
    #[test]
    fn test_character_classes() -> Result<()> {
        let mut random = random();
        let searcher = set_up(&mut random)?;

        assert_eq!(0, regex_query_nr_hits(&searcher, "\\d")?);
        assert_eq!(1, regex_query_nr_hits(&searcher, "\\d*")?);
        assert_eq!(1, regex_query_nr_hits(&searcher, "\\d{6}")?);
        assert_eq!(1, regex_query_nr_hits(&searcher, "[a\\d]{6}")?);
        assert_eq!(1, regex_query_nr_hits(&searcher, "\\d{2,7}")?);
        assert_eq!(0, regex_query_nr_hits(&searcher, "\\d{4}")?);
        assert_eq!(0, regex_query_nr_hits(&searcher, "\\dog")?);
        assert_eq!(1, regex_query_nr_hits(&searcher, "493\\d32")?);

        assert_eq!(1, regex_query_nr_hits(&searcher, "\\wox")?);
        assert_eq!(1, regex_query_nr_hits(&searcher, "493\\w32")?);
        assert_eq!(1, regex_query_nr_hits(&searcher, "\\?\\?\\?")?);
        assert_eq!(1, regex_query_nr_hits(&searcher, "\\?\\W\\?")?);
        assert_eq!(1, regex_query_nr_hits(&searcher, "\\?\\S\\?")?);

        assert_eq!(1, regex_query_nr_hits(&searcher, "\\[foo\\]")?);
        assert_eq!(1, regex_query_nr_hits(&searcher, "\\[\\w{3}\\]")?);

        assert_eq!(0, regex_query_nr_hits(&searcher, "\\s.*")?);
        assert_eq!(1, regex_query_nr_hits(&searcher, "\\S*ck")?);
        assert_eq!(1, regex_query_nr_hits(&searcher, "[\\d\\.]{3,10}")?);
        assert_eq!(
            1,
            regex_query_nr_hits(&searcher, "\\d{1,3}(\\.(\\d{1,2}))+")?
        );

        assert_eq!(1, regex_query_nr_hits(&searcher, "\\\\")?);
        assert_eq!(1, regex_query_nr_hits(&searcher, "\\\\.*")?);

        let err = regex_query_nr_hits(&searcher, "\\p").unwrap_err();
        match err {
            LuceneError::IllegalArgument(msg) => {
                assert!(msg.to_string().contains("invalid character class"));
            },
            _ => unreachable!(),
        }

        Ok(())
    }
    #[test]
    fn test_case_insensitive() -> Result<()> {
        let mut random = random();
        let searcher = set_up(&mut random)?;

        assert_eq!(0, regex_query_nr_hits(&searcher, "Quick")?);
        assert_eq!(
            1,
            case_insensitive_regex_query_nr_hits(&mut random, &searcher, "Quick")?
        );
        Ok(())
    }

    #[test]
    fn test_regex_negated_character_class() -> Result<()> {
        let mut random = random();
        let searcher = set_up(&mut random)?;

        assert_eq!(1, regex_query_nr_hits(&searcher, "[^a-z]")?);
        assert_eq!(1, regex_query_nr_hits(&searcher, "[^03ad]")?);
        Ok(())
    }
    #[test]
    fn test_custom_provider() -> Result<()> {
        let mut random = random();
        let searcher = set_up(&mut random)?;

        let query = RegexpQuery::with_provider(
            new_term("<quickBrown>"),
            RegExp::ALL,
            &MyProvider,
            Operations::DEFAULT_DETERMINIZE_WORK_LIMIT as i32,
        )?;

        let top_docs = searcher.search(query, 5)?;
        assert_eq!(1, top_docs.total_hits.value());

        Ok(())
    }
    struct MyProvider;
    impl AutomatonProvider for MyProvider {
        fn get_automaton(&self, name: &str) -> Result<Option<Automaton>> {
            if name == "quickBrown" {
                Ok(Some(Operations::union_list(&[
                    &Automata::make_string("quick")?,
                    &Automata::make_string("brown")?,
                    &Automata::make_string("bob")?,
                ])?))
            } else {
                Ok(None)
            }
        }
    }
    /// Test a corner case for backtracking: In this case the term dictionary has 493432 followed by 49344.
    /// When backtracking from 49343... to 4934, it's necessary to test that 4934 itself is ok before trying to
    /// append more characters.
    #[test]
    fn test_backtracking() -> Result<()> {
        let mut random = random();
        let searcher = set_up(&mut random)?;
        assert_eq!(1, regex_query_nr_hits(&searcher, "4934[314]")?);
        Ok(())
    }

    #[test]
    fn test_slow_common_suffix() -> Result<()> {
        let err = RegexpQuery::new(Term::from_text("stringvalue", "(.*a){2000}")).unwrap_err();
        match err {
            LuceneError::TooComplexToDeterminize(_) => {},
            _ => {
                return Err(LuceneError::illegal_state(
                    "expected TooComplexToDeterminizeException",
                ));
            },
        }
        Ok(())
    }
}
