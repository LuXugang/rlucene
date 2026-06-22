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
use crate::test::core::util::lucene_test_case::{at_least, random};
use std::collections::BTreeSet;

use crate::core::index::field_term_iterator::FieldTermIterator;
use crate::core::index::prefix_coded_terms::PrefixCodedTermsBuilder;
use crate::core::index::term::Term;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::util::test_util::TestUtil;

#[allow(dead_code)] // for quick search
struct TestPrefixCodedTerms;

#[test]
fn test_empty() -> Result<()> {
  let mut builder = PrefixCodedTermsBuilder::new();
  let prefix_coded_terms = builder.finish();
  let mut iter = prefix_coded_terms.iterator()?;
  assert!(iter.next()?.is_none());
  Ok(())
}
#[test]
fn test_one() -> Result<()> {
  let term = Term::from_text("foo".to_string(), "bogus");
  let mut builder = PrefixCodedTermsBuilder::new();
  builder.add_term(&term)?;
  let prefix_coded_terms = builder.finish();
  let mut iter = prefix_coded_terms.iterator()?;
  let first_term = iter.next()?.expect("Expected a term, but got None");
  assert_eq!(first_term.utf8_to_string()?, "bogus");
  assert_eq!(iter.field(), "foo");
  assert!(iter.next()?.is_none());

  Ok(())
}

#[test]
fn test_random() -> Result<()> {
  let mut random = random();
  let mut terms = BTreeSet::new();
  let nterms = at_least(&mut random, 10_000);

  for _ in 0..nterms {
    let field = TestUtil::random_unicode_string_with_len(&mut random, 2);
    let text = TestUtil::random_unicode_string(&mut random);
    let term = Term::from_text(field, &text);
    terms.insert(term);
  }
  let mut builder = PrefixCodedTermsBuilder::new();
  for term in &terms {
    builder.add_term(term)?;
  }
  let pb = builder.finish();
  let mut iter = pb.iterator()?;
  let mut expected = terms.iter();

  assert_eq!(terms.len(), pb.size() as usize);

  while let Some(actual_bytes) = iter.next()? {
    let actual_bytes = actual_bytes.into_owned();
    let expected_term = expected.next();
    assert!(expected_term.is_some());
    let actual_term = Term::new(iter.field().to_string(), actual_bytes);

    assert_eq!(*expected_term.unwrap(), actual_term);
  }
  assert!(expected.next().is_none());
  Ok(())
}
