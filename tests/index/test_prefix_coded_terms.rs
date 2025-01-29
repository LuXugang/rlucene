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
use crate::util::lucene_test_case::{at_least, random};
use crate::util::test_error::TestError;
use crate::util::TestUtil;
use rlucene::index::field_term_iterator::FieldTermIterator;
use rlucene::index::prefix_coded_terms::PrefixCodedTermsBuilder;
use rlucene::index::term::Term;
use rlucene::util::bytes_ref_iterator::BytesRefIterator;
use std::collections::BTreeSet;

#[allow(dead_code)] // for quick search
pub struct TestPrefixCodedTerms;

#[test]
fn test_empty() -> Result<(), TestError> {
    let mut builder = PrefixCodedTermsBuilder::new()?;
    let prefix_coded_terms = builder.finish()?;
    let mut iter = prefix_coded_terms.iterator();
    assert!(iter.next()?.is_none());
    Ok(())
}
#[test]
fn test_one() -> Result<(), TestError> {
    let term = Term::from_text("foo".to_string(), "bogus");
    let mut builder = PrefixCodedTermsBuilder::new()?;
    builder.add_term(&term)?;
    let prefix_coded_terms = builder.finish()?;
    let mut iter = prefix_coded_terms.iterator();
    let first_term = iter.next()?.expect("Expected a term, but got None");
    assert_eq!(iter.field(), "foo");
    assert_eq!(first_term.utf8_to_string()?, "bogus");
    assert!(iter.next()?.is_none());

    Ok(())
}

#[test]
fn test_random() -> Result<(), TestError> {
    let mut random = random();
    let mut terms = BTreeSet::new();
    let nterms = at_least(&mut random, 10_000);

    for _ in 0..nterms {
        let field = TestUtil::random_unicode_string_with_length(&mut random, 2);
        let text = TestUtil::random_unicode_string_with_length(&mut random, 0);
        let term = Term::from_text(field, &text);
        terms.insert(term);
    }
    let mut builder = PrefixCodedTermsBuilder::new()?;
    for term in &terms {
        builder.add_term(term)?;
    }
    let pb = builder.finish()?;
    let mut iter = pb.iterator();
    let mut expected = terms.iter();

    assert_eq!(terms.len(), pb.size() as usize);

    while let Some(actual_bytes) = iter.next()? {
        let expected_term = expected.next();
        assert!(expected_term.is_some());
        let actual_term = Term::new(iter.field().to_string(), actual_bytes.clone());

        assert_eq!(*expected_term.unwrap(), actual_term);
    }
    assert!(expected.next().is_none());
    Ok(())
}
