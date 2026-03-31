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
use crate::core::document::binary_doc_values_field::BinaryDocValuesField;
use crate::core::document::document::Document;
use crate::core::document::field::Store;
use crate::core::document::float_doc_values_field::FloatDocValuesField;
use crate::core::document::numeric_doc_values_field::NumericDocValuesField;
use crate::core::document::sorted_doc_values_field::SortedDocValuesField;
use crate::core::index::BytesRef;
use crate::core::index::binary_doc_values::BinaryDocValues;
use crate::core::index::composite_reader::get_context;
use crate::core::index::directory_reader::directory_reader_util;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::index::sorted_doc_values::SortedDocValues;
use crate::core::index::stored_fields::StoredFields;
use crate::core::index::term::Term;
use crate::core::index::terms_enum::{SeekStatus, TermsEnum};
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder as BooleanQueryBuilder;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::search::term_query::TermQuery;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::index::base_index_file_format_test_case::BaseIndexFileFormatTestCase;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  get_only_leaf_reader, new_bytes_ref_from_bytes, new_bytes_ref_from_string, new_directory_shared,
  new_index_writer_config_with_analyzer, new_log_merge_policy, new_searcher_with_reader,
  new_string_field, new_text_field,
};
use rand::{Rng, RngExt};
use std::collections::HashMap;

pub trait LegacyBaseDocValuesFormatTestCase: BaseIndexFileFormatTestCase {
  fn add_random_fields<R: Rng + ?Sized>(_random: &mut R) -> Result<()> {
    todo!()
  }
  fn test_one_number<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
    let directory = new_directory_shared(random)?;
    let iwriter = RandomIndexWriter::new(random, directory.clone());

    let mut doc = Document::new();
    let long_term = "longtermlongtermlongtermlongtermlongtermlongtermlongtermlongterm\
         longtermlongtermlongtermlongtermlongtermlongtermlongtermlongterm\
         longtermlongterm";
    let text = format!("This is the text to be indexed. {}", long_term);

    let mut field_to_type = HashMap::new();
    doc.add(new_text_field(
      random,
      "fieldname",
      &text,
      Store::Yes,
      &mut field_to_type,
    )?);
    doc.add(NumericDocValuesField::new("dv", 5));

    iwriter.add_document(doc)?;
    iwriter.close()?;

    let ireader =
      self.maybe_wrap_with_merging_reader(directory_reader_util::open(directory.clone())?)?;
    let isearcher = new_searcher_with_reader(ireader)?;
    let mut stored_fields = isearcher.stored_fields()?;

    assert_eq!(
      1,
      isearcher.count(TermQuery::new(Term::from_text("fieldname", long_term)))?
    );

    let query = TermQuery::new(Term::from_text("fieldname", "text"));
    let hits = isearcher.search(query, 1)?;
    assert_eq!(1, hits.total_hits.value());

