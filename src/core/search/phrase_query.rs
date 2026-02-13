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
use crate::core::index::impacts_enum::ImpactsEnum;
use crate::core::index::index_reader::Identity;
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader::LRTermState;
use crate::core::index::term::Term;
use crate::core::index::term_states::TermStates;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::match_no_docs_query::MatchNoDocsQuery;
use crate::core::search::query::{Query, QueryBase, QueryWeight};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::term_query::TermQuery;
use crate::core::util::HasIdentity;
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
use std::cmp::Ordering;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct PhraseQuery {
    id: Identity,
    slop: usize,
    terms: Arc<Vec<Term>>,
    positions: Arc<Vec<usize>>,
    field: Option<String>,
}
impl PhraseQuery {
    /// Create a phrase query which will match documents that contain the given
    /// list of terms at consecutive positions in `field`, and at a maximum edit
    /// distance of `slop`.
    ///
    /// For more complicated use-cases, use [PhraseQuery::builder](Builder).
    ///
    /// # See also
    ///
    /// - [`PhraseQuery::get_slop`]
    pub fn from_terms(slop: usize, field: &str, terms: &[&str]) -> Result<Self> {
        let terms = to_terms(field, terms);
        let positions = incremental_positions(terms.len());
        PhraseQuery::new(slop, terms, positions)
    }

    /// Create a phrase query which will match documents that contain the given
    /// list of terms at consecutive positions in `field`.
    pub fn from_terms_no_slop(field: &str, terms: &[&str]) -> Result<Self> {
        Self::from_terms(0, field, terms)
    }

    /// Create a phrase query which will match documents that contain the given
    /// list of terms at consecutive positions in `field`, and at a maximum edit
    /// distance of `slop`.
    ///
    /// For more complicated use-cases, use [`PhraseQuery::builder`](Builder).
    ///
    /// # See also
    ///
    /// - [`PhraseQuery::get_slop`]
    pub fn from_bytes(slop: usize, field: &str, terms: Vec<BytesRef<Vec<u8>>>) -> Result<Self> {
        let terms = to_terms_from_bytes(field, terms);
        let positions = incremental_positions(terms.len());
        PhraseQuery::new(slop, terms, positions)
    }

    /// Create a phrase query which will match documents that contain the given
    /// list of terms at consecutive positions in `field`.
    pub fn from_bytes_no_slop(field: &str, terms: Vec<BytesRef<Vec<u8>>>) -> Result<Self> {
        Self::from_bytes(0, field, terms)
    }

    /// Return the slop for this `PhraseQuery`.
    ///
    /// The slop is an edit distance between the respective positions of terms as
    /// defined in this `PhraseQuery` and the actual positions of these terms in
    /// a document.
    ///
    /// For instance, when searching for `"quick fox"`, it is expected that the
    /// difference between the positions of `fox` and `quick` is `1`. So
    /// `"a quick brown fox"` would be at an edit distance of `1` since the
    /// difference of the positions of `fox` and `quick` is `2`. Similarly,
    /// `"the fox is quick"` would be at an edit distance of `3` since the
    /// difference of the positions of `fox` and `quick` is `-2`.
    ///
    /// The slop defines the maximum edit distance for a document to match this
    /// phrase query.
    ///
    /// More exact matches are scored higher than sloppier matches, so search
    /// results are ordered by exactness.
    pub fn get_slop(&self) -> usize {
        self.slop
    }

    /// Returns the field this query applies to.
    ///
    /// If the query contains no terms, this returns `None`. Otherwise, it
    /// returns the field shared by all terms in this phrase query.
    pub fn get_field(&self) -> Option<&str> {
        self.field.as_deref()
    }

    /// Returns the list of terms in this phrase.
    ///
    /// The returned slice preserves the order in which terms were added to the
    /// phrase query. All terms are guaranteed to belong to the same field.
    pub fn get_terms(&self) -> &[Term] {
        &self.terms
    }

    /// Returns the relative positions of terms in this phrase.
    ///
    /// The returned slice has the same length as [`get_terms`](Self::get_terms),
    /// and each position corresponds to the term at the same index.
    pub fn get_positions(&self) -> &[usize] {
        &self.positions
    }

