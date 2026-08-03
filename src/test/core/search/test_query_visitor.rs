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
use crate::core::search::boolean_query::{BooleanQuery, Builder as BooleanQueryBuilder};
use crate::core::search::boost_query::BoostQuery;
use crate::core::search::multi_term_query::MultiTermQuerySet;
use crate::core::search::phrase_query::{Builder as PhraseQueryBuilder, PhraseQuery};
use crate::core::search::prefix_query::PrefixQuery;
use crate::core::search::query::{Query, QueryBase, QueryRef};
use crate::core::search::query_visitor::{DefaultQueryVisitor, QueryVisitor, term_collector};
use crate::core::search::term_query::TermQuery;
use crate::core::util::error::lucene_error::Result;
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};

#[allow(dead_code)] // for quick search
struct TestQueryVisitor;

fn query() -> Result<Query> {
  let mut inner = BooleanQueryBuilder::new();
  inner.add(
    TermQuery::new(Term::from_text("field1", "tm2")),
    Occur::Should,
  )?;
  inner.add(
    BoostQuery::new(TermQuery::new(Term::from_text("field1", "tm3")), 2.0)?,
    Occur::Should,
  )?;

  let mut phrase = PhraseQueryBuilder::new();
  phrase.add_term(Term::from_text("field1", "term4"))?;
  phrase.add_term(Term::from_text("field1", "term5"))?;

  let mut field2 = BooleanQueryBuilder::new();
  field2.add(
    BoostQuery::new(TermQuery::new(Term::from_text("field2", "term10")), 3.0)?,
    Occur::Must,
  )?;

  let mut query = BooleanQueryBuilder::new();
  query.add(TermQuery::new(Term::from_text("field1", "t1")), Occur::Must)?;
  query.add(inner.build(), Occur::Must)?;
  query.add(BoostQuery::new(phrase.build()?, 3.0)?, Occur::Must)?;
  query.add(
    TermQuery::new(Term::from_text("field1", "term8")),
    Occur::MustNot,
  )?;
  query.add(
    MultiTermQuerySet::from(PrefixQuery::new(Term::from_text("field1", "term9"))?),
    Occur::Should,
  )?;
  query.add(BoostQuery::new(field2.build(), 2.0)?, Occur::Should)?;
  Ok(query.build().into())
}

#[test]
fn test_extract_terms_equivalent() -> Result<()> {
  let mut terms = HashSet::new();
  let expected = HashSet::from([
    Term::from_text("field1", "t1"),
    Term::from_text("field1", "tm2"),
    Term::from_text("field1", "tm3"),
    Term::from_text("field1", "term4"),
    Term::from_text("field1", "term5"),
    Term::from_text("field2", "term10"),
  ]);
  query()?.visit(&mut term_collector(&mut terms))?;
  assert_eq!(expected, terms);
  Ok(())
}

#[test]
fn extract_all_terms() -> Result<()> {
  let mut terms = HashSet::new();
  let expected = HashSet::from([
    Term::from_text("field1", "t1"),
    Term::from_text("field1", "tm2"),
    Term::from_text("field1", "tm3"),
    Term::from_text("field1", "term4"),
    Term::from_text("field1", "term5"),
    Term::from_text("field1", "term8"),
    Term::from_text("field2", "term10"),
  ]);
  query()?.visit(&mut AllTermsVisitor(&mut terms))?;
  assert_eq!(expected, terms);
  Ok(())
}

#[test]
fn extract_terms_from_field() -> Result<()> {
  let mut actual = HashSet::new();
  let expected = HashSet::from([Term::from_text("field2", "term10")]);
  query()?.visit(&mut FieldTermsVisitor(&mut actual))?;
  assert_eq!(expected, actual);
  Ok(())
}

