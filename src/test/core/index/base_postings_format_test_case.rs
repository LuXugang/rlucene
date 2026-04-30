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
use crate::core::document::field::Store::No;
use crate::core::document::field::{Field, Store};
use crate::core::document::field_type::FieldType;
use crate::core::document::fields::FieldTokenStreamEnum;
use crate::core::document::text_field::{TextField, text_field_type};
use crate::core::index::BytesRef;
use crate::core::index::composite_reader::get_context;
use crate::core::index::directory_reader;
use crate::core::index::fields::Fields;
use crate::core::index::index_options::IndexOptions;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::postings_enum::{
  ALL, FREQS, NONE, OFFSETS, PAYLOADS, POSITIONS, PostingsEnum, PostingsEnumEnum2,
};
use crate::core::index::term::Term;
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::{SeekStatus, TermsEnum};
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::term_query::TermQuery;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::canned_token_stream::CannedTokenStream;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::analysis::token;
use crate::test::core::index::base_index_file_format_test_case::BaseIndexFileFormatTestCase;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::index::random_postings_tester::Option_;
use crate::test::core::index::random_postings_tester::RandomPostingsTester;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  create_temp_dir_with_prefix, get_only_leaf_reader, new_directory_shared, new_fs_directory,
  new_index_writer_config, new_index_writer_config_with_analyzer, new_log_merge_policy,
  new_string_field, new_text_field, new_tiered_merge_policy,
};
use rand::prelude::SliceRandom;
use rand::{Rng, RngExt};
use std::collections::{HashMap, HashSet};
use strum::IntoEnumIterator;

pub trait BasePostingsFormatTestCase: BaseIndexFileFormatTestCase {
  fn create_postings<R>(&self, random: &mut R) -> RandomPostingsTester
  where
    R: Rng + ?Sized;