    fn new(slop: usize, terms: Vec<Term>, positions: Vec<usize>) -> Result<Self> {
        if terms.len() != positions.len() {
            return Err(LuceneError::illegal_argument(
                "Must have as many terms as positions".to_string(),
            ));
        }
        if terms.len() > 1 {
            let field = terms[0].field();
            for term in &terms[1..] {
                if term.field() != field {
                    return Err(LuceneError::illegal_argument(
                        "All terms should have the same field".to_string(),
                    ));
                }
            }
        }

        for i in 1..positions.len() {
            if positions[i] < positions[i - 1] {
                return Err(LuceneError::illegal_argument(format!(
                    "Positions should not go backwards, got {} before {}",
                    positions[i - 1],
                    positions[i]
                )));
            }
        }

        let field = terms.first().map(|t| t.field().to_string());

        Ok(Self {
            id: Identity::new(),
            slop,
            terms: Arc::new(terms),
            positions: Arc::new(positions),
            field,
        })
    }
}

impl Hash for PhraseQuery {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.slop.hash(state);
        self.terms.hash(state);
        self.positions.hash(state);
    }
}
impl Eq for PhraseQuery {}
impl PartialEq for PhraseQuery {
    fn eq(&self, other: &Self) -> bool {
        self.slop == other.slop && self.terms == other.terms && self.positions == other.positions
    }
}

impl HasIdentity for PhraseQuery {
    fn identity(&self) -> &Identity {
        &self.id
    }
}

impl QueryBase for PhraseQuery {
    fn as_string(&self, f: &str) -> String {
        let mut buffer = String::new();

        if let Some(field) = &self.field
            && field != f
        {
            buffer.push_str(field);
            buffer.push(':');
        }

        buffer.push('"');

        let max_position = self.positions.last().copied();

        let mut pieces: Vec<Option<String>> = match max_position {
            None => Vec::new(),
            Some(max) => vec![None; max + 1],
        };

        for (term, &pos) in self.terms.iter().zip(self.positions.iter()) {
            let text = term.text().unwrap_or_else(|_| "None".to_string());
            match &mut pieces[pos] {
                None => {
                    pieces[pos] = Some(text.to_string());
                },
                Some(existing) => {
                    existing.push('|');
                    existing.push_str(&text);
                },
            }
        }

        for (i, piece) in pieces.iter().enumerate() {
            if i > 0 {
                buffer.push(' ');
            }
            match piece {
                None => buffer.push('?'),
                Some(s) => buffer.push_str(s),
            }
        }

        buffer.push('"');

        if self.slop != 0 {
            buffer.push('~');
            buffer.push_str(&self.slop.to_string());
        }

        buffer
    }

    fn create_weight<IRC>(
        self,
        _searcher: &IndexSearcher<IRC>,
        _score_mode: &ScoreMode,
        _boost: f32,
        _per_reader_term_state: Option<TermStates<LRTermState<IRCLeafReader<IRC>>>>,
    ) -> Result<QueryWeight<IRCLeafReader<IRC>>>
    where
        IRC: IndexReaderContext,
        Self: Sized,
        <IRC as IndexReaderContext>::LeafReader: 'static,
    {
        todo!()
    }

    fn rewrite<IRC>(self, _searcher: &IndexSearcher<IRC>) -> Result<Query>
    where
        IRC: IndexReaderContext,
        Self: Sized,
    {
        let len = self.terms.len();
        if len == 0 {
            Ok(MatchNoDocsQuery::with_message("empty PhraseQuery").into())
        } else if len == 1 {
            Ok(TermQuery::new(self.terms[0].clone()).into())
        } else if let Some(&first_pos) = self.positions.first() {
            if first_pos != 0 {
                let mut new_positions = Vec::with_capacity(self.positions.len());
                for &p in self.positions.iter() {
                    new_positions.push(p - first_pos);
                }
                Ok(PhraseQuery::new(self.slop, (*self.terms).clone(), new_positions)?.into())
            } else {
                Ok(self.into())
            }
        } else {
            Ok(self.into())
        }
    }

    fn visit<QV>(&self, _visitor: &QV)
    where
        QV: QueryVisitor,
    {
        todo!()
    }
}

