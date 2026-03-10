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
use crate::core::search::boolean_query::Builder;
use crate::core::search::phrase_query::PhraseQuery;
use crate::core::search::query::Query;
use crate::core::search::term_query::TermQuery;
use crate::core::search::wildcard_query::WildcardQuery;
use crate::core::util::error::lucene_error::Result;
use crate::test::search::test_boolean_min_should_match::Callback;
use rand::{Rng, RngExt};

#[allow(dead_code)] // for quick search
pub struct TestBoolean2;

pub(crate) fn rand_bool_query<R: Rng + ?Sized, C: Callback>(
    rnd: &mut R,
    allow_must: bool,
    level: i32,
    field: &str,
    vals: &[String],
    cb: Option<&C>,
) -> Result<Builder> {
    let mut current = Builder::new();

    for _ in 0..(rnd.random_range(0..vals.len()) + 1) {
        let mut q_type = 0;
        if level > 0 {
            q_type = rnd.random_range(0..10);
        }

        let q: Query = if q_type < 3 {
            TermQuery::new(Term::from_text(
                field,
                &vals[rnd.random_range(0..vals.len())],
            ))
            .into()
        } else if q_type < 4 {
            let t1 = &vals[rnd.random_range(0..vals.len())];
            let t2 = &vals[rnd.random_range(0..vals.len())];
            PhraseQuery::from_terms(10, field, &[t1.as_str(), t2.as_str()])?.into()
        } else if q_type < 7 {
            WildcardQuery::new(Term::from_text(field, "w*"))?.into()
        } else {
            rand_bool_query(rnd, allow_must, level - 1, field, vals, cb)?
                .build()
                .into()
        };

        let r = rnd.random_range(0..10);
        let occur = if r < 2 {
            Occur::MustNot
        } else if r < 5 {
            if allow_must {
                Occur::Must
            } else {
                Occur::Should
            }
        } else {
            Occur::Should
        };

        current.add(q, occur)?;
    }

    if let Some(cb) = cb {
        cb.post_create(rnd, &mut current)?;
    }

    Ok(current)
}