#[test]
fn test_extract_terms_and_boosts() -> Result<()> {
  let mut terms_to_boosts = HashMap::new();
  query()?.visit(&mut BoostedTermExtractor::Extractor {
    boost: 1.0,
    terms_to_boosts: &mut terms_to_boosts,
  })?;
  let expected = HashMap::from([
    (Term::from_text("field1", "t1"), 1.0),
    (Term::from_text("field1", "tm2"), 1.0),
    (Term::from_text("field1", "tm3"), 2.0),
    (Term::from_text("field1", "term4"), 3.0),
    (Term::from_text("field1", "term5"), 3.0),
    (Term::from_text("field2", "term10"), 6.0),
  ]);
  assert_eq!(expected, terms_to_boosts);
  Ok(())
}

#[test]
fn test_leaf_query_type_counts() -> Result<()> {
  let mut query_counts = HashMap::new();
  query()?.visit(&mut QueryTypeCounter::Counter(&mut query_counts))?;
  assert_eq!(Some(&4), query_counts.get("TermQuery"));
  assert_eq!(Some(&1), query_counts.get("PhraseQuery"));
  Ok(())
}

enum QueryNode {
  Term(Term),
  Conjunction(Vec<QueryNode>),
  Disjunction(Vec<QueryNode>),
}

impl QueryNode {
  fn get_weight(&mut self) -> usize {
    match self {
      Self::Term(term) => term.text().expect("term must be valid UTF-8").len(),
      Self::Conjunction(children) => {
        let mut weighted: Vec<_> = children
          .drain(..)
          .map(|mut child| {
            let weight = child.get_weight();
            (weight, child)
          })
          .collect();
        weighted.sort_by_key(|(weight, _)| *weight);
        *children = weighted.into_iter().map(|(_, child)| child).collect();
        children[0].get_weight()
      },
      Self::Disjunction(children) => {
        let mut weighted: Vec<_> = children
          .drain(..)
          .map(|mut child| {
            let weight = child.get_weight();
            (weight, child)
          })
          .collect();
        weighted.sort_by_key(|(weight, _)| std::cmp::Reverse(*weight));
        *children = weighted.into_iter().map(|(_, child)| child).collect();
        children[0].get_weight()
      },
    }
  }

  fn collect_terms(&mut self, terms: &mut HashSet<Term>) {
    match self {
      Self::Term(term) => {
        terms.insert(term.clone());
      },
      Self::Conjunction(children) => {
        let mut weighted: Vec<_> = children
          .drain(..)
          .map(|mut child| {
            let weight = child.get_weight();
            (weight, child)
          })
          .collect();
        weighted.sort_by_key(|(weight, _)| *weight);
        *children = weighted.into_iter().map(|(_, child)| child).collect();
        children[0].collect_terms(terms);
      },
      Self::Disjunction(children) => {
        for child in children {
          child.collect_terms(terms);
        }
      },
    }
  }

  fn next_term_set(&mut self) -> bool {
    match self {
      Self::Term(_) => false,
      Self::Conjunction(children) => {
        let mut weighted: Vec<_> = children
          .drain(..)
          .map(|mut child| {
            let weight = child.get_weight();
            (weight, child)
          })
          .collect();
        weighted.sort_by_key(|(weight, _)| *weight);
        *children = weighted.into_iter().map(|(_, child)| child).collect();
        if children[0].next_term_set() {
          true
        } else if children.len() == 1 {
          false
        } else {
          children.remove(0);
          true
        }
      },
      Self::Disjunction(children) => {
        let mut next = false;
        for child in children {
          next |= child.next_term_set();
        }
        next
      },
    }
  }
}

impl Display for QueryNode {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Term(term) => write!(f, "TERM({term})"),
      Self::Conjunction(children) | Self::Disjunction(children) => {
        let operator = if matches!(self, Self::Conjunction(_)) {
          "AND"
        } else {
          "OR"
        };
        write!(f, "{operator}(")?;
        for (index, child) in children.iter().enumerate() {
          if index > 0 {
            write!(f, ",")?;
          }
          write!(f, "{child}")?;
        }
        write!(f, ")")
      },
    }
  }
}

enum QueryNodeVisitor<'a> {
  Node(&'a mut QueryNode),
  Empty,
}