    for i in 0..hits.score_docs.len() {
      let hit_doc = stored_fields.document(hits.score_docs[i].doc)?;
      assert_eq!(text, hit_doc.get("fieldname")?.unwrap().into_owned());

      assert_eq!(1, isearcher.get_leaf_contexts()?.len());
      let leaf = &isearcher.get_leaf_contexts()?[0];
      let mut dv = leaf.reader().get_numeric_doc_values("dv")?.unwrap();

      let doc_id = hits.score_docs[i].doc;
      assert_eq!(doc_id, dv.advance(doc_id)?);
      assert_eq!(5, dv.long_value()?);
    }
    Ok(())
  }

  fn test_one_float<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
    let directory = new_directory_shared(random)?;
    let iwriter = RandomIndexWriter::new(random, directory.clone());

    let mut doc = Document::new();
    let long_term = "longtermlongtermlongtermlongtermlongtermlongtermlongtermlongterm\
         longtermlongtermlongtermlongtermlongtermlongtermlongtermlongterm\
         longtermlongterm";
    let text = format!("This is the text to be indexed. {}", long_term);

    let mut field_to_type = HashMap::new();
    doc.add(new_text_field(
      random,
      "fieldname",
      &text,
      Store::Yes,
      &mut field_to_type,
    )?);
    doc.add(FloatDocValuesField::new("dv", 5.7f32));

    iwriter.add_document(doc)?;
    iwriter.close()?;

    let ireader =
      self.maybe_wrap_with_merging_reader(directory_reader_util::open(directory.clone())?)?;
    let isearcher = new_searcher_with_reader(ireader)?;
    let mut stored_fields = isearcher.stored_fields()?;

    assert_eq!(
      1,
      isearcher.count(TermQuery::new(Term::from_text("fieldname", long_term)))?
    );

    let query = TermQuery::new(Term::from_text("fieldname", "text"));
    let hits = isearcher.search(query, 1)?;
    assert_eq!(1, hits.total_hits.value());

    for i in 0..hits.score_docs.len() {
      let doc_id = hits.score_docs[i].doc;
      let hit_doc = stored_fields.document(doc_id)?;
      assert_eq!(text, hit_doc.get("fieldname")?.unwrap().into_owned());

      assert_eq!(1, isearcher.get_leaf_contexts()?.len());
      let leaf = &isearcher.get_leaf_contexts()?[0];
      let mut dv = leaf.reader().get_numeric_doc_values("dv")?.unwrap();

      assert_eq!(doc_id, dv.advance(doc_id)?);
      assert_eq!(5.7f32.to_bits() as i32 as i64, dv.long_value()?);
    }
    Ok(())
  }

  fn test_two_numbers<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
    let directory = new_directory_shared(random)?;
    let iwriter = RandomIndexWriter::new(random, directory.clone());

    let mut doc = Document::new();
    let long_term = "longtermlongtermlongtermlongtermlongtermlongtermlongtermlongterm\
         longtermlongtermlongtermlongtermlongtermlongtermlongtermlongterm\
         longtermlongterm";
    let text = format!("This is the text to be indexed. {}", long_term);

    let mut field_to_type = HashMap::new();
    doc.add(new_text_field(
      random,
      "fieldname",
      &text,
      Store::Yes,
      &mut field_to_type,
    )?);
    doc.add(NumericDocValuesField::new("dv1", 5));
    doc.add(NumericDocValuesField::new("dv2", 17));

    iwriter.add_document(doc)?;
    iwriter.close()?;

    let ireader =
      self.maybe_wrap_with_merging_reader(directory_reader_util::open(directory.clone())?)?;
    let isearcher = new_searcher_with_reader(ireader)?;
    let mut stored_fields = isearcher.stored_fields()?;

    assert_eq!(
      1,
      isearcher.count(TermQuery::new(Term::from_text("fieldname", long_term)))?
    );

    let query = TermQuery::new(Term::from_text("fieldname", "text"));
    let hits = isearcher.search(query, 1)?;
    assert_eq!(1, hits.total_hits.value());

    for i in 0..hits.score_docs.len() {
      let doc_id = hits.score_docs[i].doc;
      let hit_doc = stored_fields.document(doc_id)?;
      assert_eq!(text, hit_doc.get("fieldname")?.unwrap().into_owned());

      assert_eq!(1, isearcher.get_leaf_contexts()?.len());
      let leaf = &isearcher.get_leaf_contexts()?[0];

      let mut dv = leaf.reader().get_numeric_doc_values("dv1")?.unwrap();
      assert_eq!(doc_id, dv.advance(doc_id)?);
      assert_eq!(5, dv.long_value()?);

      let mut dv = leaf.reader().get_numeric_doc_values("dv2")?.unwrap();
      assert_eq!(doc_id, dv.advance(doc_id)?);
      assert_eq!(17, dv.long_value()?);
    }
    Ok(())
  }

  fn test_two_binary_values<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
    let directory = new_directory_shared(random)?;
    let iwriter = RandomIndexWriter::new(random, directory.clone());

    let mut doc = Document::new();
    let long_term = "longtermlongtermlongtermlongtermlongtermlongtermlongtermlongterm\
         longtermlongtermlongtermlongtermlongtermlongtermlongtermlongterm\
         longtermlongterm";
    let text = format!("This is the text to be indexed. {}", long_term);

    let mut field_to_type = HashMap::new();
    doc.add(new_text_field(
      random,
      "fieldname",
      &text,
      Store::Yes,
      &mut field_to_type,
    )?);
    doc.add(BinaryDocValuesField::new(
      "dv1",
      new_bytes_ref_from_string(random, long_term)?,
    ));
    doc.add(BinaryDocValuesField::new(
      "dv2",
      new_bytes_ref_from_string(random, &text)?,
    ));

    iwriter.add_document(doc)?;
    iwriter.close()?;

    let ireader =
      self.maybe_wrap_with_merging_reader(directory_reader_util::open(directory.clone())?)?;
    let isearcher = new_searcher_with_reader(ireader)?;
    let mut stored_fields = isearcher.stored_fields()?;

    assert_eq!(
      1,
      isearcher.count(TermQuery::new(Term::from_text("fieldname", long_term)))?
    );

    let query = TermQuery::new(Term::from_text("fieldname", "text"));
    let hits = isearcher.search(query, 1)?;
    assert_eq!(1, hits.total_hits.value());

    for i in 0..hits.score_docs.len() {
      let hit_doc_id = hits.score_docs[i].doc;
      let hit_doc = stored_fields.document(hit_doc_id)?;
      assert_eq!(text, hit_doc.get("fieldname")?.unwrap().into_owned());

      assert_eq!(1, isearcher.get_leaf_contexts()?.len());
      let leaf = &isearcher.get_leaf_contexts()?[0];

      let mut dv = leaf.reader().get_binary_doc_values("dv1")?.unwrap();
      assert_eq!(hit_doc_id, dv.advance(hit_doc_id)?);
      assert_eq!(
        &new_bytes_ref_from_string(random, long_term)?,
        dv.binary_value()?.as_ref()
      );

      let mut dv = leaf.reader().get_binary_doc_values("dv2")?.unwrap();
      assert_eq!(hit_doc_id, dv.advance(hit_doc_id)?);
      assert_eq!(
        &new_bytes_ref_from_string(random, &text)?,
        dv.binary_value()?.as_ref()
      );
    }
    Ok(())
  }

  fn test_variously_compressible_binary_values<R: Rng + ?Sized>(
    &self,
    random: &mut R,
  ) -> Result<()> {
    let directory = new_directory_shared(random)?;
    let iwriter = RandomIndexWriter::new(random, directory.clone());
    let num_docs = 1 + random.random_range(0..100);

    let mut written_values: HashMap<i32, BytesRef<Vec<u8>>> = HashMap::new();

    let vocab_range = 1 + random.random_range(0..(u8::MAX as usize - 1));

    for i in 0..num_docs {
      let mut doc = Document::new();
      let mut field_to_type = HashMap::new();

      let mut value = vec![0u8; 500 + random.random_range(0..1024)];
      for b in &mut value {
        *b = random.random_range(0..vocab_range) as u8;
      }

      let bytes_ref = new_bytes_ref_from_bytes(random, value.as_ref())?;
      written_values.insert(i, bytes_ref.clone());

      doc.add(new_text_field(
        random,
        "id",
        i.to_string(),
        Store::Yes,
        &mut field_to_type,
      )?);
      doc.add(BinaryDocValuesField::new("dv1", bytes_ref));
      iwriter.add_document(doc)?;
    }
    iwriter.force_merge(1)?;
    iwriter.close()?;

    let ireader =
      self.maybe_wrap_with_merging_reader(directory_reader_util::open(directory.clone())?)?;
    let isearcher = new_searcher_with_reader(ireader)?;
    let mut stored_fields = isearcher.stored_fields()?;

    for i in 0..num_docs {
      let id = i.to_string();
      let query = TermQuery::new(Term::from_text("id", &id));
      let hits = isearcher.search(query, 1)?;
      assert_eq!(1, hits.total_hits.value());

      let hit_doc_id = hits.score_docs[0].doc;
      let hit_doc = stored_fields.document(hit_doc_id)?;
      assert_eq!(id, hit_doc.get("id")?.unwrap().into_owned());

      assert_eq!(1, isearcher.get_leaf_contexts()?.len());
      let leaf = &isearcher.get_leaf_contexts()?[0];
      let mut dv = leaf.reader().get_binary_doc_values("dv1")?.unwrap();
      assert_eq!(hit_doc_id, dv.advance(hit_doc_id)?);
      assert_eq!(
        written_values.get(&{ i }).unwrap(),
        dv.binary_value()?.as_ref()
      );
    }
    Ok(())
  }

  fn test_two_fields_mixed<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
    let directory = new_directory_shared(random)?;
    let iwriter = RandomIndexWriter::new(random, directory.clone());

    let mut doc = Document::new();
    let long_term = "longtermlongtermlongtermlongtermlongtermlongtermlongtermlongterm\
         longtermlongtermlongtermlongtermlongtermlongtermlongtermlongterm\
         longtermlongterm";
    let text = format!("This is the text to be indexed. {}", long_term);

    let mut field_to_type = HashMap::new();
    doc.add(new_text_field(
      random,
      "fieldname",
      &text,
      Store::Yes,
      &mut field_to_type,
    )?);
    doc.add(NumericDocValuesField::new("dv1", 5));
    doc.add(BinaryDocValuesField::new(
      "dv2",
      new_bytes_ref_from_string(random, "hello world")?,
    ));

    iwriter.add_document(doc)?;
    iwriter.close()?;

    let ireader =
      self.maybe_wrap_with_merging_reader(directory_reader_util::open(directory.clone())?)?;
    let isearcher = new_searcher_with_reader(ireader)?;
    let mut stored_fields = isearcher.stored_fields()?;

    assert_eq!(
      1,
      isearcher.count(TermQuery::new(Term::from_text("fieldname", long_term)))?
    );

    let query = TermQuery::new(Term::from_text("fieldname", "text"));
    let hits = isearcher.search(query, 1)?;
    assert_eq!(1, hits.total_hits.value());

    for i in 0..hits.score_docs.len() {
      let doc_id = hits.score_docs[i].doc;
      let hit_doc = stored_fields.document(doc_id)?;
      assert_eq!(text, hit_doc.get("fieldname")?.unwrap().into_owned());

      assert_eq!(1, isearcher.get_leaf_contexts()?.len());
      let leaf = &isearcher.get_leaf_contexts()?[0];

      let mut dv = leaf.reader().get_numeric_doc_values("dv1")?.unwrap();
      assert_eq!(doc_id, dv.advance(doc_id)?);
      assert_eq!(5, dv.long_value()?);

      let mut dv2 = leaf.reader().get_binary_doc_values("dv2")?.unwrap();
      assert_eq!(doc_id, dv2.advance(doc_id)?);
      assert_eq!(
        &new_bytes_ref_from_string(random, "hello world")?,
        dv2.binary_value()?.as_ref()
      );
    }
    Ok(())
  }

  fn test_three_fields_mixed<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
    let directory = new_directory_shared(random)?;
    let iwriter = RandomIndexWriter::new(random, directory.clone());

    let mut doc = Document::new();
    let long_term = "longtermlongtermlongtermlongtermlongtermlongtermlongtermlongterm\
         longtermlongtermlongtermlongtermlongtermlongtermlongtermlongterm\
         longtermlongterm";
    let text = format!("This is the text to be indexed. {}", long_term);

    let mut field_to_type = HashMap::new();
    doc.add(new_text_field(
      random,
      "fieldname",
      &text,
      Store::Yes,
      &mut field_to_type,
    )?);
    doc.add(SortedDocValuesField::new(
      "dv1",
      new_bytes_ref_from_string(random, "hello hello")?,
    ));
    doc.add(NumericDocValuesField::new("dv2", 5));
    doc.add(BinaryDocValuesField::new(
      "dv3",
      new_bytes_ref_from_string(random, "hello world")?,
    ));

    iwriter.add_document(doc)?;
    iwriter.close()?;

    let ireader =
      self.maybe_wrap_with_merging_reader(directory_reader_util::open(directory.clone())?)?;
    let isearcher = new_searcher_with_reader(ireader)?;

    assert_eq!(
      1,
      isearcher.count(TermQuery::new(Term::from_text("fieldname", long_term)))?
    );
    let query = TermQuery::new(Term::from_text("fieldname", "text"));
    let hits = isearcher.search(query, 1)?;
    let mut stored_fields = isearcher.stored_fields()?;
    assert_eq!(1, hits.total_hits.value());

    for i in 0..hits.score_docs.len() {
      let doc_id = hits.score_docs[i].doc;
      let hit_doc = stored_fields.document(doc_id)?;
      assert_eq!(text, hit_doc.get("fieldname")?.unwrap().into_owned());

      assert_eq!(1, isearcher.get_leaf_contexts()?.len());
      let leaf = &isearcher.get_leaf_contexts()?[0];

      let mut dv = leaf.reader().get_sorted_doc_values("dv1")?.unwrap();
      assert_eq!(doc_id, dv.advance(doc_id)?);
      let ord = dv.ord_value()?;
      assert_eq!(
        &new_bytes_ref_from_string(random, "hello hello")?,
        dv.lookup_ord(ord)?.as_ref()
      );

      let mut dv2 = leaf.reader().get_numeric_doc_values("dv2")?.unwrap();
      assert_eq!(doc_id, dv2.advance(doc_id)?);
      assert_eq!(5, dv2.long_value()?);

      let mut dv3 = leaf.reader().get_binary_doc_values("dv3")?.unwrap();
      assert_eq!(doc_id, dv3.advance(doc_id)?);
      assert_eq!(
        &new_bytes_ref_from_string(random, "hello world")?,
        dv3.binary_value()?.as_ref()
      );
    }
    Ok(())
  }

  fn test_three_fields_mixed2<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
    let directory = new_directory_shared(random)?;
    let iwriter = RandomIndexWriter::new(random, directory.clone());

    let mut doc = Document::new();
    let long_term = "longtermlongtermlongtermlongtermlongtermlongtermlongtermlongterm\
         longtermlongtermlongtermlongtermlongtermlongtermlongtermlongterm\
         longtermlongterm";
    let text = format!("This is the text to be indexed. {}", long_term);

    let mut field_to_type = HashMap::new();
    doc.add(new_text_field(
      random,
      "fieldname",
      &text,
      Store::Yes,
      &mut field_to_type,
    )?);
    doc.add(BinaryDocValuesField::new(
      "dv1",
      new_bytes_ref_from_string(random, "hello world")?,
    ));
    doc.add(SortedDocValuesField::new(
      "dv2",
      new_bytes_ref_from_string(random, "hello hello")?,
    ));
    doc.add(NumericDocValuesField::new("dv3", 5));

    iwriter.add_document(doc)?;
    iwriter.close()?;

    let ireader =
      self.maybe_wrap_with_merging_reader(directory_reader_util::open(directory.clone())?)?;
    let isearcher = new_searcher_with_reader(ireader)?;
    let mut stored_fields = isearcher.stored_fields()?;

    assert_eq!(
      1,
      isearcher.count(TermQuery::new(Term::from_text("fieldname", long_term)))?
    );
    let query = TermQuery::new(Term::from_text("fieldname", "text"));
    let hits = isearcher.search(query, 1)?;
    assert_eq!(1, hits.total_hits.value());

    for i in 0..hits.score_docs.len() {
      let doc_id = hits.score_docs[i].doc;
      let hit_doc = stored_fields.document(doc_id)?;
      assert_eq!(text, hit_doc.get("fieldname")?.unwrap().into_owned());

      assert_eq!(1, isearcher.get_leaf_contexts()?.len());
      let leaf = &isearcher.get_leaf_contexts()?[0];

      let mut dv = leaf.reader().get_sorted_doc_values("dv2")?.unwrap();
      assert_eq!(doc_id, dv.advance(doc_id)?);
      let ord = dv.ord_value()?;
      assert_eq!(
        &new_bytes_ref_from_string(random, "hello hello")?,
        dv.lookup_ord(ord)?.as_ref()
      );

      let mut dv2 = leaf.reader().get_numeric_doc_values("dv3")?.unwrap();
      assert_eq!(doc_id, dv2.advance(doc_id)?);
      assert_eq!(5, dv2.long_value()?);

      let mut dv3 = leaf.reader().get_binary_doc_values("dv1")?.unwrap();
      assert_eq!(doc_id, dv3.advance(doc_id)?);
      assert_eq!(
        &new_bytes_ref_from_string(random, "hello world")?,
        dv3.binary_value()?.as_ref()
      );
    }
    Ok(())
  }

  fn test_two_documents_numeric<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
    let analyzer = MockAnalyzer::new(random);

    let directory = new_directory_shared(random)?;
    let mut conf = new_index_writer_config_with_analyzer(random, analyzer);
    conf.set_merge_policy(new_log_merge_policy(random)?);
    let iwriter = RandomIndexWriter::with_config(random, directory.clone(), conf);

    let mut doc = Document::new();
    doc.add(NumericDocValuesField::new("dv", 1));
    iwriter.add_document(doc)?;

    let mut doc = Document::new();
    doc.add(NumericDocValuesField::new("dv", 2));
    iwriter.add_document(doc)?;
    iwriter.force_merge(1)?;
    iwriter.close()?;

    let ireader =
      self.maybe_wrap_with_merging_reader(directory_reader_util::open(directory.clone())?)?;
    let top_reader_context = get_context(&ireader)?;
    assert_eq!(1, top_reader_context.leaves()?.len());
    let leaves = top_reader_context.leaves()?;
    let mut dv = leaves[0].reader().get_numeric_doc_values("dv")?.unwrap();
    assert_eq!(0, dv.next_doc()?);
    assert_eq!(1, dv.long_value()?);
    assert_eq!(1, dv.next_doc()?);
    assert_eq!(2, dv.long_value()?);
    Ok(())
  }

  fn test_two_documents_merged<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
    let analyzer = MockAnalyzer::new(random);

    let directory = new_directory_shared(random)?;
    let mut conf = new_index_writer_config_with_analyzer(random, analyzer);
    conf.set_merge_policy(new_log_merge_policy(random)?);
    let iwriter = RandomIndexWriter::with_config(random, directory.clone(), conf);

    let mut doc = Document::new();
    let mut field_to_type = HashMap::new();
    doc.add(new_string_field(
      random,
      "id",
      "0",
      Store::Yes,
      &mut field_to_type,
    )?);
    doc.add(NumericDocValuesField::new("dv", -10));
    iwriter.add_document(doc)?;
    iwriter.commit()?;

    let mut doc = Document::new();
    let mut field_to_type = HashMap::new();
    doc.add(new_string_field(
      random,
      "id",
      "1",
      Store::Yes,
      &mut field_to_type,
    )?);
    doc.add(NumericDocValuesField::new("dv", 99));
    iwriter.add_document(doc)?;
    iwriter.force_merge(1)?;
    iwriter.close()?;

    let ireader =
      self.maybe_wrap_with_merging_reader(directory_reader_util::open(directory.clone())?)?;
    let top_reader_context = get_context(&ireader)?;
    assert_eq!(1, top_reader_context.leaves()?.len());
    let leaves = top_reader_context.leaves()?;
    let leaf = leaves[0].reader();
    let mut dv = leaf.get_numeric_doc_values("dv")?.unwrap();
    let mut stored_fields = leaf.stored_fields()?;

    for i in 0..2 {
      let doc2 = stored_fields.document(i)?;
      let expected = if doc2.get("id")?.unwrap().as_ref() == "0" {
        -10
      } else {
        99
      };
      assert_eq!(i, dv.next_doc()?);
      assert_eq!(expected, dv.long_value()?);
    }
    Ok(())
  }

  fn test_big_numeric_range<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
    let analyzer = MockAnalyzer::new(random);

    let directory = new_directory_shared(random)?;
    let mut conf = new_index_writer_config_with_analyzer(random, analyzer);
    conf.set_merge_policy(new_log_merge_policy(random)?);
    let iwriter = RandomIndexWriter::with_config(random, directory.clone(), conf);

    let mut doc = Document::new();
    doc.add(NumericDocValuesField::new("dv", i64::MIN));
    iwriter.add_document(doc)?;

    let mut doc = Document::new();
    doc.add(NumericDocValuesField::new("dv", i64::MAX));
    iwriter.add_document(doc)?;
    iwriter.force_merge(1)?;
    iwriter.close()?;

    let ireader =
      self.maybe_wrap_with_merging_reader(directory_reader_util::open(directory.clone())?)?;
    let top_reader_context = get_context(&ireader)?;
    assert_eq!(1, top_reader_context.leaves()?.len());
    let leaves = top_reader_context.leaves()?;
    let mut dv = leaves[0].reader().get_numeric_doc_values("dv")?.unwrap();
    assert_eq!(0, dv.next_doc()?);
    assert_eq!(i64::MIN, dv.long_value()?);
    assert_eq!(1, dv.next_doc()?);
    assert_eq!(i64::MAX, dv.long_value()?);
    Ok(())
  }

  fn test_big_numeric_range2<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
    let analyzer = MockAnalyzer::new(random);

    let directory = new_directory_shared(random)?;
    let mut conf = new_index_writer_config_with_analyzer(random, analyzer);
    conf.set_merge_policy(new_log_merge_policy(random)?);
    let iwriter = RandomIndexWriter::with_config(random, directory.clone(), conf);

    let mut doc = Document::new();
    doc.add(NumericDocValuesField::new("dv", -8841491950446638677));
    iwriter.add_document(doc)?;

    let mut doc = Document::new();
    doc.add(NumericDocValuesField::new("dv", 9062230939892376225));
    iwriter.add_document(doc)?;
    iwriter.force_merge(1)?;
    iwriter.close()?;

    let ireader =
      self.maybe_wrap_with_merging_reader(directory_reader_util::open(directory.clone())?)?;
    let top_reader_context = get_context(&ireader)?;
    assert_eq!(1, top_reader_context.leaves()?.len());
    let leaves = top_reader_context.leaves()?;
    let mut dv = leaves[0].reader().get_numeric_doc_values("dv")?.unwrap();
    assert_eq!(0, dv.next_doc()?);
    assert_eq!(-8841491950446638677, dv.long_value()?);
    assert_eq!(1, dv.next_doc()?);
    assert_eq!(9062230939892376225, dv.long_value()?);
    Ok(())
  }

  fn test_bytes<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
    let analyzer = MockAnalyzer::new(random);

    let directory = new_directory_shared(random)?;
    let conf = new_index_writer_config_with_analyzer(random, analyzer);
    let iwriter = RandomIndexWriter::with_config(random, directory.clone(), conf);

    let mut doc = Document::new();
    let long_term = "longtermlongtermlongtermlongtermlongtermlongtermlongtermlongterm\
         longtermlongtermlongtermlongtermlongtermlongtermlongtermlongterm\
         longtermlongterm";
    let text = format!("This is the text to be indexed. {}", long_term);

    let mut field_to_type = HashMap::new();
    doc.add(new_text_field(
      random,
      "fieldname",
      &text,
      Store::Yes,
      &mut field_to_type,
    )?);
    doc.add(BinaryDocValuesField::new(
      "dv",
      new_bytes_ref_from_string(random, "hello world")?,
    ));
    iwriter.add_document(doc)?;
    iwriter.close()?;

    let ireader =
      self.maybe_wrap_with_merging_reader(directory_reader_util::open(directory.clone())?)?;
    let isearcher = new_searcher_with_reader(ireader)?;
    let mut stored_fields = isearcher.stored_fields()?;

    assert_eq!(
      1,
      isearcher.count(TermQuery::new(Term::from_text("fieldname", long_term)))?
    );
    let query = TermQuery::new(Term::from_text("fieldname", "text"));
    let hits = isearcher.search(query, 1)?;
    assert_eq!(1, hits.total_hits.value());

    for i in 0..hits.score_docs.len() {
      let hit_doc_id = hits.score_docs[i].doc;
      let hit_doc = stored_fields.document(hit_doc_id)?;
      assert_eq!(text, hit_doc.get("fieldname")?.unwrap().into_owned());

      assert_eq!(1, isearcher.get_leaf_contexts()?.len());
      let leaf = &isearcher.get_leaf_contexts()?[0];
      let mut dv = leaf.reader().get_binary_doc_values("dv")?.unwrap();
      assert_eq!(hit_doc_id, dv.advance(hit_doc_id)?);
      assert_eq!(
        &new_bytes_ref_from_string(random, "hello world")?,
        dv.binary_value()?.as_ref()
      );
    }
    Ok(())
  }

  fn test_bytes_two_documents_merged<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
    let analyzer = MockAnalyzer::new(random);

    let directory = new_directory_shared(random)?;
    let mut conf = new_index_writer_config_with_analyzer(random, analyzer);
    conf.set_merge_policy(new_log_merge_policy(random)?);
    let iwriter = RandomIndexWriter::with_config(random, directory.clone(), conf);

    let mut doc = Document::new();
    let mut field_to_type = HashMap::new();
    doc.add(new_string_field(
      random,
      "id",
      "0",
      Store::Yes,
      &mut field_to_type,
    )?);
    doc.add(BinaryDocValuesField::new(
      "dv",
      new_bytes_ref_from_string(random, "hello world 1")?,
    ));
    iwriter.add_document(doc)?;
    iwriter.commit()?;

    let mut doc = Document::new();
    let mut field_to_type = HashMap::new();
    doc.add(new_string_field(
      random,
      "id",
      "1",
      Store::Yes,
      &mut field_to_type,
    )?);
    doc.add(BinaryDocValuesField::new(
      "dv",
      new_bytes_ref_from_string(random, "hello 2")?,
    ));
    iwriter.add_document(doc)?;
    iwriter.force_merge(1)?;
    iwriter.close()?;

    let ireader =
      self.maybe_wrap_with_merging_reader(directory_reader_util::open(directory.clone())?)?;
    let top_reader_context = get_context(&ireader)?;
    assert_eq!(1, top_reader_context.leaves()?.len());
    let leaves = top_reader_context.leaves()?;
    let leaf = leaves[0].reader();
    let mut dv = leaf.get_binary_doc_values("dv")?.unwrap();
    let mut stored_fields = leaf.stored_fields()?;
    for i in 0..2 {
      let doc2 = stored_fields.document(i)?;
      let expected = if doc2.get("id")?.unwrap().as_ref() == "0" {
        "hello world 1"
      } else {
        "hello 2"
      };
      assert_eq!(i, dv.next_doc()?);
      assert_eq!(expected, dv.binary_value()?.utf8_to_string()?);
    }
    Ok(())
  }

  fn test_bytes_merge_away_all_values<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
    let directory = new_directory_shared(random)?;
    let analyzer = MockAnalyzer::new(random);
    let mut iwconfig = new_index_writer_config_with_analyzer(random, analyzer);
    iwconfig.set_merge_policy(new_log_merge_policy(random)?);
    let iwriter = RandomIndexWriter::with_config(random, directory.clone(), iwconfig);

    let mut doc = Document::new();
    let mut field_to_type = HashMap::new();
    doc.add(new_string_field(
      random,
      "id",
      "0",
      Store::No,
      &mut field_to_type,
    )?);
    iwriter.add_document(doc)?;

    let mut doc = Document::new();
    let mut field_to_type = HashMap::new();
    doc.add(new_string_field(
      random,
      "id",
      "1",
      Store::No,
      &mut field_to_type,
    )?);
    doc.add(BinaryDocValuesField::new(
      "field",
      new_bytes_ref_from_string(random, "hi")?,
    ));
    iwriter.add_document(doc)?;
    iwriter.commit()?;
    iwriter.delete_documents_with_terms(vec![Term::from_text("id", "1")])?;
    iwriter.force_merge(1)?;

    let ireader = iwriter.get_reader()?;
    iwriter.close()?;

    let mut dv = get_only_leaf_reader(&ireader)?
      .get_binary_doc_values("field")?
      .unwrap();
    assert_eq!(NO_MORE_DOCS, dv.next_doc()?);
    Ok(())
  }

  fn test_sorted_bytes<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
    let analyzer = MockAnalyzer::new(random);

    let directory = new_directory_shared(random)?;
    let conf = new_index_writer_config_with_analyzer(random, analyzer);
    let iwriter = RandomIndexWriter::with_config(random, directory.clone(), conf);

    let mut doc = Document::new();
    let long_term = "longtermlongtermlongtermlongtermlongtermlongtermlongtermlongterm\
         longtermlongtermlongtermlongtermlongtermlongtermlongtermlongterm\
         longtermlongterm";
    let text = format!("This is the text to be indexed. {}", long_term);
    let mut field_to_type = HashMap::new();
    doc.add(new_text_field(
      random,
      "fieldname",
      &text,
      Store::Yes,
      &mut field_to_type,
    )?);
    doc.add(SortedDocValuesField::new(
      "dv",
      new_bytes_ref_from_string(random, "hello world")?,
    ));
    iwriter.add_document(doc)?;
    iwriter.close()?;

    let ireader =
      self.maybe_wrap_with_merging_reader(directory_reader_util::open(directory.clone())?)?;
    let isearcher = new_searcher_with_reader(ireader)?;

    assert_eq!(
      1,
      isearcher.count(TermQuery::new(Term::from_text("fieldname", long_term)))?
    );
    let query = TermQuery::new(Term::from_text("fieldname", "text"));
    let hits = isearcher.search(query, 1)?;
    assert_eq!(1, hits.total_hits.value());
    let mut stored_fields = isearcher.stored_fields()?;
    for i in 0..hits.score_docs.len() {
      let doc_id = hits.score_docs[i].doc;
      let hit_doc = stored_fields.document(doc_id)?;
      assert_eq!(text, hit_doc.get("fieldname")?.unwrap().into_owned());
      assert_eq!(1, isearcher.get_leaf_contexts()?.len());
      let leaf = &isearcher.get_leaf_contexts()?[0];
      let mut dv = leaf.reader().get_sorted_doc_values("dv")?.unwrap();
      assert_eq!(doc_id, dv.advance(doc_id)?);
      let ord = dv.ord_value()?;
      assert_eq!(
        &new_bytes_ref_from_string(random, "hello world")?,
        dv.lookup_ord(ord)?.as_ref()
      );
    }
    Ok(())
  }

  fn test_sorted_bytes_two_documents<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
    let analyzer = MockAnalyzer::new(random);

    let directory = new_directory_shared(random)?;
    let mut conf = new_index_writer_config_with_analyzer(random, analyzer);
    conf.set_merge_policy(new_log_merge_policy(random)?);
    let iwriter = RandomIndexWriter::with_config(random, directory.clone(), conf);

    let mut doc = Document::new();
    doc.add(SortedDocValuesField::new(
      "dv",
      new_bytes_ref_from_string(random, "hello world 1")?,
    ));
    iwriter.add_document(doc)?;

    let mut doc = Document::new();
    doc.add(SortedDocValuesField::new(
      "dv",
      new_bytes_ref_from_string(random, "hello world 2")?,
    ));
    iwriter.add_document(doc)?;
    iwriter.force_merge(1)?;
    iwriter.close()?;

    let ireader =
      self.maybe_wrap_with_merging_reader(directory_reader_util::open(directory.clone())?)?;
    let top_reader_context = get_context(&ireader)?;
    assert_eq!(1, top_reader_context.leaves()?.len());
    let leaves = top_reader_context.leaves()?;
    let mut dv = leaves[0].reader().get_sorted_doc_values("dv")?.unwrap();
    assert_eq!(0, dv.next_doc()?);
    let ord = dv.ord_value()?;
    assert_eq!("hello world 1", dv.lookup_ord(ord)?.utf8_to_string()?);
    assert_eq!(1, dv.next_doc()?);
    let ord = dv.ord_value()?;
    assert_eq!("hello world 2", dv.lookup_ord(ord)?.utf8_to_string()?);
    Ok(())
  }

  fn test_sorted_bytes_three_documents<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
    let analyzer = MockAnalyzer::new(random);

    let directory = new_directory_shared(random)?;
    let mut conf = new_index_writer_config_with_analyzer(random, analyzer);
    conf.set_merge_policy(new_log_merge_policy(random)?);
    let iwriter = RandomIndexWriter::with_config(random, directory.clone(), conf);

    let mut doc = Document::new();
    doc.add(SortedDocValuesField::new(
      "dv",
      new_bytes_ref_from_string(random, "hello world 1")?,
    ));
    iwriter.add_document(doc)?;

    let mut doc = Document::new();
    doc.add(SortedDocValuesField::new(
      "dv",
      new_bytes_ref_from_string(random, "hello world 2")?,
    ));
    iwriter.add_document(doc)?;

    let mut doc = Document::new();
    doc.add(SortedDocValuesField::new(
      "dv",
      new_bytes_ref_from_string(random, "hello world 1")?,
    ));
    iwriter.add_document(doc)?;
    iwriter.force_merge(1)?;
    iwriter.close()?;

    let ireader =
      self.maybe_wrap_with_merging_reader(directory_reader_util::open(directory.clone())?)?;
    let top_reader_context = get_context(&ireader)?;
    assert_eq!(1, top_reader_context.leaves()?.len());
    let leaves = top_reader_context.leaves()?;
    let mut dv = leaves[0].reader().get_sorted_doc_values("dv")?.unwrap();
    assert_eq!(2, dv.get_value_count()?);
    assert_eq!(0, dv.next_doc()?);
    assert_eq!(0, dv.ord_value()?);
    assert_eq!("hello world 1", dv.lookup_ord(0)?.utf8_to_string()?);
    assert_eq!(1, dv.next_doc()?);
    assert_eq!(1, dv.ord_value()?);
    assert_eq!("hello world 2", dv.lookup_ord(1)?.utf8_to_string()?);
    assert_eq!(2, dv.next_doc()?);
    assert_eq!(0, dv.ord_value()?);
    Ok(())
  }

  fn test_sorted_bytes_two_documents_merged<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
    let analyzer = MockAnalyzer::new(random);

    let directory = new_directory_shared(random)?;
    let mut conf = new_index_writer_config_with_analyzer(random, analyzer);
    conf.set_merge_policy(new_log_merge_policy(random)?);
    let iwriter = RandomIndexWriter::with_config(random, directory.clone(), conf);

    let mut doc = Document::new();
    let mut field_to_type = HashMap::new();
    doc.add(new_string_field(
      random,
      "id",
      "0",
      Store::Yes,
      &mut field_to_type,
    )?);
    doc.add(SortedDocValuesField::new(
      "dv",
      new_bytes_ref_from_string(random, "hello world 1")?,
    ));
    iwriter.add_document(doc)?;
    iwriter.commit()?;

    let mut doc = Document::new();
    let mut field_to_type = HashMap::new();
    doc.add(new_string_field(
      random,
      "id",
      "1",
      Store::Yes,
      &mut field_to_type,
    )?);
    doc.add(SortedDocValuesField::new(
      "dv",
      new_bytes_ref_from_string(random, "hello world 2")?,
    ));
    iwriter.add_document(doc)?;
    iwriter.force_merge(1)?;
    iwriter.close()?;

    let ireader =
      self.maybe_wrap_with_merging_reader(directory_reader_util::open(directory.clone())?)?;
    let top_reader_context = get_context(&ireader)?;
    assert_eq!(1, top_reader_context.leaves()?.len());
    let leaves = top_reader_context.leaves()?;
    let leaf = leaves[0].reader();
    let mut dv = leaf.get_sorted_doc_values("dv")?.unwrap();
    assert_eq!(2, dv.get_value_count()?);
    assert_eq!(0, dv.next_doc()?);
    let ord = dv.ord_value()?;
    assert_eq!(
      &new_bytes_ref_from_string(random, "hello world 1")?,
      dv.lookup_ord(ord)?.as_ref()
    );
    assert_eq!(
      &new_bytes_ref_from_string(random, "hello world 2")?,
      dv.lookup_ord(1)?.as_ref()
    );
    let mut stored_fields = leaf.stored_fields()?;
    for i in 0..2 {
      let doc2 = stored_fields.document(i)?;
      let expected = if doc2.get("id")?.unwrap().as_ref() == "0" {
        "hello world 1"
      } else {
        "hello world 2"
      };
      if dv.doc_id() < i {
        assert_eq!(i, dv.next_doc()?);
      }
      let ord = dv.ord_value()?;
      assert_eq!(expected, dv.lookup_ord(ord)?.utf8_to_string()?);
    }
    Ok(())
  }

  fn test_sorted_merge_away_all_values<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
    let directory = new_directory_shared(random)?;
    let analyzer = MockAnalyzer::new(random);
    let mut iwconfig = new_index_writer_config_with_analyzer(random, analyzer);
    iwconfig.set_merge_policy(new_log_merge_policy(random)?);
    let iwriter = RandomIndexWriter::with_config(random, directory.clone(), iwconfig);

    let mut doc = Document::new();
    let mut field_to_type = HashMap::new();
    doc.add(new_string_field(
      random,
      "id",
      "0",
      Store::No,
      &mut field_to_type,
    )?);
    iwriter.add_document(doc)?;

    let mut doc = Document::new();
    let mut field_to_type = HashMap::new();
    doc.add(new_string_field(
      random,
      "id",
      "1",
      Store::No,
      &mut field_to_type,
    )?);
    doc.add(SortedDocValuesField::new(
      "field",
      new_bytes_ref_from_string(random, "hello")?,
    ));
    iwriter.add_document(doc)?;
    iwriter.commit()?;
    iwriter.delete_documents_with_terms(vec![Term::from_text("id", "1")])?;
    iwriter.force_merge(1)?;

    let ireader = iwriter.get_reader()?;
    iwriter.close()?;

    let mut dv = get_only_leaf_reader(&ireader)?
      .get_sorted_doc_values("field")?
      .unwrap();
    assert_eq!(NO_MORE_DOCS, dv.next_doc()?);

    let mut terms_enum = dv.terms_enum()?;
    let lucene = new_bytes_ref_from_string(random, "lucene")?;
    assert!(!terms_enum.seek_exact(&lucene)?);
    assert_eq!(SeekStatus::End, terms_enum.seek_ceil(&lucene)?);
    assert_eq!(-1, dv.lookup_term(&lucene)?);
    Ok(())
  }

  fn test_bytes_with_newline<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
    let analyzer = MockAnalyzer::new(random);

    let directory = new_directory_shared(random)?;
    let mut conf = new_index_writer_config_with_analyzer(random, analyzer);
    conf.set_merge_policy(new_log_merge_policy(random)?);
    let iwriter = RandomIndexWriter::with_config(random, directory.clone(), conf);

    let mut doc = Document::new();
    doc.add(BinaryDocValuesField::new(
      "dv",
      new_bytes_ref_from_string(random, "hello\nworld\r1")?,
    ));
    iwriter.add_document(doc)?;
    iwriter.close()?;

    let ireader =
      self.maybe_wrap_with_merging_reader(directory_reader_util::open(directory.clone())?)?;
    let top_reader_context = get_context(&ireader)?;
    assert_eq!(1, top_reader_context.leaves()?.len());
    let leaves = top_reader_context.leaves()?;
    let mut dv = leaves[0].reader().get_binary_doc_values("dv")?.unwrap();
    assert_eq!(0, dv.next_doc()?);
    assert_eq!(
      &new_bytes_ref_from_string(random, "hello\nworld\r1")?,
      dv.binary_value()?.as_ref()
    );
    Ok(())
  }

  fn test_missing_sorted_bytes<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
    let analyzer = MockAnalyzer::new(random);

    let directory = new_directory_shared(random)?;
    let mut conf = new_index_writer_config_with_analyzer(random, analyzer);
    conf.set_merge_policy(new_log_merge_policy(random)?);
    let iwriter = RandomIndexWriter::with_config(random, directory.clone(), conf);

    let mut doc = Document::new();
    doc.add(SortedDocValuesField::new(
      "dv",
      new_bytes_ref_from_string(random, "hello world 2")?,
    ));
    iwriter.add_document(doc)?;
    iwriter.add_document(Document::new())?;
    iwriter.close()?;

    let ireader =
      self.maybe_wrap_with_merging_reader(directory_reader_util::open(directory.clone())?)?;
    let top_reader_context = get_context(&ireader)?;
    assert_eq!(1, top_reader_context.leaves()?.len());
    let leaves = top_reader_context.leaves()?;
    let mut dv = leaves[0].reader().get_sorted_doc_values("dv")?.unwrap();
    assert_eq!(0, dv.next_doc()?);
    let ord = dv.ord_value()?;
    assert_eq!(
      &new_bytes_ref_from_string(random, "hello world 2")?,
      dv.lookup_ord(ord)?.as_ref()
    );
    assert_eq!(NO_MORE_DOCS, dv.next_doc()?);
    Ok(())
  }

  fn test_sorted_terms_enum<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
    let directory = new_directory_shared(random)?;
    let analyzer = MockAnalyzer::new(random);
    let mut iwconfig = new_index_writer_config_with_analyzer(random, analyzer);
    iwconfig.set_merge_policy(new_log_merge_policy(random)?);
    let iwriter = RandomIndexWriter::with_config(random, directory.clone(), iwconfig);

    let mut doc = Document::new();
    doc.add(SortedDocValuesField::new(
      "field",
      new_bytes_ref_from_string(random, "hello")?,
    ));
    iwriter.add_document(doc)?;

    let mut doc = Document::new();
    doc.add(SortedDocValuesField::new(
      "field",
      new_bytes_ref_from_string(random, "world")?,
    ));
    iwriter.add_document(doc)?;

    let mut doc = Document::new();
    doc.add(SortedDocValuesField::new(
      "field",
      new_bytes_ref_from_string(random, "beer")?,
    ));
    iwriter.add_document(doc)?;
    iwriter.force_merge(1)?;

    let ireader = iwriter.get_reader()?;
    iwriter.close()?;

    let mut dv = get_only_leaf_reader(&ireader)?
      .get_sorted_doc_values("field")?
      .unwrap();
    assert_eq!(3, dv.get_value_count()?);

    let mut terms_enum = dv.terms_enum()?;

    assert_eq!("beer", terms_enum.next()?.unwrap().utf8_to_string()?);
    assert_eq!(0, terms_enum.ord()?);
    assert_eq!("hello", terms_enum.next()?.unwrap().utf8_to_string()?);
    assert_eq!(1, terms_enum.ord()?);
    assert_eq!("world", terms_enum.next()?.unwrap().utf8_to_string()?);
    assert_eq!(2, terms_enum.ord()?);

    assert_eq!(
      SeekStatus::NotFound,
      terms_enum.seek_ceil(&new_bytes_ref_from_string(random, "ha!")?)?
    );
    assert_eq!("hello", terms_enum.term()?.utf8_to_string()?);
    assert_eq!(1, terms_enum.ord()?);
    assert_eq!(
      SeekStatus::Found,
      terms_enum.seek_ceil(&new_bytes_ref_from_string(random, "beer")?)?
    );
    assert_eq!("beer", terms_enum.term()?.utf8_to_string()?);
    assert_eq!(0, terms_enum.ord()?);
    assert_eq!(
      SeekStatus::End,
      terms_enum.seek_ceil(&new_bytes_ref_from_string(random, "zzz")?)?
    );
    assert_eq!(
      SeekStatus::NotFound,
      terms_enum.seek_ceil(&new_bytes_ref_from_string(random, "aba")?)?
    );
    assert_eq!(0, terms_enum.ord()?);

    assert!(terms_enum.seek_exact(&new_bytes_ref_from_string(random, "beer")?)?);
    assert_eq!("beer", terms_enum.term()?.utf8_to_string()?);
    assert_eq!(0, terms_enum.ord()?);
    assert!(terms_enum.seek_exact(&new_bytes_ref_from_string(random, "hello")?)?);
    assert_eq!("hello", terms_enum.term()?.utf8_to_string()?);
    assert_eq!(1, terms_enum.ord()?);
    assert!(terms_enum.seek_exact(&new_bytes_ref_from_string(random, "world")?)?);
    assert_eq!("world", terms_enum.term()?.utf8_to_string()?);
    assert_eq!(2, terms_enum.ord()?);
    assert!(!terms_enum.seek_exact(&new_bytes_ref_from_string(random, "bogus")?)?);

    terms_enum.seek_exact_with_ord(0)?;
    assert_eq!("beer", terms_enum.term()?.utf8_to_string()?);
    assert_eq!(0, terms_enum.ord()?);
    terms_enum.seek_exact_with_ord(1)?;
    assert_eq!("hello", terms_enum.term()?.utf8_to_string()?);
    assert_eq!(1, terms_enum.ord()?);
    terms_enum.seek_exact_with_ord(2)?;
    assert_eq!("world", terms_enum.term()?.utf8_to_string()?);
    assert_eq!(2, terms_enum.ord()?);

    // TODO IMPORTANT SortedDocValues#intersect 未实现
    Ok(())
  }

  fn test_empty_sorted_bytes<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
    let analyzer = MockAnalyzer::new(random);

    let directory = new_directory_shared(random)?;
    let mut conf = new_index_writer_config_with_analyzer(random, analyzer);
    conf.set_merge_policy(new_log_merge_policy(random)?);
    let iwriter = RandomIndexWriter::with_config(random, directory.clone(), conf);

    let mut doc = Document::new();
    doc.add(SortedDocValuesField::new(
      "dv",
      new_bytes_ref_from_string(random, "")?,
    ));
    iwriter.add_document(doc)?;

    let mut doc = Document::new();
    doc.add(SortedDocValuesField::new(
      "dv",
      new_bytes_ref_from_string(random, "")?,
    ));
    iwriter.add_document(doc)?;
    iwriter.force_merge(1)?;
    iwriter.close()?;

    let ireader =
      self.maybe_wrap_with_merging_reader(directory_reader_util::open(directory.clone())?)?;
    let top_reader_context = get_context(&ireader)?;
    assert_eq!(1, top_reader_context.leaves()?.len());
    let leaves = top_reader_context.leaves()?;
    let mut dv = leaves[0].reader().get_sorted_doc_values("dv")?.unwrap();
    assert_eq!(0, dv.next_doc()?);
    assert_eq!(0, dv.ord_value()?);
    assert_eq!(1, dv.next_doc()?);
    assert_eq!(0, dv.ord_value()?);
    assert_eq!("", dv.lookup_ord(0)?.utf8_to_string()?);
    Ok(())
  }

  fn test_empty_bytes<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
    let analyzer = MockAnalyzer::new(random);

    let directory = new_directory_shared(random)?;
    let mut conf = new_index_writer_config_with_analyzer(random, analyzer);
    conf.set_merge_policy(new_log_merge_policy(random)?);
    let iwriter = RandomIndexWriter::with_config(random, directory.clone(), conf);

    let mut doc = Document::new();
    doc.add(BinaryDocValuesField::new(
      "dv",
      new_bytes_ref_from_string(random, "")?,
    ));
    iwriter.add_document(doc)?;

    let mut doc = Document::new();
    doc.add(BinaryDocValuesField::new(
      "dv",
      new_bytes_ref_from_string(random, "")?,
    ));
    iwriter.add_document(doc)?;
    iwriter.force_merge(1)?;
    iwriter.close()?;

    let ireader =
      self.maybe_wrap_with_merging_reader(directory_reader_util::open(directory.clone())?)?;
    let top_reader_context = get_context(&ireader)?;
    assert_eq!(1, top_reader_context.leaves()?.len());
    let leaves = top_reader_context.leaves()?;
    let mut dv = leaves[0].reader().get_binary_doc_values("dv")?.unwrap();
    assert_eq!(0, dv.next_doc()?);
    assert_eq!("", dv.binary_value()?.utf8_to_string()?);
    assert_eq!(1, dv.next_doc()?);
    assert_eq!("", dv.binary_value()?.utf8_to_string()?);
    Ok(())
  }

  fn test_very_large_but_legal_bytes<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
    let analyzer = MockAnalyzer::new(random);

    let directory = new_directory_shared(random)?;
    let mut conf = new_index_writer_config_with_analyzer(random, analyzer);
    conf.set_merge_policy(new_log_merge_policy(random)?);
    let iwriter = RandomIndexWriter::with_config(random, directory.clone(), conf);

    let mut doc = Document::new();
    let mut bytes = vec![0u8; 32766];
    random.fill_bytes(&mut bytes);
    let b = new_bytes_ref_from_bytes(random, bytes.as_ref())?;
    doc.add(BinaryDocValuesField::new("dv", b.clone()));
    iwriter.add_document(doc)?;
    iwriter.close()?;

    let ireader =
      self.maybe_wrap_with_merging_reader(directory_reader_util::open(directory.clone())?)?;
    let top_reader_context = get_context(&ireader)?;
    assert_eq!(1, top_reader_context.leaves()?.len());
    let leaves = top_reader_context.leaves()?;
    let mut dv = leaves[0].reader().get_binary_doc_values("dv")?.unwrap();
    assert_eq!(0, dv.next_doc()?);
    assert_eq!(&b, dv.binary_value()?.as_ref());
    Ok(())
  }

  fn test_very_large_but_legal_sorted_bytes<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
    let analyzer = MockAnalyzer::new(random);

    let directory = new_directory_shared(random)?;
    let mut conf = new_index_writer_config_with_analyzer(random, analyzer);
    conf.set_merge_policy(new_log_merge_policy(random)?);
    let iwriter = RandomIndexWriter::with_config(random, directory.clone(), conf);

    let mut doc = Document::new();
    let mut bytes = vec![0u8; 32766];
    random.fill_bytes(&mut bytes);
    let b = new_bytes_ref_from_bytes(random, bytes.as_ref())?;
    doc.add(SortedDocValuesField::new("dv", b.clone()));
    iwriter.add_document(doc)?;
    iwriter.close()?;

    let ireader =
      self.maybe_wrap_with_merging_reader(directory_reader_util::open(directory.clone())?)?;
    let top_reader_context = get_context(&ireader)?;
    assert_eq!(1, top_reader_context.leaves()?.len());
    let leaves = top_reader_context.leaves()?;
    let mut dv = leaves[0].reader().get_sorted_doc_values("dv")?.unwrap();
    assert_eq!(0, dv.next_doc()?);
    let ord = dv.ord_value()?;
    assert_eq!(&b, dv.lookup_ord(ord)?.as_ref());
    Ok(())
  }

  fn test_codec_uses_own_bytes<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
    let analyzer = MockAnalyzer::new(random);

    let directory = new_directory_shared(random)?;
    let mut conf = new_index_writer_config_with_analyzer(random, analyzer);
    conf.set_merge_policy(new_log_merge_policy(random)?);
    let iwriter = RandomIndexWriter::with_config(random, directory.clone(), conf);

    let mut doc = Document::new();
    doc.add(BinaryDocValuesField::new(
      "dv",
      new_bytes_ref_from_string(random, "boo!")?,
    ));
    iwriter.add_document(doc)?;
    iwriter.close()?;

    let ireader =
      self.maybe_wrap_with_merging_reader(directory_reader_util::open(directory.clone())?)?;
    let top_reader_context = get_context(&ireader)?;
    assert_eq!(1, top_reader_context.leaves()?.len());
    let leaves = top_reader_context.leaves()?;
    let mut dv = leaves[0].reader().get_binary_doc_values("dv")?.unwrap();
    assert_eq!(0, dv.next_doc()?);
    assert_eq!("boo!", dv.binary_value()?.utf8_to_string()?);
    Ok(())
  }

  fn test_codec_uses_own_sorted_bytes<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
    let analyzer = MockAnalyzer::new(random);

    let directory = new_directory_shared(random)?;
    let mut conf = new_index_writer_config_with_analyzer(random, analyzer);
    conf.set_merge_policy(new_log_merge_policy(random)?);
    let iwriter = RandomIndexWriter::with_config(random, directory.clone(), conf);

    let mut doc = Document::new();
    doc.add(SortedDocValuesField::new(
      "dv",
      new_bytes_ref_from_string(random, "boo!")?,
    ));
    iwriter.add_document(doc)?;
    iwriter.close()?;

    let ireader =
      self.maybe_wrap_with_merging_reader(directory_reader_util::open(directory.clone())?)?;
    let top_reader_context = get_context(&ireader)?;
    assert_eq!(1, top_reader_context.leaves()?.len());
    let leaves = top_reader_context.leaves()?;
    let mut dv = leaves[0].reader().get_sorted_doc_values("dv")?.unwrap();
    assert_eq!(0, dv.next_doc()?);
    let ord = dv.ord_value()?;
    assert_eq!("boo!", dv.lookup_ord(ord)?.utf8_to_string()?);
    Ok(())
  }

  fn test_doc_values_simple<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
    let dir = new_directory_shared(random)?;
    let analyzer = MockAnalyzer::new(random);
    let mut conf = new_index_writer_config_with_analyzer(random, analyzer);
    conf.set_merge_policy(new_log_merge_policy(random)?);
    let writer = IndexWriter::new(dir.clone(), conf)?;

    for i in 0..5 {
      let mut doc = Document::new();
      let mut field_to_type = HashMap::new();
      doc.add(NumericDocValuesField::new("docId", i as i64));
      doc.add(new_text_field(
        random,
        "docId",
        i.to_string(),
        Store::No,
        &mut field_to_type,
      )?);
      writer.add_document(doc)?;
    }
    writer.commit()?;
    writer.force_merge_with_wait(1, true)?;
    writer.close()?;

    let reader = self.maybe_wrap_with_merging_reader(directory_reader_util::open(dir.clone())?)?;
    let top_reader_context = get_context(&reader)?;
    assert_eq!(1, top_reader_context.leaves()?.len());

    let searcher = new_searcher_with_reader(reader)?;
    let mut query = BooleanQueryBuilder::new();
    query.add(TermQuery::new(Term::from_text("docId", "0")), Occur::Should)?;
    query.add(TermQuery::new(Term::from_text("docId", "1")), Occur::Should)?;
    query.add(TermQuery::new(Term::from_text("docId", "2")), Occur::Should)?;
    query.add(TermQuery::new(Term::from_text("docId", "3")), Occur::Should)?;
    query.add(TermQuery::new(Term::from_text("docId", "4")), Occur::Should)?;

    let search = searcher.search(query.build(), 10)?;
    assert_eq!(5, search.total_hits.value());
    let mut doc_values = get_only_leaf_reader(searcher.reader_context.reader())?
      .get_numeric_doc_values("docId")?
      .unwrap();
    for (i, score_doc) in search.score_docs.iter().enumerate() {
      assert_eq!(i as i32, score_doc.doc);
      assert_eq!(i as i32, doc_values.advance(i as i32)?);
      assert_eq!(i as i64, doc_values.long_value()?);
    }
    Ok(())
  }
}