  fn test_docs_only<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut postings_tester = self.create_postings(random);
    postings_tester.test_full(
      random,
      &self.get_codec()?,
      create_temp_dir_with_prefix("testPostingsFormat.testExact")?,
      IndexOptions::Docs,
      false,
    )
  }

  fn test_docs_and_freqs<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut postings_tester = self.create_postings(random);
    postings_tester.test_full(
      random,
      &self.get_codec()?,
      create_temp_dir_with_prefix("testPostingsFormat.testExact")?,
      IndexOptions::DocsAndFreqs,
      false,
    )
  }

  fn test_docs_and_freqs_and_positions<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut postings_tester = self.create_postings(random);
    postings_tester.test_full(
      random,
      &self.get_codec()?,
      create_temp_dir_with_prefix("testPostingsFormat.testExact")?,
      IndexOptions::DocsAndFreqsAndPositions,
      false,
    )
  }

  fn test_docs_and_freqs_and_positions_and_payloads<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut postings_tester = self.create_postings(random);
    postings_tester.test_full(
      random,
      &self.get_codec()?,
      create_temp_dir_with_prefix("testPostingsFormat.testExact")?,
      IndexOptions::DocsAndFreqsAndPositions,
      true,
    )
  }

  fn test_docs_and_freqs_and_positions_and_offsets<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut postings_tester = self.create_postings(random);
    postings_tester.test_full(
      random,
      &self.get_codec()?,
      create_temp_dir_with_prefix("testPostingsFormat.testExact")?,
      IndexOptions::DocsAndFreqsAndPositionsAndOffsets,
      false,
    )
  }

  fn test_docs_and_freqs_and_positions_and_offsets_and_payloads<R>(
    &self,
    random: &mut R,
  ) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut postings_tester = self.create_postings(random);
    postings_tester.test_full(
      random,
      &self.get_codec()?,
      create_temp_dir_with_prefix("testPostingsFormat.testExact")?,
      IndexOptions::DocsAndFreqsAndPositionsAndOffsets,
      true,
    )
  }

  fn test_random<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let iters = 5;
    for _ in 0..iters {
      let path = create_temp_dir_with_prefix("testPostingsFormat")?;
      let dir = new_fs_directory(random, path)?;

      let index_payloads = random.random_bool(0.5);
      let mut postings_tester = self.create_postings(random);
      let fields_producer = postings_tester.build_index(
        &self.get_codec()?,
        dir.clone(),
        IndexOptions::DocsAndFreqsAndPositionsAndOffsets,
        index_payloads,
        false,
      )?;

      postings_tester.test_fields(&fields_producer)?;

      let mut opts: HashSet<Option_> = Option_::iter().collect();
      // TODO IMPORTANT 多线程不支持
      opts.remove(&Option_::Threads);

      postings_tester.test_terms(
        random,
        &fields_producer,
        &opts,
        IndexOptions::DocsAndFreqsAndPositionsAndOffsets,
        IndexOptions::DocsAndFreqsAndPositionsAndOffsets,
        false,
      )?;

      drop(fields_producer);
      drop(dir);
    }
    Ok(())
  }

  fn is_postings_enum_reuse_implemented(&self) -> bool {
    true
  }
  fn test_postings_enum_reuse<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let path = create_temp_dir_with_prefix("testPostingsEnumReuse")?;
    let dir = new_fs_directory(random, path)?;

    let mut postings_tester = self.create_postings(random);
    let fields_producer = postings_tester.build_index(
      &self.get_codec()?,
      dir.clone(),
      IndexOptions::DocsAndFreqsAndPositionsAndOffsets,
      random.random_bool(0.5),
      true,
    )?;

    let mut all_terms = postings_tester.all_terms().to_vec();
    all_terms.shuffle(random);
    let field_and_term = all_terms.into_iter().next().unwrap();

    let terms = fields_producer.terms(field_and_term.field())?.unwrap();
    let mut terms_enum = terms.iterator()?;

    assert!(terms_enum.seek_exact(field_and_term.term())?);
    self.check_reuse(&mut terms_enum, FREQS as i32, ALL as i32, false)?;
    if self.is_postings_enum_reuse_implemented() {
      self.check_reuse(&mut terms_enum, ALL as i32, ALL as i32, true)?;
    }
    Ok(())
  }

  fn check_reuse<TE>(
    &self,
    terms_enum: &mut TE,
    first_flags: i32,
    second_flags: i32,
    _should_reuse: bool,
  ) -> Result<()>
  where
    TE: TermsEnum,
    TE::PostingsEnum: PostingsEnum,
  {
    let postings1 = terms_enum.postings_with_flags(None, first_flags)?;
    let _postings2 = terms_enum.postings_with_flags(Some(postings1), second_flags)?;
    Ok(())
  }

  fn test_just_empty_field<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(random)?;
    let mut iwc = new_index_writer_config(random);
    iwc.base.codec = self.get_codec()?;
    let iw = RandomIndexWriter::with_config(random, dir, iwc);
    let mut doc = Document::new();
    let mut field_types = HashMap::new();
    doc.add(new_string_field(
      random,
      "",
      "something",
      No,
      &mut field_types,
    )?);
    iw.add_document(doc)?;
    let ir = iw.get_reader()?;
    let ar = get_only_leaf_reader(ir)?;
    assert_eq!(1, ar.get_field_infos()?.size());
    let terms = ar.terms("")?.unwrap();
    let mut terms_enum = terms.iterator()?;
    let term = terms_enum.next()?.unwrap();
    assert_eq!(term.as_ref(), &BytesRef::from_string("something"));
    assert!(terms_enum.next()?.is_none());
    Ok(())
  }

  fn test_empty_field_and_empty_term<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(random)?;
    let mut iwc = new_index_writer_config(random);
    iwc.base.codec = self.get_codec()?;
    let iw = RandomIndexWriter::with_config(random, dir, iwc);
    let mut doc = Document::new();
    let mut field_types = HashMap::new();
    doc.add(new_string_field(random, "", "", No, &mut field_types)?);
    iw.add_document(doc)?;
    let ir = iw.get_reader()?;
    let ar = get_only_leaf_reader(ir)?;
    assert_eq!(1, ar.get_field_infos()?.size());
    let terms = ar.terms("")?.unwrap();
    let mut terms_enum = terms.iterator()?;
    let term = terms_enum.next()?.unwrap();
    assert_eq!(term.as_ref(), &BytesRef::from_string(""));
    assert!(terms_enum.next()?.is_none());
    Ok(())
  }

  fn test_didnt_want_freqs_but_asked_anyway<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(random)?;
    let mut iwc = new_index_writer_config(random);
    iwc.base.codec = self.get_codec()?;
    let iw = RandomIndexWriter::with_config(random, dir, iwc);
    let mut doc = Document::new();
    let mut field_types = HashMap::new();
    doc.add(new_text_field(
      random,
      "field",
      "value",
      No,
      &mut field_types,
    )?);
    iw.add_document(doc.clone())?;
    iw.add_document(doc)?;
    let ir = iw.get_reader()?;
    let ar = get_only_leaf_reader(ir)?;
    let mut terms_enum = ar.terms("field")?.unwrap().iterator()?;
    assert!(terms_enum.seek_exact(&BytesRef::from_string("value"))?);
    let mut docs_enum = terms_enum.postings_with_flags(None, NONE as i32)?;
    assert_eq!(0, docs_enum.next_doc()?);
    assert_eq!(1, docs_enum.freq()?);
    assert_eq!(1, docs_enum.next_doc()?);
    assert_eq!(1, docs_enum.freq()?);
    Ok(())
  }

  fn test_ask_for_positions_when_not_there<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(random)?;
    let mut iwc = new_index_writer_config(random);
    iwc.base.codec = self.get_codec()?;
    let iw = RandomIndexWriter::with_config(random, dir, iwc);
    let mut doc = Document::new();
    let mut field_types = HashMap::new();
    doc.add(new_string_field(
      random,
      "field",
      "value",
      No,
      &mut field_types,
    )?);
    iw.add_document(doc.clone())?;
    iw.add_document(doc)?;
    let ir = iw.get_reader()?;
    let ar = get_only_leaf_reader(ir)?;
    let mut terms_enum = ar.terms("field")?.unwrap().iterator()?;
    assert!(terms_enum.seek_exact(&BytesRef::from_string("value"))?);
    let mut docs_enum = terms_enum.postings_with_flags(None, POSITIONS as i32)?;
    assert_eq!(0, docs_enum.next_doc()?);
    assert_eq!(1, docs_enum.freq()?);
    assert_eq!(1, docs_enum.next_doc()?);
    assert_eq!(1, docs_enum.freq()?);
    Ok(())
  }

  // tests that ghost fields still work
  // TODO: can this be improved?
  fn test_ghosts<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(random)?;
    let mut iwc = new_index_writer_config(random);
    iwc.base.codec = self.get_codec()?;
    iwc.base.merge_policy = new_log_merge_policy(random)?;
    let iw = IndexWriter::new(dir.clone(), iwc)?;
    let doc = Document::new();
    iw.add_document(doc)?;
    let mut doc = Document::new();
    let mut field_types = HashMap::new();
    doc.add(new_string_field(
      random,
      "ghostField",
      "something",
      No,
      &mut field_types,
    )?);
    iw.add_document(doc)?;
    iw.force_merge(1)?;
    iw.delete_documents_with_terms(vec![Term::from_text("ghostField", "something")])?;
    iw.force_merge(1)?;
    let ir = iw.get_reader(true, false)?;
    let ar = get_only_leaf_reader(ir)?;
    assert!(ar.get_field_infos()?.size() <= 1);
    if let Some(terms) = ar.terms("ghostField")? {
      let mut terms_enum = terms.iterator()?;
      if terms_enum.next()?.is_some() {
        let mut postings_enum = terms_enum.postings(None)?;
        assert_eq!(NO_MORE_DOCS, postings_enum.next_doc()?);
      }
    }
    Ok(())
  }

  // Test seek in disorder.
  fn test_disorder<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(random)?;
    let mut iwc = new_index_writer_config(random);
    iwc.base.codec = self.get_codec()?;
    iwc.base.merge_policy =
      crate::test::core::util::lucene_test_case::lucene_test_case_util::new_tiered_merge_policy(
        random,
      )
      .into();
    let iw = IndexWriter::new(dir.clone(), iwc)?;
    let mut field_types = HashMap::new();

    for i in 0..10000 {
      let mut document = Document::new();
      document.add(new_string_field(
        random,
        "id",
        i.to_string(),
        No,
        &mut field_types,
      )?);
      iw.add_document(document)?;
    }
    iw.commit()?;
    iw.force_merge(1)?;

    let reader = directory_reader::open(dir)?;
    let mut terms_enum = get_only_leaf_reader(reader)?
      .terms("id")?
      .unwrap()
      .iterator()?;

    for _ in 0..20000 {
      let n = random.random_range(0..10000);
      let target = BytesRef::from_string(&n.to_string());
      assert!(terms_enum.seek_exact(&target)?);
      assert_eq!(terms_enum.term()?.as_ref(), &target);
      assert_eq!(SeekStatus::Found, terms_enum.seek_ceil(&target)?);
      assert_eq!(terms_enum.term()?.as_ref(), &target);
    }

    Ok(())
  }

  fn sub_check_binary_search<TE>(&self, _terms_enum: &mut TE) -> Result<()>
  where
    TE: TermsEnum,
    TE::PostingsEnum: PostingsEnum,
  {
    Ok(())
  }

  fn test_binary_search_term_leaf<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(random)?;
    let mut iwc = new_index_writer_config(random);
    iwc.base.codec = self.get_codec()?;
    iwc.base.merge_policy = new_tiered_merge_policy(random).into();
    let iw = IndexWriter::new(dir.clone(), iwc)?;
    let mut field_types = HashMap::new();

    for i in 100000..=100400 {
      if i % 2 == 1 {
        let mut document = Document::new();
        document.add(new_string_field(
          random,
          "id",
          i.to_string(),
          No,
          &mut field_types,
        )?);
        iw.add_document(document)?;
      }
    }
    iw.commit()?;
    iw.force_merge(1)?;

    let reader = directory_reader::open(dir)?;
    let mut terms_enum = get_only_leaf_reader(reader)?
      .terms("id")?
      .unwrap()
      .iterator()?;

    for i in 100000..=100400 {
      let target = BytesRef::from_string(&i.to_string());
      if i % 2 == 1 {
        assert!(terms_enum.seek_exact(&target)?);
        assert_eq!(terms_enum.term()?.as_ref(), &target);
      } else {
        assert!(!terms_enum.seek_exact(&target)?);
      }
    }

    self.sub_check_binary_search(&mut terms_enum)?;

    for i in 100000..100400 {
      let target = BytesRef::from_string(&i.to_string());
      if i % 2 == 1 {
        assert_eq!(SeekStatus::Found, terms_enum.seek_ceil(&target)?);
        assert_eq!(terms_enum.term()?.as_ref(), &target);
        if i <= 100397 {
          let next_term = terms_enum.next()?.unwrap();
          let expected_next = BytesRef::from_string(&(i + 2).to_string());
          assert_eq!(next_term.as_ref(), &expected_next);
        }
      } else {
        assert_eq!(SeekStatus::NotFound, terms_enum.seek_ceil(&target)?);
        assert_eq!(
          terms_enum.term()?.as_ref(),
          &BytesRef::from_string(&(i + 1).to_string())
        );
      }
    }
    assert_eq!(
      SeekStatus::End,
      terms_enum.seek_ceil(&BytesRef::from_string("100400"))?
    );

    Ok(())
  }

  // tests that level 2 ghost fields still work
  fn test_level2_ghosts<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(random)?;
    let mut iwc = new_index_writer_config(random);
    iwc.base.codec = self.get_codec()?;
    iwc.base.merge_policy = new_log_merge_policy(random)?;
    let iw = IndexWriter::new(dir.clone(), iwc)?;
    let mut field_types = HashMap::new();

    let mut document = Document::new();
    document.add(new_string_field(random, "id", "0", No, &mut field_types)?);
    document.add(new_string_field(
      random,
      "suggest_field",
      "apples",
      No,
      &mut field_types,
    )?);
    iw.add_document(document)?;
    iw.add_document(Document::new())?;
    iw.commit()?;

    let mut document = Document::new();
    document.add(new_string_field(random, "id", "1", No, &mut field_types)?);
    document.add(new_string_field(
      random,
      "suggest_field2",
      "apples",
      No,
      &mut field_types,
    )?);
    iw.add_document(document)?;
    iw.commit()?;

    iw.delete_documents_with_terms(vec![Term::from_text("id", "0")])?;
    iw.force_merge(1)?;

    iw.add_document(Document::new())?;
    iw.force_merge(1)?;

    let reader = directory_reader::open(dir)?;
    let searcher = IndexSearcher::new(get_context(reader)?)?;
    assert_eq!(
      1,
      searcher.count(TermQuery::new(Term::from_text("id", "1")))?
    );

    Ok(())
  }

  fn test_inverted_write<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    // TODO IMPORTANT setCodec未实现
    Ok(())
  }

  fn test_postings_enum_docs_only<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(random)?;
    let analyzer = MockAnalyzer::new(random);
    let iwc = new_index_writer_config_with_analyzer(random, analyzer);
    let w = RandomIndexWriter::with_config(random, dir, iwc);
    let mut field_types = HashMap::new();
    let mut doc = Document::new();
    doc.add(new_string_field(
      random,
      "foo",
      "bar",
      No,
      &mut field_types,
    )?);
    w.add_document(doc)?;
    w.commit()?;

    let reader = w.get_reader()?;
    let leaf = get_only_leaf_reader(reader)?;
    let mut postings = match leaf.postings(&Term::from_text("foo", "bar"))?.unwrap() {
      PostingsEnumEnum2::A(p) => p,
      PostingsEnumEnum2::B(_) => unreachable!(),
    };
    assert_eq!(-1, postings.doc_id());
    assert_eq!(0, postings.next_doc()?);
    assert_eq!(1, postings.freq()?);
    assert_eq!(NO_MORE_DOCS, postings.next_doc()?);

    let mut terms_enum = leaf.terms("foo")?.unwrap().iterator()?;
    assert!(terms_enum.seek_exact(&BytesRef::from_string("bar"))?);
    let mut postings2 = terms_enum.postings(None)?;
    assert_eq!(-1, postings2.doc_id());
    assert_eq!(0, postings2.next_doc()?);
    assert_eq!(1, postings2.freq()?);
    assert_eq!(NO_MORE_DOCS, postings2.next_doc()?);

    for flag in [NONE as i32, FREQS as i32, POSITIONS as i32, ALL as i32] {
      let mut p = terms_enum.postings_with_flags(None, flag)?;
      assert_eq!(-1, p.doc_id());
      assert_eq!(0, p.next_doc()?);
      if flag != NONE as i32 {
        assert_eq!(1, p.freq()?);
      }
      assert_eq!(NO_MORE_DOCS, p.next_doc()?);
      let mut p2 = terms_enum.postings_with_flags(Some(p), flag)?;
      assert_eq!(-1, p2.doc_id());
      assert_eq!(0, p2.next_doc()?);
      if flag != NONE as i32 {
        assert_eq!(1, p2.freq()?);
      }
      assert_eq!(NO_MORE_DOCS, p2.next_doc()?);
    }

    Ok(())
  }

  fn test_postings_enum_freqs<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(random)?;
    // TODO IMPORTANT MockTokenizer未实现
    let analyzer = MockAnalyzer::new(random);
    let iwc = new_index_writer_config_with_analyzer(random, analyzer);
    let w = RandomIndexWriter::with_config(random, dir, iwc);

    let mut ft = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
    ft.set_index_options(IndexOptions::DocsAndFreqs)?;
    let mut doc = Document::new();
    doc.add(Field::from_string("foo", "bar bar", ft)?);
    w.add_document(doc)?;

    let reader = w.get_reader()?;
    let leaf = get_only_leaf_reader(reader)?;
    let mut postings = match leaf.postings(&Term::from_text("foo", "bar"))?.unwrap() {
      PostingsEnumEnum2::A(p) => p,
      PostingsEnumEnum2::B(_) => unreachable!(),
    };
    assert_eq!(-1, postings.doc_id());
    assert_eq!(0, postings.next_doc()?);
    assert_eq!(2, postings.freq()?);
    assert_eq!(NO_MORE_DOCS, postings.next_doc()?);

    let mut terms_enum = leaf.terms("foo")?.unwrap().iterator()?;
    assert!(terms_enum.seek_exact(&BytesRef::from_string("bar"))?);
    let mut postings2 = terms_enum.postings(None)?;
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

    for flag in [NONE as i32, FREQS as i32, POSITIONS as i32, ALL as i32] {
      let mut p = terms_enum.postings_with_flags(None, flag)?;
      assert_eq!(-1, p.doc_id());
      assert_eq!(0, p.next_doc()?);
      if flag != NONE as i32 {
        assert_eq!(2, p.freq()?);
      }
      assert_eq!(NO_MORE_DOCS, p.next_doc()?);
      let mut p2 = terms_enum.postings_with_flags(Some(p), flag)?;
      assert_eq!(-1, p2.doc_id());
      assert_eq!(0, p2.next_doc()?);
      if flag != NONE as i32 {
        assert_eq!(2, p2.freq()?);
      }
      assert_eq!(NO_MORE_DOCS, p2.next_doc()?);
    }

    Ok(())
  }

  fn test_postings_enum_positions<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(random)?;
    // TODO IMPORTANT MockTokenizer未实现
    let analyzer = MockAnalyzer::new(random);
    let iwc = new_index_writer_config_with_analyzer(random, analyzer);
    let w = RandomIndexWriter::with_config(random, dir, iwc);

    let mut doc = Document::new();
    doc.add(TextField::from_string("foo", "bar bar", Store::No)?);
    w.add_document(doc)?;

    let reader = w.get_reader()?;
    let leaf = get_only_leaf_reader(reader)?;

    let mut postings = match leaf.postings(&Term::from_text("foo", "bar"))?.unwrap() {
      PostingsEnumEnum2::A(p) => p,
      PostingsEnumEnum2::B(_) => unreachable!(),
    };
    assert_eq!(-1, postings.doc_id());
    assert_eq!(0, postings.next_doc()?);
    assert_eq!(2, postings.freq()?);
    assert_eq!(NO_MORE_DOCS, postings.next_doc()?);

    let mut terms_enum = leaf.terms("foo")?.unwrap().iterator()?;
    assert!(terms_enum.seek_exact(&BytesRef::from_string("bar"))?);

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

    Ok(())
  }

  fn test_postings_enum_offsets<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(random)?;
    // TODO IMPORTANT MockTokenizer未实现
    let analyzer = MockAnalyzer::new(random);
    let iwc = new_index_writer_config_with_analyzer(random, analyzer);
    let w = RandomIndexWriter::with_config(random, dir, iwc);

    let mut ft = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
    ft.set_index_options(IndexOptions::DocsAndFreqsAndPositionsAndOffsets)?;
    let mut doc = Document::new();
    doc.add(Field::from_string("foo", "bar bar", ft)?);
    w.add_document(doc)?;

    let reader = w.get_reader()?;
    let leaf = get_only_leaf_reader(reader)?;

    let mut postings = match leaf.postings(&Term::from_text("foo", "bar"))?.unwrap() {
      PostingsEnumEnum2::A(p) => p,
      PostingsEnumEnum2::B(_) => unreachable!(),
    };
    assert_eq!(-1, postings.doc_id());
    assert_eq!(0, postings.next_doc()?);
    assert_eq!(2, postings.freq()?);
    assert_eq!(NO_MORE_DOCS, postings.next_doc()?);

    let mut terms_enum = leaf.terms("foo")?.unwrap().iterator()?;
    assert!(terms_enum.seek_exact(&BytesRef::from_string("bar"))?);

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

    Ok(())
  }

  fn test_postings_enum_payloads<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(random)?;
    let iwc = new_index_writer_config(random);
    let w = RandomIndexWriter::with_config(random, dir, iwc);

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

    let mut doc = Document::new();
    doc.add(TextField::from_token_stream(
      "foo",
      FieldTokenStreamEnum::custom(CannedTokenStream::new(vec![token1, token2])),
    )?);
    w.add_document(doc)?;

    let reader = w.get_reader()?;
    let leaf = get_only_leaf_reader(reader)?;
    // sugar method (FREQS)
    let mut postings = match leaf.postings(&Term::from_text("foo", "bar"))?.unwrap() {
      PostingsEnumEnum2::A(p) => p,
      PostingsEnumEnum2::B(_) => unreachable!(),
    };
    assert_eq!(-1, postings.doc_id());
    assert_eq!(0, postings.next_doc()?);
    assert_eq!(2, postings.freq()?);
    assert_eq!(NO_MORE_DOCS, postings.next_doc()?);
    // termsenum reuse (FREQS)
    let mut terms_enum = leaf.terms("foo")?.unwrap().iterator()?;
    assert!(terms_enum.seek_exact(&BytesRef::from_string("bar"))?);

    let mut postings2 = terms_enum.postings(Some(postings))?;
    // and it had better work
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

    let mut docs_and_positions_enum = leaf
      .postings_with_flag(&Term::from_text("foo", "bar"), POSITIONS as i32)?
      .unwrap();
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

    let docs_and_positions_enum = match docs_and_positions_enum {
      PostingsEnumEnum2::A(p) => p,
      PostingsEnumEnum2::B(_) => unreachable!(),
    };
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

    let mut docs_and_positions_enum = leaf
      .postings_with_flag(&Term::from_text("foo", "bar"), PAYLOADS as i32)?
      .unwrap();

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

    let docs_and_positions_enum = match docs_and_positions_enum {
      PostingsEnumEnum2::A(p) => p,
      PostingsEnumEnum2::B(_) => unreachable!(),
    };
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

    let mut docs_and_positions_enum = leaf
      .postings_with_flag(&Term::from_text("foo", "bar"), OFFSETS as i32)?
      .unwrap();

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

    let docs_and_positions_enum = match docs_and_positions_enum {
      PostingsEnumEnum2::A(p) => p,
      PostingsEnumEnum2::B(_) => unreachable!(),
    };
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
    let mut docs_and_positions_enum = leaf
      .postings_with_flag(&Term::from_text("foo", "bar"), ALL as i32)?
      .unwrap();
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
    let docs_and_positions_enum = match docs_and_positions_enum {
      PostingsEnumEnum2::A(p) => p,
      PostingsEnumEnum2::B(_) => unreachable!(),
    };
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

  fn test_postings_enum_all<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    // TODO Token未实现
    Ok(())
  }

  fn test_line_file_docs<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    // TODO LineFileDocs 未实现
    Ok(())
  }

  fn test_mismatched_fields<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    // TODO MismatchedCodecReader未实现
    Ok(())
  }
}