#[test]
fn test_extract_matching_term_set() -> Result<()> {
  let mut extractor = QueryNode::Conjunction(Vec::new());
  query()?.visit(&mut QueryNodeVisitor::Node(&mut extractor))?;
  let mut minimum_term_set = HashSet::new();
  extractor.collect_terms(&mut minimum_term_set);

  let expected1 = HashSet::from([Term::from_text("field1", "t1")]);
  assert_eq!(expected1, minimum_term_set);
  assert!(extractor.next_term_set());
  let expected2 = HashSet::from([
    Term::from_text("field1", "tm2"),
    Term::from_text("field1", "tm3"),
  ]);
  minimum_term_set.clear();
  extractor.collect_terms(&mut minimum_term_set);
  assert_eq!(expected2, minimum_term_set);

  let mut inner = BooleanQueryBuilder::new();
  inner.add(TermQuery::new(Term::from_text("f", "1")), Occur::Must)?;
  inner.add(TermQuery::new(Term::from_text("f", "61")), Occur::Must)?;
  inner.add(TermQuery::new(Term::from_text("f", "211")), Occur::Filter)?;
  inner.add(TermQuery::new(Term::from_text("f", "5")), Occur::Should)?;
  let mut outer = BooleanQueryBuilder::new();
  outer.add(inner.build(), Occur::Should)?;
  outer.add(
    PhraseQuery::from_terms_no_slop("f", &["3333", "44444"])?,
    Occur::Should,
  )?;
  let query: BooleanQuery = outer.build();
  let mut extractor2 = QueryNode::Conjunction(Vec::new());
  query.visit(&mut QueryNodeVisitor::Node(&mut extractor2))?;
  let expected3 = HashSet::from([Term::from_text("f", "1"), Term::from_text("f", "3333")]);
  minimum_term_set.clear();
  extractor2.collect_terms(&mut minimum_term_set);
  assert_eq!(expected3, minimum_term_set);
  extractor2.get_weight();
  assert_eq!(
    "AND(AND(OR(AND(TERM(f:3333),TERM(f:44444)),AND(TERM(f:1),TERM(f:61),AND(TERM(f:211))))))",
    extractor2.to_string()
  );
  Ok(())
}

struct AllTermsVisitor<'a>(&'a mut HashSet<Term>);

struct FieldTermsVisitor<'a>(&'a mut HashSet<Term>);

enum BoostedTermExtractor<'a> {
  Extractor {
    boost: f32,
    terms_to_boosts: &'a mut HashMap<Term, f32>,
  },
  Empty,
}

enum QueryTypeCounter<'a> {
  Counter(&'a mut HashMap<&'static str, usize>),
  Empty,
}

impl QueryTypeCounter<'_> {
  fn count_query(&mut self, query: QueryRef<'_>) {
    if let Self::Counter(query_counts) = self {
      let name = match query {
        QueryRef::Term(_) => "TermQuery",
        QueryRef::Phrase(_) => "PhraseQuery",
        _ => return,
      };
      *query_counts.entry(name).or_default() += 1;
    }
  }
}

impl QueryVisitor for AllTermsVisitor<'_> {
  type SubVisitor<'a>
    = &'a mut Self
  where
    Self: 'a;

  fn consume_terms(&mut self, _query: QueryRef<'_>, terms: &[Term]) -> Result<()> {
    self.0.extend(terms.iter().cloned());
    Ok(())
  }

  fn get_sub_visitor<'a>(
    &'a mut self,
    _occur: Occur,
    _parent: QueryRef<'_>,
  ) -> Self::SubVisitor<'a> {
    self
  }
}

impl QueryVisitor for FieldTermsVisitor<'_> {
  type SubVisitor<'a>
    = DefaultQueryVisitor<'a, Self>
  where
    Self: 'a;

  fn consume_terms(&mut self, _query: QueryRef<'_>, terms: &[Term]) -> Result<()> {
    self.0.extend(terms.iter().cloned());
    Ok(())
  }

  fn accept_field(&self, field: &str) -> bool {
    field == "field2"
  }

  fn get_sub_visitor<'a>(&'a mut self, occur: Occur, parent: QueryRef<'_>) -> Self::SubVisitor<'a> {
    self.default_get_sub_visitor(occur, parent)
  }
}

