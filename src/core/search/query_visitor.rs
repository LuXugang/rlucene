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
use crate::core::index::term::Term;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::query::QueryRef;
use crate::core::util::automation::byte_run_automaton::ByteRunAutomaton;
use crate::core::util::error::lucene_error::Result;
use std::collections::HashSet;

/// Allows recursion through a query tree.
///
/// See [`Query::visit`](crate::core::search::query::QueryBase::visit).
pub trait QueryVisitor {
  /// The visitor type used to visit a child query.
  type SubVisitor<'a>: QueryVisitor
  where
    Self: 'a;

  /// Called by leaf queries that match on specific terms.
  ///
  /// # Parameters
  ///
  /// - `query`: the leaf query
  /// - `terms`: the terms the query will match on
  fn consume_terms(&mut self, _query: QueryRef<'_>, _terms: &[Term]) -> Result<()> {
    Ok(())
  }

  /// Called by leaf queries that match on a class of terms.
  ///
  /// # Parameters
  ///
  /// - `query`: the leaf query
  /// - `field`: the field queried against
  /// - `automaton`: a supplier for an automaton defining which terms match
  ///
  /// Experimental.
  fn consume_terms_matching<A>(
    &mut self,
    query: QueryRef<'_>,
    _field: &str,
    _automaton: A,
  ) -> Result<()>
  where
    A: Fn() -> Result<Option<ByteRunAutomaton>>,
  {
    self.visit_leaf(query) // default impl for backward compatibility
  }

  /// Called by leaf queries that do not match on terms.
  ///
  /// # Parameters
  ///
  /// - `query`: the query
  fn visit_leaf(&mut self, _query: QueryRef<'_>) -> Result<()> {
    Ok(())
  }

  /// Whether or not this field is of interest to the visitor.
  ///
  /// Implement this to avoid collecting terms from heavy queries such as
  /// [`TermInSetQuery`](crate::core::search::term_in_set_query::TermInSetQuery) that are not
  /// running on fields of interest.
  fn accept_field(&self, _field: &str) -> bool {
    true
  }

  /// Pulls a visitor instance for visiting child clauses of a query.
  ///
  /// The Java default implementation returns this visitor, unless `occur` is equal to
  /// [`Occur::MustNot`] in which case it returns [`EMPTY_VISITOR`].
  ///
  /// # Parameters
  ///
  /// - `occur`: the relationship between the parent and its children
  /// - `parent`: the query visited
  fn get_sub_visitor<'a>(&'a mut self, occur: Occur, parent: QueryRef<'_>) -> Self::SubVisitor<'a>;

  /// Default implementation of [`QueryVisitor::get_sub_visitor`].
  fn default_get_sub_visitor<'a>(
    &'a mut self,
    occur: Occur,
    _parent: QueryRef<'_>,
  ) -> DefaultQueryVisitor<'a, Self> {
    if occur == Occur::MustNot {
      DefaultQueryVisitor::Empty
    } else {
      DefaultQueryVisitor::Visitor(self)
    }
  }
}

/// The visitor returned by Java's default `getSubVisitor` behavior.
pub enum DefaultQueryVisitor<'a, V>
where
  V: ?Sized,
{
  Visitor(&'a mut V),
  Empty,
}

impl<V> QueryVisitor for DefaultQueryVisitor<'_, V>
where
  V: QueryVisitor + ?Sized,
{
  type SubVisitor<'a>
    = DefaultQueryVisitor<'a, V>
  where
    Self: 'a;

  fn consume_terms(&mut self, query: QueryRef<'_>, terms: &[Term]) -> Result<()> {
    match self {
      Self::Visitor(visitor) => visitor.consume_terms(query, terms),
      Self::Empty => Ok(()),
    }
  }

  fn consume_terms_matching<S>(
    &mut self,
    query: QueryRef<'_>,
    field: &str,
    automaton: S,
  ) -> Result<()>
  where
    S: Fn() -> Result<Option<ByteRunAutomaton>>,
  {
    match self {
      Self::Visitor(visitor) => visitor.consume_terms_matching(query, field, automaton),
      Self::Empty => Ok(()),
    }
  }

  fn visit_leaf(&mut self, query: QueryRef<'_>) -> Result<()> {
    match self {
      Self::Visitor(visitor) => visitor.visit_leaf(query),
      Self::Empty => Ok(()),
    }
  }

  fn accept_field(&self, field: &str) -> bool {
    match self {
      Self::Visitor(visitor) => visitor.accept_field(field),
      Self::Empty => true,
    }
  }

  fn get_sub_visitor<'a>(
    &'a mut self,
    occur: Occur,
    _parent: QueryRef<'_>,
  ) -> Self::SubVisitor<'a> {
    match self {
      Self::Visitor(visitor) if occur != Occur::MustNot => {
        DefaultQueryVisitor::Visitor(&mut **visitor)
      },
      Self::Visitor(_) | Self::Empty => DefaultQueryVisitor::Empty,
    }
  }
}

