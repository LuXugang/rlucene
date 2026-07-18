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
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};

use crate::core::analysis::analyzer::Analyzer;
use crate::core::analysis::reader::{ReaderEnum, StringReader};
use crate::core::document::document::Document;
use crate::core::document::field::{FieldDataEnum, IndexingTokenStreamEnum3, Store};
use crate::core::document::field_type::FieldType;
use crate::core::document::invertable_field::InvertableType;
use crate::core::document::stored_field::stored_field_type;
use crate::core::index::BytesRef;
use crate::core::index::doc_values_skip_index_type::DocValuesSkipIndexType;
use crate::core::index::doc_values_type::DocValuesType;
use crate::core::index::fields::Fields as FieldsTrait;
use crate::core::index::index_options::IndexOptions;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::indexable_field::{
  IndexableField, IndexingTokenStream, ReusedIndexingTokenStream,
};
use crate::core::index::indexable_field_type::{IndexableFieldType, IndexableFieldTypeEnum};
use crate::core::index::postings_enum::{ALL, PostingsEnum};
use crate::core::index::stored_fields::StoredFields as StoredFieldsTrait;
use crate::core::index::term::Term;
use crate::core::index::term_vectors::TermVectors;
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::TermsEnum;
use crate::core::index::vector_encoding::VectorEncoding;
use crate::core::index::vector_similarity_function::VectorSimilarityFunction;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder as BooleanQueryBuilder;
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, NO_MORE_DOCS};
use crate::core::search::term_query::TermQuery;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::number::Number;
pub use crate::test_framework::core::document::{CustomField, MyField};
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::util::lucene_test_case::{
  at_least, new_directory_shared, new_searcher_with_reader, new_string_field, random,
};
use crate::test_framework::core::util::test_util::TestUtil;

#[allow(dead_code)] // for quick search
pub(crate) struct TestIndexableField;

// Silly test showing how to index documents w/o using Lucene's core
// Document nor Field struct
#[test]
fn test_arbitrary_fields() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = RandomIndexWriter::new(&mut random, dir.clone())?;

  let num_docs = at_least(&mut random, 27) as usize;
  if cfg!(feature = "test_log_verbose") {
    println!("TEST: {num_docs} docs");
  }
  let mut fields_per_doc = vec![0usize; num_docs];
  let mut base_count = 0usize;
  let mut field_to_type = HashMap::new();

  for (doc_count, fields_in_doc) in fields_per_doc.iter_mut().enumerate().take(num_docs) {
    let field_count = TestUtil::next_int(&mut random, 1, 17) as usize;
    *fields_in_doc = field_count - 1;

    if cfg!(feature = "test_log_verbose") {
      println!("TEST: {field_count} fields in doc {doc_count}");
    }

    let final_base_count = base_count;
    base_count += field_count - 1;

    let mut d = Document::new();
    d.add(new_string_field(
      &mut random,
      "id",
      doc_count.to_string(),
      Store::Yes,
      &mut field_to_type,
    )?);
    for field_upto in 1..field_count {
      d.add(MyField::new((final_base_count + (field_upto - 1)) as i32)?);
    }
    w.add_document(&mut random, d)?;
  }

  let r = w.get_reader(&mut random)?;
  w.close(&mut random)?;

  let mut term_vectors = r.term_vectors()?;
  let s = new_searcher_with_reader(r)?;
  let mut stored_fields = s.stored_fields()?;
  let mut counter = 0;
  for (id, fields_in_doc) in fields_per_doc.iter().enumerate() {
    if cfg!(feature = "test_log_verbose") {
      println!("TEST: verify doc id={id} ({fields_in_doc} fields) counter={counter}");
    }

    let hits = s.search(TermQuery::new(Term::from_text("id", id.to_string())), 1)?;
    assert_eq!(1, hits.total_hits.value());
    let doc_id = hits.score_docs[0].doc;
    let doc = stored_fields.document(doc_id)?;
    let end_counter = counter + *fields_in_doc as i32;
    while counter < end_counter {
      let name = format!("f{counter}");
      let field_id = counter % 10;

      let stored = (counter & 1) == 0 || field_id == 3;
      let binary = field_id == 3;
      let indexed = field_id != 3;

      let string_value = if field_id != 3 && field_id != 7 {
        Some(format!("text {counter}"))
      } else {
        None
      };

      // stored:
      if stored {
        let f = doc
          .get_field(&name)
          .unwrap_or_else(|| panic!("doc {id} doesn't have field f{counter}"));
        if binary {
          let b = f.binary_value()?.unwrap();
          assert_eq!(10, b.length);
          for idx in 0..10 {
            assert_eq!((idx as i32 + counter) as u8, b.bytes[b.offset + idx]);
          }
        } else {
          let actual = f.string_value()?.unwrap();
          assert_eq!(string_value.unwrap().as_str(), actual.as_ref().as_str());
        }
      }

      if indexed {
        let tv = counter % 2 == 1 && field_id != 9;
        if tv {
          let tfv = term_vectors.get(doc_id)?.unwrap().terms(&name)?.unwrap();
          let mut terms_enum = tfv.iterator()?;
          assert_eq!(
            BytesRef::from_string(&counter.to_string()),
            terms_enum.next()?.unwrap().into_owned()
          );
          assert_eq!(1, terms_enum.total_term_freq()?);
          let mut dp_enum = terms_enum.postings_with_flags(None, ALL as i32)?;
          assert_ne!(NO_MORE_DOCS, dp_enum.next_doc()?);
          assert_eq!(1, dp_enum.freq()?);
          assert_eq!(1, dp_enum.next_position()?);

          assert_eq!(
            BytesRef::from_string("text"),
            terms_enum.next()?.unwrap().into_owned()
          );
          assert_eq!(1, terms_enum.total_term_freq()?);
          let mut dp_enum = terms_enum.postings_with_flags(Some(dp_enum), ALL as i32)?;
          assert_ne!(NO_MORE_DOCS, dp_enum.next_doc()?);
          assert_eq!(1, dp_enum.freq()?);
          assert_eq!(0, dp_enum.next_position()?);

          assert!(terms_enum.next()?.is_none());

          // TODO: offsets
        } else {
          let vectors = term_vectors.get(doc_id)?;
          assert!(vectors.is_none() || vectors.unwrap().terms(&name)?.is_none());
        }

        let mut bq = BooleanQueryBuilder::new();
        bq.add(
          TermQuery::new(Term::from_text("id", id.to_string())),
          Occur::Must,
        )?;
        bq.add(TermQuery::new(Term::from_text(&name, "text")), Occur::Must)?;
        let hits2 = s.search(bq.build(), 1)?;
        assert_eq!(1, hits2.total_hits.value());
        assert_eq!(doc_id, hits2.score_docs[0].doc);

        let mut bq = BooleanQueryBuilder::new();
        bq.add(
          TermQuery::new(Term::from_text("id", id.to_string())),
          Occur::Must,
        )?;
        bq.add(
          TermQuery::new(Term::from_text(&name, counter.to_string())),
          Occur::Must,
        )?;
        let hits3 = s.search(bq.build(), 1)?;
        assert_eq!(1, hits3.total_hits.value());
        assert_eq!(doc_id, hits3.score_docs[0].doc);
      }

      counter += 1;
    }
  }

  Ok(())
}

// LUCENE-5611
#[test]
fn test_not_indexed_term_vectors() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = RandomIndexWriter::new(&mut random, dir)?;
  let result = w.add_document(&mut random, vec![CustomField::new()?.into()]);
  assert!(matches!(result, Err(LuceneError::IllegalArgument(_))));
  w.close(&mut random)?;
  Ok(())
}
