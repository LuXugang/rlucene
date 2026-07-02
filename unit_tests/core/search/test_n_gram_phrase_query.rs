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
use crate::core::index::directory_reader;
use crate::core::index::term::Term;
use crate::core::search::n_gram_phrase_query::NGramPhraseQuery;
use crate::core::search::phrase_query::PhraseQuery;
use crate::core::search::query::{Query, QueryBase};
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::util::DefaultIndexSearchCR;
use crate::test_framework::core::util::lucene_test_case::{
  new_directory_shared, new_searcher_with_reader, random,
};
use rand::Rng;
#[allow(dead_code)] // for quick search
struct TestNGramPhraseQuery;
fn set_up<R>(random: &mut R) -> Result<DefaultIndexSearchCR>
where
  R: Rng + ?Sized,
{
  let directory = new_directory_shared(random)?;
  let writer = RandomIndexWriter::new(random, directory.clone())?;
  writer.close(random)?;

  let reader = directory_reader::open(directory.clone())?;
  let searcher = new_searcher_with_reader(reader)?;

  Ok(searcher)
}
#[test]
fn test_rewrite() -> Result<()> {
  let mut random = random();
  let searcher = set_up(&mut random)?;

  // bi-gram test ABC => AB/BC => AB/BC
  let pq1 = NGramPhraseQuery::new(2, PhraseQuery::from_terms_no_slop("f", &["AB", "BC"])?);

  let q = pq1.rewrite(&searcher)?;
  assert_eq!(q.clone().rewrite(&searcher)?, q);
  let Query::Phrase(rewritten1) = q else {
    panic!("expected PhraseQuery");
  };
  assert_eq!(
    &vec![Term::from_text("f", "AB"), Term::from_text("f", "BC")],
    rewritten1.get_terms()
  );
  assert_eq!(&vec![0, 1], rewritten1.get_positions());

  // bi-gram test ABCD => AB/BC/CD => AB//CD
  let pq2 = NGramPhraseQuery::new(
    2,
    PhraseQuery::from_terms_no_slop("f", &["AB", "BC", "CD"])?,
  );

  let q = pq2.rewrite(&searcher)?;
  assert!(matches!(q, Query::Phrase(_)));
  let Query::Phrase(rewritten2) = q else {
    panic!("expected PhraseQuery");
  };
  assert_eq!(
    &vec![Term::from_text("f", "AB"), Term::from_text("f", "CD")],
    rewritten2.get_terms()
  );
  assert_eq!(&vec![0, 2], rewritten2.get_positions());

  // tri-gram test ABCDEFGH => ABC/BCD/CDE/DEF/EFG/FGH => ABC///DEF//FGH
  let pq3 = NGramPhraseQuery::new(
    3,
    PhraseQuery::from_terms_no_slop("f", &["ABC", "BCD", "CDE", "DEF", "EFG", "FGH"])?,
  );

  let q = pq3.rewrite(&searcher)?;
  assert!(matches!(q, Query::Phrase(_)));
  let Query::Phrase(rewritten3) = q else {
    panic!("expected PhraseQuery");
  };
  assert_eq!(
    &vec![
      Term::from_text("f", "ABC"),
      Term::from_text("f", "DEF"),
      Term::from_text("f", "FGH"),
    ],
    rewritten3.get_terms()
  );
  assert_eq!(&vec![0, 3, 5], rewritten3.get_positions());

  Ok(())
}