/// A builder for phrase queries
pub struct Builder {
    slop: usize,
    terms: Vec<Term>,
    positions: Vec<usize>,
}
impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}
impl Builder {
    pub fn new() -> Self {
        Self {
            slop: 0,
            terms: Vec::new(),
            positions: Vec::new(),
        }
    }
    pub fn set_slop(&mut self, slop: usize) -> &mut Self {
        self.slop = slop;
        self
    }
    /// Adds a term to the end of the query phrase.
    ///
    /// The relative position of the term is the one immediately after the last
    /// term added.
    pub fn add_term(&mut self, term: Term) -> Result<&mut Self> {
        let position = match self.positions.last() {
            None => 0,
            Some(&last) => last + 1,
        };
        self.add(term, position)
    }
    /// Adds a term to the end of the query phrase.
    ///
    /// The relative position of the term within the phrase is specified explicitly,
    /// but must be greater than or equal to that of the previously added term.
    /// A greater position allows phrases with gaps (e.g. in connection with
    /// stopwords).
    ///
    /// If the position is equal, you most likely should be using
    /// [`MultiPhraseQuery`](crate::core::search::multi_phrase_query::MultiPhraseQuery) instead, which only requires one term at each position
    /// to match; this class requires all of them.
    pub fn add(&mut self, term: Term, position: usize) -> Result<&mut Self> {
        if let Some(&last_position) = self.positions.last()
            && position < last_position
        {
            return Err(LuceneError::illegal_argument(format!(
                "Positions must be added in order, got {} after {}",
                position, last_position
            )))?;
        }

        if let Some(first_term) = self.terms.first()
            && term.field() != first_term.field()
        {
            return Err(LuceneError::illegal_argument(format!(
                "All terms must be on the same field, got {} and {}",
                term.field(),
                first_term.field()
            )))?;
        }
        self.terms.push(term);
        self.positions.push(position);
        Ok(self)
    }
    /// Build a phrase query based on the terms that have been added.
    pub fn build(self) -> Result<PhraseQuery> {
        PhraseQuery::new(self.slop, self.terms, self.positions)
    }
}

fn incremental_positions(length: usize) -> Vec<usize> {
    (0..length).collect()
}

fn to_terms(field: &str, term_strings: &[&str]) -> Vec<Term> {
    let mut terms = Vec::with_capacity(term_strings.len());
    for &s in term_strings {
        terms.push(Term::from_text(field, s));
    }
    terms
}

fn to_terms_from_bytes(field: &str, term_bytes: Vec<BytesRef<Vec<u8>>>) -> Vec<Term> {
    let mut terms = Vec::with_capacity(term_bytes.len());
    for b in term_bytes {
        terms.push(Term::new(field, b));
    }
    terms
}

pub struct PostingsAndFreq<IE>
where
    IE: ImpactsEnum,
{
    pub(crate) postings: IE,
    pub(crate) position: usize,
    pub(crate) terms: Option<Vec<Term>>,
    pub(crate) n_terms: usize, // for faster comparisons
}
impl<IE> PostingsAndFreq<IE>
where
    IE: ImpactsEnum,
{
    pub fn new(postings: IE, position: usize, terms: &[Term]) -> Self {
        let n_terms = terms.len();

        let terms_vec = if n_terms == 0 {
            None
        } else if n_terms == 1 {
            Some(vec![terms[0].clone()])
        } else {
            let mut v = terms.to_vec();
            v.sort();
            Some(v)
        };

        Self {
            postings,
            position,
            terms: terms_vec,
            n_terms,
        }
    }
}
impl<IE> Ord for PostingsAndFreq<IE>
where
    IE: ImpactsEnum,
{
    fn cmp(&self, other: &Self) -> Ordering {
        match self.position.cmp(&other.position) {
            Ordering::Equal => {},
            ord => return ord,
        }

        match self.n_terms.cmp(&other.n_terms) {
            Ordering::Equal => {},
            ord => return ord,
        }

        if self.n_terms == 0 {
            return Ordering::Equal;
        }

        let a = self.terms.as_ref().unwrap();
        let b = other.terms.as_ref().unwrap();

        for i in 0..a.len() {
            let ord = a[i].cmp(&b[i]);
            if ord != Ordering::Equal {
                return ord;
            }
        }

        Ordering::Equal
    }
}

impl<IE> PartialOrd for PostingsAndFreq<IE>
where
    IE: ImpactsEnum,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl<IE> PartialEq for PostingsAndFreq<IE>
where
    IE: ImpactsEnum,
{
    fn eq(&self, other: &Self) -> bool {
        if self.position != other.position {
            return false;
        }

        match (&self.terms, &other.terms) {
            (None, None) => true,
            (Some(a), Some(b)) => a == b,
            _ => false,
        }
    }
}

impl<IE> Eq for PostingsAndFreq<IE> where IE: ImpactsEnum {}