impl<V> QueryVisitor for &mut V
where
  V: QueryVisitor + ?Sized,
{
  type SubVisitor<'a>
    = V::SubVisitor<'a>
  where
    Self: 'a;

  fn consume_terms(&mut self, query: QueryRef<'_>, terms: &[Term]) -> Result<()> {
    (**self).consume_terms(query, terms)
  }

  fn consume_terms_matching<A>(
    &mut self,
    query: QueryRef<'_>,
    field: &str,
    automaton: A,
  ) -> Result<()>
  where
    A: Fn() -> Result<Option<ByteRunAutomaton>>,
  {
    (**self).consume_terms_matching(query, field, automaton)
  }

  fn visit_leaf(&mut self, query: QueryRef<'_>) -> Result<()> {
    (**self).visit_leaf(query)
  }

  fn accept_field(&self, field: &str) -> bool {
    (**self).accept_field(field)
  }

  fn get_sub_visitor<'a>(&'a mut self, occur: Occur, parent: QueryRef<'_>) -> Self::SubVisitor<'a> {
    (**self).get_sub_visitor(occur, parent)
  }
}

/// Builds a [`QueryVisitor`] instance that collects all terms that may match a query.
///
/// # Parameters
///
/// - `term_set`: a set to add collected terms to
pub fn term_collector(term_set: &mut HashSet<Term>) -> impl QueryVisitor + use<'_> {
  TermCollector::Collector(term_set)
}

enum TermCollector<'a> {
  Collector(&'a mut HashSet<Term>),
  Empty(EmptyQueryVisitor),
}

impl QueryVisitor for TermCollector<'_> {
  type SubVisitor<'a>
    = TermCollector<'a>
  where
    Self: 'a;

  fn consume_terms(&mut self, _query: QueryRef<'_>, terms: &[Term]) -> Result<()> {
    match self {
      Self::Collector(term_set) => term_set.extend(terms.iter().cloned()),
      Self::Empty(_) => {},
    }
    Ok(())
  }

  fn get_sub_visitor<'a>(
    &'a mut self,
    occur: Occur,
    _parent: QueryRef<'_>,
  ) -> Self::SubVisitor<'a> {
    match self {
      Self::Collector(term_set) if occur != Occur::MustNot => TermCollector::Collector(term_set),
      Self::Collector(_) | Self::Empty(_) => TermCollector::Empty(EMPTY_VISITOR),
    }
  }
}

/// Unit visitor used by [`EMPTY_VISITOR`].
#[derive(Clone, Copy, Debug, Default)]
pub struct EmptyQueryVisitor;

impl QueryVisitor for EmptyQueryVisitor {
  type SubVisitor<'a>
    = EmptyQueryVisitor
  where
    Self: 'a;

  fn get_sub_visitor<'a>(
    &'a mut self,
    _occur: Occur,
    _parent: QueryRef<'_>,
  ) -> Self::SubVisitor<'a> {
    EMPTY_VISITOR
  }
}

/// A [`QueryVisitor`] implementation that does nothing.
pub const EMPTY_VISITOR: EmptyQueryVisitor = EmptyQueryVisitor;
