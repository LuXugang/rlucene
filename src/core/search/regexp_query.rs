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
    pub const DEFAULT_PROVIDER: fn(&str) -> Option<Automaton> = |_name| None;

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