impl<'n> QueryVisitor for BoostedTermExtractor<'n> {
  type SubVisitor<'a>
    = BoostedTermExtractor<'a>
  where
    Self: 'a;

  fn consume_terms(&mut self, _query: QueryRef<'_>, terms: &[Term]) -> Result<()> {
    if let Self::Extractor {
      boost,
      terms_to_boosts,
    } = self
    {
      for term in terms {
        terms_to_boosts.insert(term.clone(), *boost);
      }
    }
    Ok(())
  }

  fn get_sub_visitor<'a>(&'a mut self, occur: Occur, parent: QueryRef<'_>) -> Self::SubVisitor<'a> {
    match self {
      Self::Extractor {
        boost,
        terms_to_boosts,
      } => {
        if let QueryRef::Boost(query) = parent {
          BoostedTermExtractor::Extractor {
            boost: *boost * query.get_boost(),
            terms_to_boosts,
          }
        } else if occur == Occur::MustNot {
          BoostedTermExtractor::Empty
        } else {
          BoostedTermExtractor::Extractor {
            boost: *boost,
            terms_to_boosts,
          }
        }
      },
      Self::Empty => BoostedTermExtractor::Empty,
    }
  }
}

impl<'n> QueryVisitor for QueryTypeCounter<'n> {
  type SubVisitor<'a>
    = QueryTypeCounter<'a>
  where
    Self: 'a;

  fn consume_terms(&mut self, query: QueryRef<'_>, _terms: &[Term]) -> Result<()> {
    self.count_query(query);
    Ok(())
  }

  fn visit_leaf(&mut self, query: QueryRef<'_>) -> Result<()> {
    self.count_query(query);
    Ok(())
  }

  fn get_sub_visitor<'a>(
    &'a mut self,
    occur: Occur,
    _parent: QueryRef<'_>,
  ) -> Self::SubVisitor<'a> {
    match self {
      Self::Counter(query_counts) if occur != Occur::MustNot => {
        QueryTypeCounter::Counter(query_counts)
      },
      Self::Counter(_) | Self::Empty => QueryTypeCounter::Empty,
    }
  }
}

impl<'n> QueryVisitor for QueryNodeVisitor<'n> {
  type SubVisitor<'a>
    = QueryNodeVisitor<'a>
  where
    Self: 'a;

  fn consume_terms(&mut self, _query: QueryRef<'_>, terms: &[Term]) -> Result<()> {
    if let Self::Node(node) = self {
      match node {
        QueryNode::Conjunction(children) | QueryNode::Disjunction(children) => {
          children.extend(terms.iter().cloned().map(QueryNode::Term));
        },
        QueryNode::Term(_) => unreachable!(),
      }
    }
    Ok(())
  }

  fn get_sub_visitor<'a>(&'a mut self, occur: Occur, parent: QueryRef<'_>) -> Self::SubVisitor<'a> {
    let Self::Node(node) = self else {
      return QueryNodeVisitor::Empty;
    };
    if occur == Occur::Must || occur == Occur::Filter {
      let children = match node {
        QueryNode::Conjunction(children) | QueryNode::Disjunction(children) => children,
        QueryNode::Term(_) => unreachable!(),
      };
      children.push(QueryNode::Conjunction(Vec::new()));
      return QueryNodeVisitor::Node(children.last_mut().unwrap());
    }
    if occur == Occur::MustNot {
      return QueryNodeVisitor::Empty;
    }
    if let QueryRef::Boolean(query) = parent
      && (!query.get_clauses_idx(Occur::Must).is_empty()
        || !query.get_clauses_idx(Occur::Filter).is_empty())
    {
      return QueryNodeVisitor::Empty;
    }
    let children = match node {
      QueryNode::Conjunction(children) | QueryNode::Disjunction(children) => children,
      QueryNode::Term(_) => unreachable!(),
    };
    children.push(QueryNode::Disjunction(Vec::new()));
    QueryNodeVisitor::Node(children.last_mut().unwrap())
  }
}
