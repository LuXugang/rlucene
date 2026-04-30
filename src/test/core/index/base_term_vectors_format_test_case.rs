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
use crate::core::analysis::token_attributes::payload_attribute::PayloadAttribute;
use crate::core::document::document::Document;
use crate::core::document::field::Field;
use crate::core::document::field_type::FieldType;
use crate::core::document::fields::FieldTokenStreamEnum;
use crate::core::document::text_field::text_field_type;
use crate::core::index::BytesRef;
use crate::core::index::directory_reader;
use crate::core::index::fields::Fields;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::postings_enum::{
  ALL, FREQS, NONE, OFFSETS, PAYLOADS, POSITIONS, PostingsEnum,
};
use crate::core::index::term_vectors::TermVectors;
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::TermsEnum;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::search::sort::Sort;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::iterator::IteratorExt;
use crate::test::core::analysis::canned_token_stream::CannedTokenStream;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::analysis::token;
use crate::test::core::index::base_index_file_format_test_case::BaseIndexFileFormatTestCase;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  get_only_leaf_reader, new_directory_shared, new_index_writer_config,
  new_index_writer_config_with_analyzer,
};
use rand::Rng;

pub trait BaseTermVectorsFormatTestCase: BaseIndexFileFormatTestCase {
  fn test_rare_vectors<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    // TODO RandomDocumentFactory未实现
    Ok(())
  }

  fn test_high_freqs<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    // TODO RandomDocumentFactory未实现
    Ok(())
  }

  fn test_lots_of_fields<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    // TODO RandomDocumentFactory未实现
    Ok(())
  }

  fn test_mixed_options<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    // TODO RandomDocumentFactory未实现
    Ok(())
  }

  fn test_random<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    // TODO RandomDocumentFactory未实现
    Ok(())
  }
  fn do_test_merge<R>(
    &self,
    _random: &mut R,
    _sort: Option<Sort>,
    _allow_deletes: bool,
  ) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    // TODO RandomDocumentFactory未实现
    Ok(())
  }

  fn test_merge<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    self.do_test_merge(random, None, false)
  }

  fn test_merge_with_deletes<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    self.do_test_merge(random, None, true)
  }

  fn test_merge_with_index_sort<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    // TODO RandomDocumentFactory未实现
    Ok(())
  }

  fn test_merge_with_index_sort_and_deletes<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    // TODO RandomDocumentFactory未实现
    Ok(())
  }

  fn test_clone<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    // TODO RandomDocumentFactory未实现
    Ok(())
  }
  fn test_postings_enum_freqs<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(random)?;
    let analyzer = MockAnalyzer::new(random);
    // TODO TokenStreamComponents未实现
    let iwc = new_index_writer_config_with_analyzer(random, analyzer);
    let iw = IndexWriter::new(dir.clone(), iwc)?;

    let mut doc = Document::new();
    let mut ft = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
    ft.set_store_term_vectors(true)?;
    doc.add(Field::new("foo", "bar bar", ft));
    iw.add_document(doc)?;

    let reader = directory_reader::open_from_writer(&iw)?;

    let leaf = get_only_leaf_reader(&reader)?;
    let mut term_vectors = leaf.term_vectors()?;
    let terms = term_vectors.get_field_terms(0, "foo")?.unwrap();
    let mut terms_enum = terms.iterator()?;
    assert_eq!(
      &BytesRef::from_string("bar"),
      terms_enum.next()?.unwrap().as_ref()
    );
    // simple use (FREQS)
    let mut postings = terms_enum.postings(None)?;
    assert_eq!(-1, postings.doc_id());
    assert_eq!(0, postings.next_doc()?);
    assert_eq!(2, postings.freq()?);
    assert_eq!(NO_MORE_DOCS, postings.next_doc()?);

    let mut postings2 = terms_enum.postings(Some(postings))?;
    assert_eq!(-1, postings2.doc_id());
    assert_eq!(0, postings2.next_doc()?);
    assert_eq!(2, postings2.freq()?);
    assert_eq!(NO_MORE_DOCS, postings2.next_doc()?);
    // asking for docs only: ok
    let mut docs_only = terms_enum.postings_with_flags(None, NONE as i32)?;
    assert_eq!(-1, docs_only.doc_id());
    assert_eq!(0, docs_only.next_doc()?);
    assert!(docs_only.freq()? == 1 || docs_only.freq()? == 2);
    assert_eq!(NO_MORE_DOCS, docs_only.next_doc()?);
    // reuse that too
    let mut docs_only2 = terms_enum.postings_with_flags(Some(docs_only), NONE as i32)?;
    // and it had better work
    assert_eq!(-1, docs_only2.doc_id());
    assert_eq!(0, docs_only2.next_doc()?);
    // we don't define what it is, but if its something else, we should look into it?
    assert!(docs_only2.freq()? == 1 || docs_only2.freq()? == 2);
    assert_eq!(NO_MORE_DOCS, docs_only2.next_doc()?);

    for flag in [NONE, FREQS, POSITIONS, PAYLOADS, OFFSETS, ALL] {
      let mut postings = terms_enum.postings_with_flags(None, flag as i32)?;
      assert_eq!(-1, postings.doc_id());
      assert_eq!(0, postings.next_doc()?);
      if flag != NONE {
        assert_eq!(2, postings.freq()?);
      }
      assert_eq!(NO_MORE_DOCS, postings.next_doc()?);

      let mut postings2 = terms_enum.postings_with_flags(Some(postings), flag as i32)?;
      assert_eq!(-1, postings2.doc_id());
      assert_eq!(0, postings2.next_doc()?);
      if flag != NONE {
        assert_eq!(2, postings2.freq()?);
      }
      assert_eq!(NO_MORE_DOCS, postings2.next_doc()?);
    }
    reader.close()?;
    iw.close()?;
    Ok(())
  }
  fn test_postings_enum_positions<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(random)?;
    let analyzer = MockAnalyzer::new(random);
    // TODO TokenStreamComponents未实现
    let iwc = new_index_writer_config_with_analyzer(random, analyzer);
    let iw = IndexWriter::new(dir.clone(), iwc)?;

    let mut doc = Document::new();
    let mut ft = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
    ft.set_store_term_vectors(true)?;
    ft.set_store_term_vector_positions(true)?;
    doc.add(Field::new("foo", "bar bar", ft));
    iw.add_document(doc)?;

    let reader = directory_reader::open_from_writer(&iw)?;

    let leaf = get_only_leaf_reader(&reader)?;
    let mut term_vectors = leaf.term_vectors()?;
    let terms = term_vectors.get_field_terms(0, "foo")?.unwrap();
    let mut terms_enum = terms.iterator()?;
    assert_eq!(
      &BytesRef::from_string("bar"),
      terms_enum.next()?.unwrap().as_ref()
    );

    let mut postings = terms_enum.postings(None)?;
    assert_eq!(-1, postings.doc_id());
    assert_eq!(0, postings.next_doc()?);
    assert_eq!(2, postings.freq()?);
    assert_eq!(NO_MORE_DOCS, postings.next_doc()?);

    let mut postings2 = terms_enum.postings(Some(postings))?;
    assert_eq!(-1, postings2.doc_id());
    assert_eq!(0, postings2.next_doc()?);
    assert_eq!(2, postings2.freq()?);
    assert_eq!(NO_MORE_DOCS, postings2.next_doc()?);

    let mut docs_only = terms_enum.postings_with_flags(None, NONE as i32)?;
    assert_eq!(-1, docs_only.doc_id());
    assert_eq!(0, docs_only.next_doc()?);
    assert!(docs_only.freq()? == 1 || docs_only.freq()? == 2);
    assert_eq!(NO_MORE_DOCS, docs_only.next_doc()?);

    let mut docs_only2 = terms_enum.postings_with_flags(Some(docs_only), NONE as i32)?;
    assert_eq!(-1, docs_only2.doc_id());
    assert_eq!(0, docs_only2.next_doc()?);
    assert!(docs_only2.freq()? == 1 || docs_only2.freq()? == 2);
    assert_eq!(NO_MORE_DOCS, docs_only2.next_doc()?);

    let mut docs_and_positions_enum = terms_enum.postings_with_flags(None, POSITIONS as i32)?;
    assert_eq!(-1, docs_and_positions_enum.doc_id());
    assert_eq!(0, docs_and_positions_enum.next_doc()?);
    assert_eq!(2, docs_and_positions_enum.freq()?);
    assert_eq!(0, docs_and_positions_enum.next_position()?);
    assert_eq!(-1, docs_and_positions_enum.start_offset()?);
    assert_eq!(-1, docs_and_positions_enum.end_offset()?);
    assert!(docs_and_positions_enum.get_payload()?.is_none());
    assert_eq!(1, docs_and_positions_enum.next_position()?);
    assert_eq!(-1, docs_and_positions_enum.start_offset()?);
    assert_eq!(-1, docs_and_positions_enum.end_offset()?);
    assert!(docs_and_positions_enum.get_payload()?.is_none());
    assert_eq!(NO_MORE_DOCS, docs_and_positions_enum.next_doc()?);

    let mut docs_and_positions_enum2 =
      terms_enum.postings_with_flags(Some(docs_and_positions_enum), POSITIONS as i32)?;
    assert_eq!(-1, docs_and_positions_enum2.doc_id());
    assert_eq!(0, docs_and_positions_enum2.next_doc()?);
    assert_eq!(2, docs_and_positions_enum2.freq()?);
    assert_eq!(0, docs_and_positions_enum2.next_position()?);
    assert_eq!(-1, docs_and_positions_enum2.start_offset()?);
    assert_eq!(-1, docs_and_positions_enum2.end_offset()?);
    assert!(docs_and_positions_enum2.get_payload()?.is_none());
    assert_eq!(1, docs_and_positions_enum2.next_position()?);
    assert_eq!(-1, docs_and_positions_enum2.start_offset()?);
    assert_eq!(-1, docs_and_positions_enum2.end_offset()?);
    assert!(docs_and_positions_enum2.get_payload()?.is_none());
    assert_eq!(NO_MORE_DOCS, docs_and_positions_enum2.next_doc()?);

    let mut docs_and_positions_enum = terms_enum.postings_with_flags(None, PAYLOADS as i32)?;
    assert_eq!(-1, docs_and_positions_enum.doc_id());
    assert_eq!(0, docs_and_positions_enum.next_doc()?);
    assert_eq!(2, docs_and_positions_enum.freq()?);
    assert_eq!(0, docs_and_positions_enum.next_position()?);
    assert_eq!(-1, docs_and_positions_enum.start_offset()?);
    assert_eq!(-1, docs_and_positions_enum.end_offset()?);
    assert!(docs_and_positions_enum.get_payload()?.is_none());
    assert_eq!(1, docs_and_positions_enum.next_position()?);
    assert_eq!(-1, docs_and_positions_enum.start_offset()?);
    assert_eq!(-1, docs_and_positions_enum.end_offset()?);
    assert!(docs_and_positions_enum.get_payload()?.is_none());
    assert_eq!(NO_MORE_DOCS, docs_and_positions_enum.next_doc()?);

    let mut docs_and_positions_enum2 =
      terms_enum.postings_with_flags(Some(docs_and_positions_enum), PAYLOADS as i32)?;
    assert_eq!(-1, docs_and_positions_enum2.doc_id());
    assert_eq!(0, docs_and_positions_enum2.next_doc()?);
    assert_eq!(2, docs_and_positions_enum2.freq()?);
    assert_eq!(0, docs_and_positions_enum2.next_position()?);
    assert_eq!(-1, docs_and_positions_enum2.start_offset()?);
    assert_eq!(-1, docs_and_positions_enum2.end_offset()?);
    assert!(docs_and_positions_enum2.get_payload()?.is_none());
    assert_eq!(1, docs_and_positions_enum2.next_position()?);
    assert_eq!(-1, docs_and_positions_enum2.start_offset()?);
    assert_eq!(-1, docs_and_positions_enum2.end_offset()?);
    assert!(docs_and_positions_enum2.get_payload()?.is_none());
    assert_eq!(NO_MORE_DOCS, docs_and_positions_enum2.next_doc()?);

    let mut docs_and_positions_enum = terms_enum.postings_with_flags(None, OFFSETS as i32)?;
    assert_eq!(-1, docs_and_positions_enum.doc_id());
    assert_eq!(0, docs_and_positions_enum.next_doc()?);
    assert_eq!(2, docs_and_positions_enum.freq()?);
    assert_eq!(0, docs_and_positions_enum.next_position()?);
    assert_eq!(-1, docs_and_positions_enum.start_offset()?);
    assert_eq!(-1, docs_and_positions_enum.end_offset()?);
    assert!(docs_and_positions_enum.get_payload()?.is_none());
    assert_eq!(1, docs_and_positions_enum.next_position()?);
    assert_eq!(-1, docs_and_positions_enum.start_offset()?);
    assert_eq!(-1, docs_and_positions_enum.end_offset()?);
    assert!(docs_and_positions_enum.get_payload()?.is_none());
    assert_eq!(NO_MORE_DOCS, docs_and_positions_enum.next_doc()?);

    let mut docs_and_positions_enum2 =
      terms_enum.postings_with_flags(Some(docs_and_positions_enum), OFFSETS as i32)?;
    assert_eq!(-1, docs_and_positions_enum2.doc_id());
    assert_eq!(0, docs_and_positions_enum2.next_doc()?);
    assert_eq!(2, docs_and_positions_enum2.freq()?);
    assert_eq!(0, docs_and_positions_enum2.next_position()?);
    assert_eq!(-1, docs_and_positions_enum2.start_offset()?);
    assert_eq!(-1, docs_and_positions_enum2.end_offset()?);
    assert!(docs_and_positions_enum2.get_payload()?.is_none());
    assert_eq!(1, docs_and_positions_enum2.next_position()?);
    assert_eq!(-1, docs_and_positions_enum2.start_offset()?);
    assert_eq!(-1, docs_and_positions_enum2.end_offset()?);
    assert!(docs_and_positions_enum2.get_payload()?.is_none());
    assert_eq!(NO_MORE_DOCS, docs_and_positions_enum2.next_doc()?);

    let mut docs_and_positions_enum = terms_enum.postings_with_flags(None, ALL as i32)?;
    assert_eq!(-1, docs_and_positions_enum.doc_id());
    assert_eq!(0, docs_and_positions_enum.next_doc()?);
    assert_eq!(2, docs_and_positions_enum.freq()?);
    assert_eq!(0, docs_and_positions_enum.next_position()?);
    assert_eq!(-1, docs_and_positions_enum.start_offset()?);
    assert_eq!(-1, docs_and_positions_enum.end_offset()?);
    assert!(docs_and_positions_enum.get_payload()?.is_none());
    assert_eq!(1, docs_and_positions_enum.next_position()?);
    assert_eq!(-1, docs_and_positions_enum.start_offset()?);
    assert_eq!(-1, docs_and_positions_enum.end_offset()?);
    assert!(docs_and_positions_enum.get_payload()?.is_none());
    assert_eq!(NO_MORE_DOCS, docs_and_positions_enum.next_doc()?);

    let mut docs_and_positions_enum2 =
      terms_enum.postings_with_flags(Some(docs_and_positions_enum), ALL as i32)?;
    assert_eq!(-1, docs_and_positions_enum2.doc_id());
    assert_eq!(0, docs_and_positions_enum2.next_doc()?);
    assert_eq!(2, docs_and_positions_enum2.freq()?);
    assert_eq!(0, docs_and_positions_enum2.next_position()?);
    assert_eq!(-1, docs_and_positions_enum2.start_offset()?);
    assert_eq!(-1, docs_and_positions_enum2.end_offset()?);
    assert!(docs_and_positions_enum2.get_payload()?.is_none());
    assert_eq!(1, docs_and_positions_enum2.next_position()?);
    assert_eq!(-1, docs_and_positions_enum2.start_offset()?);
    assert_eq!(-1, docs_and_positions_enum2.end_offset()?);
    assert!(docs_and_positions_enum2.get_payload()?.is_none());
    assert_eq!(NO_MORE_DOCS, docs_and_positions_enum2.next_doc()?);

    reader.close()?;
    iw.close()?;
    Ok(())
  }
  fn test_postings_enum_offsets<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(random)?;
    let analyzer = MockAnalyzer::new(random);
    // TODO TokenStreamComponents未实现
    let iwc = new_index_writer_config_with_analyzer(random, analyzer);
    let iw = IndexWriter::new(dir.clone(), iwc)?;

    let mut doc = Document::new();
    let mut ft = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
    ft.set_store_term_vectors(true)?;
    ft.set_store_term_vector_positions(true)?;
    ft.set_store_term_vector_offsets(true)?;
    doc.add(Field::new("foo", "bar bar", ft));
    iw.add_document(doc)?;

    let reader = directory_reader::open_from_writer(&iw)?;

    let leaf = get_only_leaf_reader(&reader)?;
    let mut term_vectors = leaf.term_vectors()?;
    let terms = term_vectors.get_field_terms(0, "foo")?.unwrap();
    let mut terms_enum = terms.iterator()?;
    assert_eq!(
      &BytesRef::from_string("bar"),
      terms_enum.next()?.unwrap().as_ref()
    );

    let mut postings = terms_enum.postings(None)?;
    assert_eq!(-1, postings.doc_id());
    assert_eq!(0, postings.next_doc()?);
    assert_eq!(2, postings.freq()?);
    assert_eq!(NO_MORE_DOCS, postings.next_doc()?);

    let mut postings2 = terms_enum.postings(Some(postings))?;
    assert_eq!(-1, postings2.doc_id());
    assert_eq!(0, postings2.next_doc()?);
    assert_eq!(2, postings2.freq()?);
    assert_eq!(NO_MORE_DOCS, postings2.next_doc()?);

    let mut docs_only = terms_enum.postings_with_flags(None, NONE as i32)?;
    assert_eq!(-1, docs_only.doc_id());
    assert_eq!(0, docs_only.next_doc()?);
    assert!(docs_only.freq()? == 1 || docs_only.freq()? == 2);
    assert_eq!(NO_MORE_DOCS, docs_only.next_doc()?);

    let mut docs_only2 = terms_enum.postings_with_flags(Some(docs_only), NONE as i32)?;
    assert_eq!(-1, docs_only2.doc_id());
    assert_eq!(0, docs_only2.next_doc()?);
    assert!(docs_only2.freq()? == 1 || docs_only2.freq()? == 2);
    assert_eq!(NO_MORE_DOCS, docs_only2.next_doc()?);

    let mut docs_and_positions_enum = terms_enum.postings_with_flags(None, POSITIONS as i32)?;
    assert_eq!(-1, docs_and_positions_enum.doc_id());
    assert_eq!(0, docs_and_positions_enum.next_doc()?);
    assert_eq!(2, docs_and_positions_enum.freq()?);
    assert_eq!(0, docs_and_positions_enum.next_position()?);
    assert!(
      docs_and_positions_enum.start_offset()? == -1 || docs_and_positions_enum.start_offset()? == 0
    );
    assert!(
      docs_and_positions_enum.end_offset()? == -1 || docs_and_positions_enum.end_offset()? == 3
    );
    assert!(docs_and_positions_enum.get_payload()?.is_none());
    assert_eq!(1, docs_and_positions_enum.next_position()?);
    assert!(
      docs_and_positions_enum.start_offset()? == -1 || docs_and_positions_enum.start_offset()? == 4
    );
    assert!(
      docs_and_positions_enum.end_offset()? == -1 || docs_and_positions_enum.end_offset()? == 7
    );
    assert!(docs_and_positions_enum.get_payload()?.is_none());
    assert_eq!(NO_MORE_DOCS, docs_and_positions_enum.next_doc()?);

    let mut docs_and_positions_enum2 =
      terms_enum.postings_with_flags(Some(docs_and_positions_enum), POSITIONS as i32)?;
    assert_eq!(-1, docs_and_positions_enum2.doc_id());
    assert_eq!(0, docs_and_positions_enum2.next_doc()?);
    assert_eq!(2, docs_and_positions_enum2.freq()?);
    assert_eq!(0, docs_and_positions_enum2.next_position()?);
    assert!(
      docs_and_positions_enum2.start_offset()? == -1
        || docs_and_positions_enum2.start_offset()? == 0
    );
    assert!(
      docs_and_positions_enum2.end_offset()? == -1 || docs_and_positions_enum2.end_offset()? == 3
    );
    assert!(docs_and_positions_enum2.get_payload()?.is_none());
    assert_eq!(1, docs_and_positions_enum2.next_position()?);
    assert!(
      docs_and_positions_enum2.start_offset()? == -1
        || docs_and_positions_enum2.start_offset()? == 4
    );
    assert!(
      docs_and_positions_enum2.end_offset()? == -1 || docs_and_positions_enum2.end_offset()? == 7
    );
    assert!(docs_and_positions_enum2.get_payload()?.is_none());
    assert_eq!(NO_MORE_DOCS, docs_and_positions_enum2.next_doc()?);

    let mut docs_and_positions_enum = terms_enum.postings_with_flags(None, PAYLOADS as i32)?;
    assert_eq!(-1, docs_and_positions_enum.doc_id());
    assert_eq!(0, docs_and_positions_enum.next_doc()?);
    assert_eq!(2, docs_and_positions_enum.freq()?);
    assert_eq!(0, docs_and_positions_enum.next_position()?);
    assert!(
      docs_and_positions_enum.start_offset()? == -1 || docs_and_positions_enum.start_offset()? == 0
    );
    assert!(
      docs_and_positions_enum.end_offset()? == -1 || docs_and_positions_enum.end_offset()? == 3
    );
    assert!(docs_and_positions_enum.get_payload()?.is_none());
    assert_eq!(1, docs_and_positions_enum.next_position()?);
    assert!(
      docs_and_positions_enum.start_offset()? == -1 || docs_and_positions_enum.start_offset()? == 4
    );
    assert!(
      docs_and_positions_enum.end_offset()? == -1 || docs_and_positions_enum.end_offset()? == 7
    );
    assert!(docs_and_positions_enum.get_payload()?.is_none());
    assert_eq!(NO_MORE_DOCS, docs_and_positions_enum.next_doc()?);

    let mut docs_and_positions_enum2 =
      terms_enum.postings_with_flags(Some(docs_and_positions_enum), PAYLOADS as i32)?;
    assert_eq!(-1, docs_and_positions_enum2.doc_id());
    assert_eq!(0, docs_and_positions_enum2.next_doc()?);
    assert_eq!(2, docs_and_positions_enum2.freq()?);
    assert_eq!(0, docs_and_positions_enum2.next_position()?);
    assert!(
      docs_and_positions_enum2.start_offset()? == -1
        || docs_and_positions_enum2.start_offset()? == 0
    );
    assert!(
      docs_and_positions_enum2.end_offset()? == -1 || docs_and_positions_enum2.end_offset()? == 3
    );
    assert!(docs_and_positions_enum2.get_payload()?.is_none());
    assert_eq!(1, docs_and_positions_enum2.next_position()?);
    assert!(
      docs_and_positions_enum2.start_offset()? == -1
        || docs_and_positions_enum2.start_offset()? == 4
    );
    assert!(
      docs_and_positions_enum2.end_offset()? == -1 || docs_and_positions_enum2.end_offset()? == 7
    );
    assert!(docs_and_positions_enum2.get_payload()?.is_none());
    assert_eq!(NO_MORE_DOCS, docs_and_positions_enum2.next_doc()?);

    let mut docs_and_positions_enum = terms_enum.postings_with_flags(None, OFFSETS as i32)?;
    assert_eq!(-1, docs_and_positions_enum.doc_id());
    assert_eq!(0, docs_and_positions_enum.next_doc()?);
    assert_eq!(2, docs_and_positions_enum.freq()?);
    assert_eq!(0, docs_and_positions_enum.next_position()?);
    assert_eq!(0, docs_and_positions_enum.start_offset()?);
    assert_eq!(3, docs_and_positions_enum.end_offset()?);
    assert!(docs_and_positions_enum.get_payload()?.is_none());
    assert_eq!(1, docs_and_positions_enum.next_position()?);
    assert_eq!(4, docs_and_positions_enum.start_offset()?);
    assert_eq!(7, docs_and_positions_enum.end_offset()?);
    assert!(docs_and_positions_enum.get_payload()?.is_none());
    assert_eq!(NO_MORE_DOCS, docs_and_positions_enum.next_doc()?);

    let mut docs_and_positions_enum2 =
      terms_enum.postings_with_flags(Some(docs_and_positions_enum), OFFSETS as i32)?;
    assert_eq!(-1, docs_and_positions_enum2.doc_id());
    assert_eq!(0, docs_and_positions_enum2.next_doc()?);
    assert_eq!(2, docs_and_positions_enum2.freq()?);
    assert_eq!(0, docs_and_positions_enum2.next_position()?);
    assert_eq!(0, docs_and_positions_enum2.start_offset()?);
    assert_eq!(3, docs_and_positions_enum2.end_offset()?);
    assert!(docs_and_positions_enum2.get_payload()?.is_none());
    assert_eq!(1, docs_and_positions_enum2.next_position()?);
    assert_eq!(4, docs_and_positions_enum2.start_offset()?);
    assert_eq!(7, docs_and_positions_enum2.end_offset()?);
    assert!(docs_and_positions_enum2.get_payload()?.is_none());
    assert_eq!(NO_MORE_DOCS, docs_and_positions_enum2.next_doc()?);

    let mut docs_and_positions_enum = terms_enum.postings_with_flags(None, ALL as i32)?;
    assert_eq!(-1, docs_and_positions_enum.doc_id());
    assert_eq!(0, docs_and_positions_enum.next_doc()?);
    assert_eq!(2, docs_and_positions_enum.freq()?);
    assert_eq!(0, docs_and_positions_enum.next_position()?);
    assert_eq!(0, docs_and_positions_enum.start_offset()?);
    assert_eq!(3, docs_and_positions_enum.end_offset()?);
    assert!(docs_and_positions_enum.get_payload()?.is_none());
    assert_eq!(1, docs_and_positions_enum.next_position()?);
    assert_eq!(4, docs_and_positions_enum.start_offset()?);
    assert_eq!(7, docs_and_positions_enum.end_offset()?);
    assert!(docs_and_positions_enum.get_payload()?.is_none());
    assert_eq!(NO_MORE_DOCS, docs_and_positions_enum.next_doc()?);

    let mut docs_and_positions_enum2 =
      terms_enum.postings_with_flags(Some(docs_and_positions_enum), ALL as i32)?;
    assert_eq!(-1, docs_and_positions_enum2.doc_id());
    assert_eq!(0, docs_and_positions_enum2.next_doc()?);
    assert_eq!(2, docs_and_positions_enum2.freq()?);
    assert_eq!(0, docs_and_positions_enum2.next_position()?);
    assert_eq!(0, docs_and_positions_enum2.start_offset()?);
    assert_eq!(3, docs_and_positions_enum2.end_offset()?);
    assert!(docs_and_positions_enum2.get_payload()?.is_none());
    assert_eq!(1, docs_and_positions_enum2.next_position()?);
    assert_eq!(4, docs_and_positions_enum2.start_offset()?);
    assert_eq!(7, docs_and_positions_enum2.end_offset()?);
    assert!(docs_and_positions_enum2.get_payload()?.is_none());
    assert_eq!(NO_MORE_DOCS, docs_and_positions_enum2.next_doc()?);

    reader.close()?;
    iw.close()?;
    Ok(())
  }

  fn test_postings_enum_offsets_without_positions<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(random)?;
    let analyzer = MockAnalyzer::new(random);
    // TODO TokenStreamComponents未实现
    let iwc = new_index_writer_config_with_analyzer(random, analyzer);
    let iw = IndexWriter::new(dir.clone(), iwc)?;

    let mut doc = Document::new();
    let mut ft = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
    ft.set_store_term_vectors(true)?;
    ft.set_store_term_vector_offsets(true)?;
    doc.add(Field::new("foo", "bar bar", ft));
    iw.add_document(doc)?;

    let reader = directory_reader::open_from_writer(&iw)?;

    let leaf = get_only_leaf_reader(&reader)?;
    let mut term_vectors = leaf.term_vectors()?;
    let terms = term_vectors.get_field_terms(0, "foo")?.unwrap();
    let mut terms_enum = terms.iterator()?;
    assert_eq!(
      &BytesRef::from_string("bar"),
      terms_enum.next()?.unwrap().as_ref()
    );

    let mut postings = terms_enum.postings(None)?;
    assert_eq!(-1, postings.doc_id());
    assert_eq!(0, postings.next_doc()?);
    assert_eq!(2, postings.freq()?);
    assert_eq!(NO_MORE_DOCS, postings.next_doc()?);

    let mut postings2 = terms_enum.postings(Some(postings))?;
    assert_eq!(-1, postings2.doc_id());
    assert_eq!(0, postings2.next_doc()?);
    assert_eq!(2, postings2.freq()?);
    assert_eq!(NO_MORE_DOCS, postings2.next_doc()?);

    let mut docs_only = terms_enum.postings_with_flags(None, NONE as i32)?;
    assert_eq!(-1, docs_only.doc_id());
    assert_eq!(0, docs_only.next_doc()?);
    assert!(docs_only.freq()? == 1 || docs_only.freq()? == 2);
    assert_eq!(NO_MORE_DOCS, docs_only.next_doc()?);

    let mut docs_only2 = terms_enum.postings_with_flags(Some(docs_only), NONE as i32)?;
    assert_eq!(-1, docs_only2.doc_id());
    assert_eq!(0, docs_only2.next_doc()?);
    assert!(docs_only2.freq()? == 1 || docs_only2.freq()? == 2);
    assert_eq!(NO_MORE_DOCS, docs_only2.next_doc()?);

    let mut docs_and_positions_enum = terms_enum.postings_with_flags(None, POSITIONS as i32)?;
    assert_eq!(-1, docs_and_positions_enum.doc_id());
    assert_eq!(0, docs_and_positions_enum.next_doc()?);
    assert_eq!(2, docs_and_positions_enum.freq()?);
    assert_eq!(-1, docs_and_positions_enum.next_position()?);
    assert!(
      docs_and_positions_enum.start_offset()? == -1 || docs_and_positions_enum.start_offset()? == 0
    );
    assert!(
      docs_and_positions_enum.end_offset()? == -1 || docs_and_positions_enum.end_offset()? == 3
    );
    assert!(docs_and_positions_enum.get_payload()?.is_none());
    assert_eq!(-1, docs_and_positions_enum.next_position()?);
    assert!(
      docs_and_positions_enum.start_offset()? == -1 || docs_and_positions_enum.start_offset()? == 4
    );
    assert!(
      docs_and_positions_enum.end_offset()? == -1 || docs_and_positions_enum.end_offset()? == 7
    );
    assert!(docs_and_positions_enum.get_payload()?.is_none());
    assert_eq!(NO_MORE_DOCS, docs_and_positions_enum.next_doc()?);

    let mut docs_and_positions_enum2 =
      terms_enum.postings_with_flags(Some(docs_and_positions_enum), POSITIONS as i32)?;
    assert_eq!(-1, docs_and_positions_enum2.doc_id());
    assert_eq!(0, docs_and_positions_enum2.next_doc()?);
    assert_eq!(2, docs_and_positions_enum2.freq()?);
    assert_eq!(-1, docs_and_positions_enum2.next_position()?);
    assert!(
      docs_and_positions_enum2.start_offset()? == -1
        || docs_and_positions_enum2.start_offset()? == 0
    );
    assert!(
      docs_and_positions_enum2.end_offset()? == -1 || docs_and_positions_enum2.end_offset()? == 3
    );
    assert!(docs_and_positions_enum2.get_payload()?.is_none());
    assert_eq!(-1, docs_and_positions_enum2.next_position()?);
    assert!(
      docs_and_positions_enum2.start_offset()? == -1
        || docs_and_positions_enum2.start_offset()? == 4
    );
    assert!(
      docs_and_positions_enum2.end_offset()? == -1 || docs_and_positions_enum2.end_offset()? == 7
    );
    assert!(docs_and_positions_enum2.get_payload()?.is_none());
    assert_eq!(NO_MORE_DOCS, docs_and_positions_enum2.next_doc()?);

    let mut docs_and_positions_enum = terms_enum.postings_with_flags(None, PAYLOADS as i32)?;
    assert_eq!(-1, docs_and_positions_enum.doc_id());
    assert_eq!(0, docs_and_positions_enum.next_doc()?);
    assert_eq!(2, docs_and_positions_enum.freq()?);
    assert_eq!(-1, docs_and_positions_enum.next_position()?);
    assert!(
      docs_and_positions_enum.start_offset()? == -1 || docs_and_positions_enum.start_offset()? == 0
    );
    assert!(
      docs_and_positions_enum.end_offset()? == -1 || docs_and_positions_enum.end_offset()? == 3
    );
    assert!(docs_and_positions_enum.get_payload()?.is_none());
    assert_eq!(-1, docs_and_positions_enum.next_position()?);
    assert!(
      docs_and_positions_enum.start_offset()? == -1 || docs_and_positions_enum.start_offset()? == 4
    );
    assert!(
      docs_and_positions_enum.end_offset()? == -1 || docs_and_positions_enum.end_offset()? == 7
    );
    assert!(docs_and_positions_enum.get_payload()?.is_none());
    assert_eq!(NO_MORE_DOCS, docs_and_positions_enum.next_doc()?);

    let mut docs_and_positions_enum2 =
      terms_enum.postings_with_flags(Some(docs_and_positions_enum), PAYLOADS as i32)?;
    assert_eq!(-1, docs_and_positions_enum2.doc_id());
    assert_eq!(0, docs_and_positions_enum2.next_doc()?);
    assert_eq!(2, docs_and_positions_enum2.freq()?);
    assert_eq!(-1, docs_and_positions_enum2.next_position()?);
    assert!(
      docs_and_positions_enum2.start_offset()? == -1
        || docs_and_positions_enum2.start_offset()? == 0
    );
    assert!(
      docs_and_positions_enum2.end_offset()? == -1 || docs_and_positions_enum2.end_offset()? == 3
    );
    assert!(docs_and_positions_enum2.get_payload()?.is_none());
    assert_eq!(-1, docs_and_positions_enum2.next_position()?);
    assert!(
      docs_and_positions_enum2.start_offset()? == -1
        || docs_and_positions_enum2.start_offset()? == 4
    );
    assert!(
      docs_and_positions_enum2.end_offset()? == -1 || docs_and_positions_enum2.end_offset()? == 7
    );
    assert!(docs_and_positions_enum2.get_payload()?.is_none());
    assert_eq!(NO_MORE_DOCS, docs_and_positions_enum2.next_doc()?);

    let mut docs_and_positions_enum = terms_enum.postings_with_flags(None, OFFSETS as i32)?;
    assert_eq!(-1, docs_and_positions_enum.doc_id());
    assert_eq!(0, docs_and_positions_enum.next_doc()?);
    assert_eq!(2, docs_and_positions_enum.freq()?);
    assert_eq!(-1, docs_and_positions_enum.next_position()?);
    assert_eq!(0, docs_and_positions_enum.start_offset()?);
    assert_eq!(3, docs_and_positions_enum.end_offset()?);
    assert!(docs_and_positions_enum.get_payload()?.is_none());
    assert_eq!(-1, docs_and_positions_enum.next_position()?);
    assert_eq!(4, docs_and_positions_enum.start_offset()?);
    assert_eq!(7, docs_and_positions_enum.end_offset()?);
    assert!(docs_and_positions_enum.get_payload()?.is_none());
    assert_eq!(NO_MORE_DOCS, docs_and_positions_enum.next_doc()?);

    let mut docs_and_positions_enum2 =
      terms_enum.postings_with_flags(Some(docs_and_positions_enum), OFFSETS as i32)?;
    assert_eq!(-1, docs_and_positions_enum2.doc_id());
    assert_eq!(0, docs_and_positions_enum2.next_doc()?);
    assert_eq!(2, docs_and_positions_enum2.freq()?);
    assert_eq!(-1, docs_and_positions_enum2.next_position()?);
    assert_eq!(0, docs_and_positions_enum2.start_offset()?);
    assert_eq!(3, docs_and_positions_enum2.end_offset()?);
    assert!(docs_and_positions_enum2.get_payload()?.is_none());
    assert_eq!(-1, docs_and_positions_enum2.next_position()?);
    assert_eq!(4, docs_and_positions_enum2.start_offset()?);
    assert_eq!(7, docs_and_positions_enum2.end_offset()?);
    assert!(docs_and_positions_enum2.get_payload()?.is_none());
    assert_eq!(NO_MORE_DOCS, docs_and_positions_enum2.next_doc()?);

    let mut docs_and_positions_enum = terms_enum.postings_with_flags(None, ALL as i32)?;
    assert_eq!(-1, docs_and_positions_enum.doc_id());
    assert_eq!(0, docs_and_positions_enum.next_doc()?);
    assert_eq!(2, docs_and_positions_enum.freq()?);
    assert_eq!(-1, docs_and_positions_enum.next_position()?);
    assert_eq!(0, docs_and_positions_enum.start_offset()?);
    assert_eq!(3, docs_and_positions_enum.end_offset()?);
    assert!(docs_and_positions_enum.get_payload()?.is_none());
    assert_eq!(-1, docs_and_positions_enum.next_position()?);
    assert_eq!(4, docs_and_positions_enum.start_offset()?);
    assert_eq!(7, docs_and_positions_enum.end_offset()?);
    assert!(docs_and_positions_enum.get_payload()?.is_none());
    assert_eq!(NO_MORE_DOCS, docs_and_positions_enum.next_doc()?);

    let mut docs_and_positions_enum2 =
      terms_enum.postings_with_flags(Some(docs_and_positions_enum), ALL as i32)?;
    assert_eq!(-1, docs_and_positions_enum2.doc_id());
    assert_eq!(0, docs_and_positions_enum2.next_doc()?);
    assert_eq!(2, docs_and_positions_enum2.freq()?);
    assert_eq!(-1, docs_and_positions_enum2.next_position()?);
    assert_eq!(0, docs_and_positions_enum2.start_offset()?);
    assert_eq!(3, docs_and_positions_enum2.end_offset()?);
    assert!(docs_and_positions_enum2.get_payload()?.is_none());
    assert_eq!(-1, docs_and_positions_enum2.next_position()?);
    assert_eq!(4, docs_and_positions_enum2.start_offset()?);
    assert_eq!(7, docs_and_positions_enum2.end_offset()?);
    assert!(docs_and_positions_enum2.get_payload()?.is_none());
    assert_eq!(NO_MORE_DOCS, docs_and_positions_enum2.next_doc()?);

    reader.close()?;
    iw.close()?;
    Ok(())
  }
  fn test_postings_enum_payloads<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(random)?;
    let iwc = new_index_writer_config(random);
    let w = IndexWriter::new(dir, iwc)?;

    let mut token1 = token::with_range(Some("bar"), 0, 3)?;
    token1
      .sub
      .token
      .set_payload(Some(BytesRef::from_string("pay1")));

    let mut token2 = token::with_range(Some("bar"), 4, 7)?;
    token2
      .sub
      .token
      .set_payload(Some(BytesRef::from_string("pay2")));

    let mut ft = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
    ft.set_store_term_vectors(true)?;
    ft.set_store_term_vector_positions(true)?;
    ft.set_store_term_vector_payloads(true)?;

    let mut doc = Document::new();
    doc.add(Field::from_token_stream(
      "foo",
      FieldTokenStreamEnum::custom(CannedTokenStream::new(vec![token1, token2])),
      ft,
    )?);
    w.add_document(doc)?;

    let reader = directory_reader::open_from_writer(&w)?;
    let leaf = get_only_leaf_reader(reader)?;

    let mut term_vectors = leaf.term_vectors()?;
    let terms = term_vectors.get_field_terms(0, "foo")?.unwrap();
    let mut terms_enum = terms.iterator()?;
    assert_eq!(
      &BytesRef::from_string("bar"),
      terms_enum.next()?.unwrap().as_ref()
    );

    let mut postings = terms_enum.postings(None)?;
    assert_eq!(-1, postings.doc_id());
    assert_eq!(0, postings.next_doc()?);
    assert_eq!(2, postings.freq()?);
    assert_eq!(NO_MORE_DOCS, postings.next_doc()?);

    let mut postings2 = terms_enum.postings(Some(postings))?;
    assert_eq!(-1, postings2.doc_id());
    assert_eq!(0, postings2.next_doc()?);
    assert_eq!(2, postings2.freq()?);
    assert_eq!(NO_MORE_DOCS, postings2.next_doc()?);

    let mut docs_only = terms_enum.postings_with_flags(None, NONE as i32)?;
    assert_eq!(-1, docs_only.doc_id());
    assert_eq!(0, docs_only.next_doc()?);
    assert!(docs_only.freq()? == 1 || docs_only.freq()? == 2);
    assert_eq!(NO_MORE_DOCS, docs_only.next_doc()?);

    let mut docs_only2 = terms_enum.postings_with_flags(Some(docs_only), NONE as i32)?;
    assert_eq!(-1, docs_only2.doc_id());
    assert_eq!(0, docs_only2.next_doc()?);
    assert!(docs_only2.freq()? == 1 || docs_only2.freq()? == 2);
    assert_eq!(NO_MORE_DOCS, docs_only2.next_doc()?);

    let mut docs_and_positions_enum = terms_enum.postings_with_flags(None, POSITIONS as i32)?;
    assert_eq!(-1, docs_and_positions_enum.doc_id());
    assert_eq!(0, docs_and_positions_enum.next_doc()?);
    assert_eq!(2, docs_and_positions_enum.freq()?);
    assert_eq!(0, docs_and_positions_enum.next_position()?);
    assert_eq!(-1, docs_and_positions_enum.start_offset()?);
    assert_eq!(-1, docs_and_positions_enum.end_offset()?);
    assert!(
      docs_and_positions_enum.get_payload()?.is_none()
        || docs_and_positions_enum.get_payload()?.unwrap().as_ref()
          == &BytesRef::from_string("pay1")
    );
    assert_eq!(1, docs_and_positions_enum.next_position()?);
    assert_eq!(-1, docs_and_positions_enum.start_offset()?);
    assert_eq!(-1, docs_and_positions_enum.end_offset()?);
    assert!(
      docs_and_positions_enum.get_payload()?.is_none()
        || docs_and_positions_enum.get_payload()?.unwrap().as_ref()
          == &BytesRef::from_string("pay2")
    );
    assert_eq!(NO_MORE_DOCS, docs_and_positions_enum.next_doc()?);

    let mut docs_and_positions_enum2 =
      terms_enum.postings_with_flags(Some(docs_and_positions_enum), POSITIONS as i32)?;
    assert_eq!(-1, docs_and_positions_enum2.doc_id());
    assert_eq!(0, docs_and_positions_enum2.next_doc()?);
    assert_eq!(2, docs_and_positions_enum2.freq()?);
    assert_eq!(0, docs_and_positions_enum2.next_position()?);
    assert_eq!(-1, docs_and_positions_enum2.start_offset()?);
    assert_eq!(-1, docs_and_positions_enum2.end_offset()?);
    assert!(
      docs_and_positions_enum2.get_payload()?.is_none()
        || docs_and_positions_enum2.get_payload()?.unwrap().as_ref()
          == &BytesRef::from_string("pay1")
    );
    assert_eq!(1, docs_and_positions_enum2.next_position()?);
    assert_eq!(-1, docs_and_positions_enum2.start_offset()?);
    assert_eq!(-1, docs_and_positions_enum2.end_offset()?);
    assert!(
      docs_and_positions_enum2.get_payload()?.is_none()
        || docs_and_positions_enum2.get_payload()?.unwrap().as_ref()
          == &BytesRef::from_string("pay2")
    );
    assert_eq!(NO_MORE_DOCS, docs_and_positions_enum2.next_doc()?);

    let mut docs_and_positions_enum = terms_enum.postings_with_flags(None, PAYLOADS as i32)?;
    assert_eq!(-1, docs_and_positions_enum.doc_id());
    assert_eq!(0, docs_and_positions_enum.next_doc()?);
    assert_eq!(2, docs_and_positions_enum.freq()?);
    assert_eq!(0, docs_and_positions_enum.next_position()?);
    assert_eq!(-1, docs_and_positions_enum.start_offset()?);
    assert_eq!(-1, docs_and_positions_enum.end_offset()?);
    assert_eq!(
      &BytesRef::from_string("pay1"),
      docs_and_positions_enum.get_payload()?.unwrap().as_ref()
    );
    assert_eq!(1, docs_and_positions_enum.next_position()?);
    assert_eq!(-1, docs_and_positions_enum.start_offset()?);
    assert_eq!(-1, docs_and_positions_enum.end_offset()?);
    assert_eq!(
      &BytesRef::from_string("pay2"),
      docs_and_positions_enum.get_payload()?.unwrap().as_ref()
    );
    assert_eq!(NO_MORE_DOCS, docs_and_positions_enum.next_doc()?);

    let mut docs_and_positions_enum2 =
      terms_enum.postings_with_flags(Some(docs_and_positions_enum), PAYLOADS as i32)?;
    assert_eq!(-1, docs_and_positions_enum2.doc_id());
    assert_eq!(0, docs_and_positions_enum2.next_doc()?);
    assert_eq!(2, docs_and_positions_enum2.freq()?);
    assert_eq!(0, docs_and_positions_enum2.next_position()?);
    assert_eq!(-1, docs_and_positions_enum2.start_offset()?);
    assert_eq!(-1, docs_and_positions_enum2.end_offset()?);
    assert_eq!(
      &BytesRef::from_string("pay1"),
      docs_and_positions_enum2.get_payload()?.unwrap().as_ref()
    );
    assert_eq!(1, docs_and_positions_enum2.next_position()?);
    assert_eq!(-1, docs_and_positions_enum2.start_offset()?);
    assert_eq!(-1, docs_and_positions_enum2.end_offset()?);
    assert_eq!(
      &BytesRef::from_string("pay2"),
      docs_and_positions_enum2.get_payload()?.unwrap().as_ref()
    );
    assert_eq!(NO_MORE_DOCS, docs_and_positions_enum2.next_doc()?);

    let mut docs_and_positions_enum = terms_enum.postings_with_flags(None, OFFSETS as i32)?;
    assert_eq!(-1, docs_and_positions_enum.doc_id());
    assert_eq!(0, docs_and_positions_enum.next_doc()?);
    assert_eq!(2, docs_and_positions_enum.freq()?);
    assert_eq!(0, docs_and_positions_enum.next_position()?);
    assert_eq!(-1, docs_and_positions_enum.start_offset()?);
    assert_eq!(-1, docs_and_positions_enum.end_offset()?);
    assert!(
      docs_and_positions_enum.get_payload()?.is_none()
        || docs_and_positions_enum.get_payload()?.unwrap().as_ref()
          == &BytesRef::from_string("pay1")
    );
    assert_eq!(1, docs_and_positions_enum.next_position()?);
    assert_eq!(-1, docs_and_positions_enum.start_offset()?);
    assert_eq!(-1, docs_and_positions_enum.end_offset()?);
    assert!(
      docs_and_positions_enum.get_payload()?.is_none()
        || docs_and_positions_enum.get_payload()?.unwrap().as_ref()
          == &BytesRef::from_string("pay2")
    );
    assert_eq!(NO_MORE_DOCS, docs_and_positions_enum.next_doc()?);

    let mut docs_and_positions_enum2 =
      terms_enum.postings_with_flags(Some(docs_and_positions_enum), OFFSETS as i32)?;
    assert_eq!(-1, docs_and_positions_enum2.doc_id());
    assert_eq!(0, docs_and_positions_enum2.next_doc()?);
    assert_eq!(2, docs_and_positions_enum2.freq()?);
    assert_eq!(0, docs_and_positions_enum2.next_position()?);
    assert_eq!(-1, docs_and_positions_enum2.start_offset()?);
    assert_eq!(-1, docs_and_positions_enum2.end_offset()?);
    assert!(
      docs_and_positions_enum2.get_payload()?.is_none()
        || docs_and_positions_enum2.get_payload()?.unwrap().as_ref()
          == &BytesRef::from_string("pay1")
    );
    assert_eq!(1, docs_and_positions_enum2.next_position()?);
    assert_eq!(-1, docs_and_positions_enum2.start_offset()?);
    assert_eq!(-1, docs_and_positions_enum2.end_offset()?);
    assert!(
      docs_and_positions_enum2.get_payload()?.is_none()
        || docs_and_positions_enum2.get_payload()?.unwrap().as_ref()
          == &BytesRef::from_string("pay2")
    );
    assert_eq!(NO_MORE_DOCS, docs_and_positions_enum2.next_doc()?);

    let mut docs_and_positions_enum = terms_enum.postings_with_flags(None, ALL as i32)?;
    assert_eq!(-1, docs_and_positions_enum.doc_id());
    assert_eq!(0, docs_and_positions_enum.next_doc()?);
    assert_eq!(2, docs_and_positions_enum.freq()?);
    assert_eq!(0, docs_and_positions_enum.next_position()?);
    assert_eq!(-1, docs_and_positions_enum.start_offset()?);
    assert_eq!(-1, docs_and_positions_enum.end_offset()?);
    assert_eq!(
      &BytesRef::from_string("pay1"),
      docs_and_positions_enum.get_payload()?.unwrap().as_ref()
    );
    assert_eq!(1, docs_and_positions_enum.next_position()?);
    assert_eq!(-1, docs_and_positions_enum.start_offset()?);
    assert_eq!(-1, docs_and_positions_enum.end_offset()?);
    assert_eq!(
      &BytesRef::from_string("pay2"),
      docs_and_positions_enum.get_payload()?.unwrap().as_ref()
    );
    assert_eq!(NO_MORE_DOCS, docs_and_positions_enum.next_doc()?);

    let mut docs_and_positions_enum2 =
      terms_enum.postings_with_flags(Some(docs_and_positions_enum), ALL as i32)?;
    assert_eq!(-1, docs_and_positions_enum2.doc_id());
    assert_eq!(0, docs_and_positions_enum2.next_doc()?);
    assert_eq!(2, docs_and_positions_enum2.freq()?);
    assert_eq!(0, docs_and_positions_enum2.next_position()?);
    assert_eq!(-1, docs_and_positions_enum2.start_offset()?);
    assert_eq!(-1, docs_and_positions_enum2.end_offset()?);
    assert_eq!(
      &BytesRef::from_string("pay1"),
      docs_and_positions_enum2.get_payload()?.unwrap().as_ref()
    );
    assert_eq!(1, docs_and_positions_enum2.next_position()?);
    assert_eq!(-1, docs_and_positions_enum2.start_offset()?);
    assert_eq!(-1, docs_and_positions_enum2.end_offset()?);
    assert_eq!(
      &BytesRef::from_string("pay2"),
      docs_and_positions_enum2.get_payload()?.unwrap().as_ref()
    );
    assert_eq!(NO_MORE_DOCS, docs_and_positions_enum2.next_doc()?);

    Ok(())
  }

  fn test_postings_enum_all<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(random)?;
    let iwc = new_index_writer_config(random);
    let w = IndexWriter::new(dir, iwc)?;

    let mut token1 = token::with_range(Some("bar"), 0, 3)?;
    token1
      .sub
      .token
      .set_payload(Some(BytesRef::from_string("pay1")));

    let mut token2 = token::with_range(Some("bar"), 4, 7)?;
    token2
      .sub
      .token
      .set_payload(Some(BytesRef::from_string("pay2")));

    let mut ft = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
    ft.set_store_term_vectors(true)?;
    ft.set_store_term_vector_positions(true)?;
    ft.set_store_term_vector_payloads(true)?;
    ft.set_store_term_vector_offsets(true)?;

    let mut doc = Document::new();
    doc.add(Field::from_token_stream(
      "foo",
      FieldTokenStreamEnum::custom(CannedTokenStream::new(vec![token1, token2])),
      ft,
    )?);
    w.add_document(doc)?;

    let reader = directory_reader::open_from_writer(&w)?;
    let leaf = get_only_leaf_reader(reader)?;

    let mut term_vectors = leaf.term_vectors()?;
    let terms = term_vectors.get_field_terms(0, "foo")?.unwrap();
    let mut terms_enum = terms.iterator()?;
    assert_eq!(
      &BytesRef::from_string("bar"),
      terms_enum.next()?.unwrap().as_ref()
    );

    let mut postings = terms_enum.postings(None)?;
    assert_eq!(-1, postings.doc_id());
    assert_eq!(0, postings.next_doc()?);
    assert_eq!(2, postings.freq()?);
    assert_eq!(NO_MORE_DOCS, postings.next_doc()?);

    let mut postings2 = terms_enum.postings(Some(postings))?;
    assert_eq!(-1, postings2.doc_id());
    assert_eq!(0, postings2.next_doc()?);
    assert_eq!(2, postings2.freq()?);
    assert_eq!(NO_MORE_DOCS, postings2.next_doc()?);

    let mut docs_only = terms_enum.postings_with_flags(None, NONE as i32)?;
    assert_eq!(-1, docs_only.doc_id());
    assert_eq!(0, docs_only.next_doc()?);
    assert!(docs_only.freq()? == 1 || docs_only.freq()? == 2);
    assert_eq!(NO_MORE_DOCS, docs_only.next_doc()?);

    let mut docs_only2 = terms_enum.postings_with_flags(Some(docs_only), NONE as i32)?;
    assert_eq!(-1, docs_only2.doc_id());
    assert_eq!(0, docs_only2.next_doc()?);
    assert!(docs_only2.freq()? == 1 || docs_only2.freq()? == 2);
    assert_eq!(NO_MORE_DOCS, docs_only2.next_doc()?);

    let mut docs_and_positions_enum = terms_enum.postings_with_flags(None, POSITIONS as i32)?;
    assert_eq!(-1, docs_and_positions_enum.doc_id());
    assert_eq!(0, docs_and_positions_enum.next_doc()?);
    assert_eq!(2, docs_and_positions_enum.freq()?);
    assert_eq!(0, docs_and_positions_enum.next_position()?);
    assert!(
      docs_and_positions_enum.start_offset()? == -1 || docs_and_positions_enum.start_offset()? == 0
    );
    assert!(
      docs_and_positions_enum.end_offset()? == -1 || docs_and_positions_enum.end_offset()? == 3
    );
    assert!(
      docs_and_positions_enum.get_payload()?.is_none()
        || docs_and_positions_enum.get_payload()?.unwrap().as_ref()
          == &BytesRef::from_string("pay1")
    );
    assert_eq!(1, docs_and_positions_enum.next_position()?);
    assert!(
      docs_and_positions_enum.start_offset()? == -1 || docs_and_positions_enum.start_offset()? == 4
    );
    assert!(
      docs_and_positions_enum.end_offset()? == -1 || docs_and_positions_enum.end_offset()? == 7
    );
    assert!(
      docs_and_positions_enum.get_payload()?.is_none()
        || docs_and_positions_enum.get_payload()?.unwrap().as_ref()
          == &BytesRef::from_string("pay2")
    );
    assert_eq!(NO_MORE_DOCS, docs_and_positions_enum.next_doc()?);

    let mut docs_and_positions_enum2 =
      terms_enum.postings_with_flags(Some(docs_and_positions_enum), POSITIONS as i32)?;
    assert_eq!(-1, docs_and_positions_enum2.doc_id());
    assert_eq!(0, docs_and_positions_enum2.next_doc()?);
    assert_eq!(2, docs_and_positions_enum2.freq()?);
    assert_eq!(0, docs_and_positions_enum2.next_position()?);
    assert!(
      docs_and_positions_enum2.start_offset()? == -1
        || docs_and_positions_enum2.start_offset()? == 0
    );
    assert!(
      docs_and_positions_enum2.end_offset()? == -1 || docs_and_positions_enum2.end_offset()? == 3
    );
    assert!(
      docs_and_positions_enum2.get_payload()?.is_none()
        || docs_and_positions_enum2.get_payload()?.unwrap().as_ref()
          == &BytesRef::from_string("pay1")
    );
    assert_eq!(1, docs_and_positions_enum2.next_position()?);
    assert!(
      docs_and_positions_enum2.start_offset()? == -1
        || docs_and_positions_enum2.start_offset()? == 4
    );
    assert!(
      docs_and_positions_enum2.end_offset()? == -1 || docs_and_positions_enum2.end_offset()? == 7
    );
    assert!(
      docs_and_positions_enum2.get_payload()?.is_none()
        || docs_and_positions_enum2.get_payload()?.unwrap().as_ref()
          == &BytesRef::from_string("pay2")
    );
    assert_eq!(NO_MORE_DOCS, docs_and_positions_enum2.next_doc()?);

    let mut docs_and_positions_enum = terms_enum.postings_with_flags(None, PAYLOADS as i32)?;
    assert_eq!(-1, docs_and_positions_enum.doc_id());
    assert_eq!(0, docs_and_positions_enum.next_doc()?);
    assert_eq!(2, docs_and_positions_enum.freq()?);
    assert_eq!(0, docs_and_positions_enum.next_position()?);
    assert!(
      docs_and_positions_enum.start_offset()? == -1 || docs_and_positions_enum.start_offset()? == 0
    );
    assert!(
      docs_and_positions_enum.end_offset()? == -1 || docs_and_positions_enum.end_offset()? == 3
    );
    assert_eq!(
      &BytesRef::from_string("pay1"),
      docs_and_positions_enum.get_payload()?.unwrap().as_ref()
    );
    assert_eq!(1, docs_and_positions_enum.next_position()?);
    assert!(
      docs_and_positions_enum.start_offset()? == -1 || docs_and_positions_enum.start_offset()? == 4
    );
    assert!(
      docs_and_positions_enum.end_offset()? == -1 || docs_and_positions_enum.end_offset()? == 7
    );
    assert_eq!(
      &BytesRef::from_string("pay2"),
      docs_and_positions_enum.get_payload()?.unwrap().as_ref()
    );
    assert_eq!(NO_MORE_DOCS, docs_and_positions_enum.next_doc()?);

    let mut docs_and_positions_enum2 =
      terms_enum.postings_with_flags(Some(docs_and_positions_enum), PAYLOADS as i32)?;
    assert_eq!(-1, docs_and_positions_enum2.doc_id());
    assert_eq!(0, docs_and_positions_enum2.next_doc()?);
    assert_eq!(2, docs_and_positions_enum2.freq()?);
    assert_eq!(0, docs_and_positions_enum2.next_position()?);
    assert!(
      docs_and_positions_enum2.start_offset()? == -1
        || docs_and_positions_enum2.start_offset()? == 0
    );
    assert!(
      docs_and_positions_enum2.end_offset()? == -1 || docs_and_positions_enum2.end_offset()? == 3
    );
    assert_eq!(
      &BytesRef::from_string("pay1"),
      docs_and_positions_enum2.get_payload()?.unwrap().as_ref()
    );
    assert_eq!(1, docs_and_positions_enum2.next_position()?);
    assert!(
      docs_and_positions_enum2.start_offset()? == -1
        || docs_and_positions_enum2.start_offset()? == 4
    );
    assert!(
      docs_and_positions_enum2.end_offset()? == -1 || docs_and_positions_enum2.end_offset()? == 7
    );
    assert_eq!(
      &BytesRef::from_string("pay2"),
      docs_and_positions_enum2.get_payload()?.unwrap().as_ref()
    );
    assert_eq!(NO_MORE_DOCS, docs_and_positions_enum2.next_doc()?);

    let mut docs_and_positions_enum = terms_enum.postings_with_flags(None, OFFSETS as i32)?;
    assert_eq!(-1, docs_and_positions_enum.doc_id());
    assert_eq!(0, docs_and_positions_enum.next_doc()?);
    assert_eq!(2, docs_and_positions_enum.freq()?);
    assert_eq!(0, docs_and_positions_enum.next_position()?);
    assert_eq!(0, docs_and_positions_enum.start_offset()?);
    assert_eq!(3, docs_and_positions_enum.end_offset()?);
    assert!(
      docs_and_positions_enum.get_payload()?.is_none()
        || docs_and_positions_enum.get_payload()?.unwrap().as_ref()
          == &BytesRef::from_string("pay1")
    );
    assert_eq!(1, docs_and_positions_enum.next_position()?);
    assert_eq!(4, docs_and_positions_enum.start_offset()?);
    assert_eq!(7, docs_and_positions_enum.end_offset()?);
    assert!(
      docs_and_positions_enum.get_payload()?.is_none()
        || docs_and_positions_enum.get_payload()?.unwrap().as_ref()
          == &BytesRef::from_string("pay2")
    );
    assert_eq!(NO_MORE_DOCS, docs_and_positions_enum.next_doc()?);

    let mut docs_and_positions_enum2 =
      terms_enum.postings_with_flags(Some(docs_and_positions_enum), OFFSETS as i32)?;
    assert_eq!(-1, docs_and_positions_enum2.doc_id());
    assert_eq!(0, docs_and_positions_enum2.next_doc()?);
    assert_eq!(2, docs_and_positions_enum2.freq()?);
    assert_eq!(0, docs_and_positions_enum2.next_position()?);
    assert_eq!(0, docs_and_positions_enum2.start_offset()?);
    assert_eq!(3, docs_and_positions_enum2.end_offset()?);
    assert!(
      docs_and_positions_enum2.get_payload()?.is_none()
        || docs_and_positions_enum2.get_payload()?.unwrap().as_ref()
          == &BytesRef::from_string("pay1")
    );
    assert_eq!(1, docs_and_positions_enum2.next_position()?);
    assert_eq!(4, docs_and_positions_enum2.start_offset()?);
    assert_eq!(7, docs_and_positions_enum2.end_offset()?);
    assert!(
      docs_and_positions_enum2.get_payload()?.is_none()
        || docs_and_positions_enum2.get_payload()?.unwrap().as_ref()
          == &BytesRef::from_string("pay2")
    );
    assert_eq!(NO_MORE_DOCS, docs_and_positions_enum2.next_doc()?);

    let mut docs_and_positions_enum = terms_enum.postings_with_flags(None, ALL as i32)?;
    assert_eq!(-1, docs_and_positions_enum.doc_id());
    assert_eq!(0, docs_and_positions_enum.next_doc()?);
    assert_eq!(2, docs_and_positions_enum.freq()?);
    assert_eq!(0, docs_and_positions_enum.next_position()?);
    assert_eq!(0, docs_and_positions_enum.start_offset()?);
    assert_eq!(3, docs_and_positions_enum.end_offset()?);
    assert_eq!(
      &BytesRef::from_string("pay1"),
      docs_and_positions_enum.get_payload()?.unwrap().as_ref()
    );
    assert_eq!(1, docs_and_positions_enum.next_position()?);
    assert_eq!(4, docs_and_positions_enum.start_offset()?);
    assert_eq!(7, docs_and_positions_enum.end_offset()?);
    assert_eq!(
      &BytesRef::from_string("pay2"),
      docs_and_positions_enum.get_payload()?.unwrap().as_ref()
    );
    assert_eq!(NO_MORE_DOCS, docs_and_positions_enum.next_doc()?);

    let mut docs_and_positions_enum2 =
      terms_enum.postings_with_flags(Some(docs_and_positions_enum), ALL as i32)?;
    assert_eq!(-1, docs_and_positions_enum2.doc_id());
    assert_eq!(0, docs_and_positions_enum2.next_doc()?);
    assert_eq!(2, docs_and_positions_enum2.freq()?);
    assert_eq!(0, docs_and_positions_enum2.next_position()?);
    assert_eq!(0, docs_and_positions_enum2.start_offset()?);
    assert_eq!(3, docs_and_positions_enum2.end_offset()?);
    assert_eq!(
      &BytesRef::from_string("pay1"),
      docs_and_positions_enum2.get_payload()?.unwrap().as_ref()
    );
    assert_eq!(1, docs_and_positions_enum2.next_position()?);
    assert_eq!(4, docs_and_positions_enum2.start_offset()?);
    assert_eq!(7, docs_and_positions_enum2.end_offset()?);
    assert_eq!(
      &BytesRef::from_string("pay2"),
      docs_and_positions_enum2.get_payload()?.unwrap().as_ref()
    );
    assert_eq!(NO_MORE_DOCS, docs_and_positions_enum2.next_doc()?);

    Ok(())
  }
}
