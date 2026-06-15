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
use crate::core::analysis::analyzer::Analyzer;
use crate::core::analysis::reader::ReaderEnum;
use crate::core::analysis::token_stream::TokenStream;
use crate::core::document::binary_doc_values_field::BinaryDocValuesField;
use crate::core::document::document::Document;
use crate::core::document::field::{Field, FieldDataEnum, Store};
use crate::core::document::field_type::FieldType;
use crate::core::document::fields::FieldTokenStreamEnum;
use crate::core::document::int_point::IntPoint;
use crate::core::document::invertable_field::InvertableType;
use crate::core::document::knn_float_vector_field::KnnFloatVectorField;
use crate::core::document::numeric_doc_values_field::NumericDocValuesField;
use crate::core::document::sorted_doc_values_field::SortedDocValuesField;
use crate::core::document::sorted_numeric_doc_values_field::SortedNumericDocValuesField;
use crate::core::document::sorted_set_doc_values_field::SortedSetDocValuesField;
use crate::core::document::stored_field::StoredField;
use crate::core::document::string_field::StringField;
use crate::core::document::text_field::TextField;
use crate::core::index::BytesRef;
use crate::core::index::directory_reader;
use crate::core::index::fields::Fields as IndexFields;
use crate::core::index::index_options::IndexOptions;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::index_writer_config::{DISABLE_AUTO_FLUSH, OpenMode};
use crate::core::index::indexable_field::IndexableField;
use crate::core::index::indexable_field::{IndexingTokenStream, ReusedIndexingTokenStream};
use crate::core::index::indexable_field_type::IndexableFieldType;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::postings_enum::{ALL, PostingsEnum};
use crate::core::index::segment_reader::SegmentReader;
use crate::core::index::stored_fields::StoredFields;
use crate::core::index::term::Term;
use crate::core::index::term_vectors::TermVectors;
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::TermsEnum;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::index::vector_similarity_function::VectorSimilarityFunction;
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, NO_MORE_DOCS};
use crate::core::store::io_context::IOContext;
use crate::core::util::LATEST;
use crate::core::util::attribute_source::{AttributeSource, Attributes};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::number::Number;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::index::doc_helper::{
  DocHelper, FIELD_1_TEXT, FIELD_2_TEXT, FIELD_3_TEXT, KEYWORD_FIELD_KEY, KEYWORD_TEXT,
  NO_NORMS_KEY, NO_NORMS_TEXT, TEXT_FIELD_1_KEY, TEXT_FIELD_2_KEY, TEXT_FIELD_3_KEY,
};
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  get_only_leaf_reader, new_directory_shared, new_index_writer_config,
  new_index_writer_config_with_analyzer, random,
};
use crate::test::core::util::test_util::TestUtil;
use std::borrow::Cow;
use std::fmt::{Display, Formatter};

#[allow(dead_code)] // for quick
struct TestDocumentWriter;

#[test]
fn test_add_document() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut test_doc = Document::new();
  DocHelper::setup_doc(&mut test_doc);
  let info = DocHelper::write_doc(&mut random, dir, test_doc)?;

  let reader = SegmentReader::new(&info, LATEST.major, &IOContext::default_io_context()?)?;
  let mut stored_fields = reader.stored_fields()?;
  let doc = stored_fields.document(0)?;

  let fields = doc.get_fields_with_name(TEXT_FIELD_2_KEY);
  assert_eq!(1, fields.len());
  assert_eq!(FIELD_2_TEXT, fields[0].string_value()?.unwrap().as_ref());
  assert!(fields[0].field_type().store_term_vectors());

  let fields = doc.get_fields_with_name(TEXT_FIELD_1_KEY);
  assert_eq!(1, fields.len());
  assert_eq!(FIELD_1_TEXT, fields[0].string_value()?.unwrap().as_ref());
  assert!(!fields[0].field_type().store_term_vectors());

  let fields = doc.get_fields_with_name(KEYWORD_FIELD_KEY);
  assert_eq!(1, fields.len());
  assert_eq!(KEYWORD_TEXT, fields[0].string_value()?.unwrap().as_ref());

  let fields = doc.get_fields_with_name(NO_NORMS_KEY);
  assert_eq!(1, fields.len());
  assert_eq!(NO_NORMS_TEXT, fields[0].string_value()?.unwrap().as_ref());

  let fields = doc.get_fields_with_name(TEXT_FIELD_3_KEY);
  assert_eq!(1, fields.len());
  assert_eq!(FIELD_3_TEXT, fields[0].string_value()?.unwrap().as_ref());

  for fi in reader.get_field_infos()?.iter() {
    if *fi.get_index_options() != IndexOptions::None {
      assert_eq!(
        fi.omits_norms(),
        reader.get_norm_values(&fi.name)?.is_none()
      );
    }
  }

  Ok(())
}

#[test]
fn test_position_increment_gap() -> Result<()> {
  // TODO IMPORTANT 自定义分词器有问题
  Ok(())
}

#[test]
fn test_token_reuse() -> Result<()> {
  // TODO IMPORTANT 自定义分词器有问题
  Ok(())
}

#[test]
fn test_pre_analyzed_field() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let analyzer = MockAnalyzer::new(&mut random);
  let writer = IndexWriter::new(
    dir.clone(),
    new_index_writer_config_with_analyzer(&mut random, analyzer),
  )?;
  let mut doc = Document::new();

  doc.add(TextField::from_token_stream(
    "preanalyzed",
    FieldTokenStreamEnum::custom(PreAnalyzedTokenStream::new()),
  )?);

  writer.add_document(doc)?;
  writer.commit()?;
  writer.close()?;
  let reader = directory_reader::open(dir)?;
  let leaf = get_only_leaf_reader(&reader)?;

  let mut term_positions = leaf
    .postings_with_flag(&Term::from_text("preanalyzed", "term1"), ALL as i32)?
    .expect("term1 postings should exist");
  assert_ne!(NO_MORE_DOCS, term_positions.next_doc()?);
  assert_eq!(1, term_positions.freq()?);
  assert_eq!(0, term_positions.next_position()?);

  let mut term_positions = leaf
    .postings_with_flag(&Term::from_text("preanalyzed", "term2"), ALL as i32)?
    .expect("term2 postings should exist");
  assert_ne!(NO_MORE_DOCS, term_positions.next_doc()?);
  assert_eq!(2, term_positions.freq()?);
  assert_eq!(1, term_positions.next_position()?);
  assert_eq!(3, term_positions.next_position()?);

  let mut term_positions = leaf
    .postings_with_flag(&Term::from_text("preanalyzed", "term3"), ALL as i32)?
    .expect("term3 postings should exist");
  assert_ne!(NO_MORE_DOCS, term_positions.next_doc()?);
  assert_eq!(1, term_positions.freq()?);
  assert_eq!(2, term_positions.next_position()?);

  Ok(())
}

#[test]
fn test_lucene_1590() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut doc = Document::new();

  let mut custom_type = FieldType::from_ref(&*crate::core::document::text_field::TYPE_NOT_STORED)?;
  custom_type.set_omit_norms(true)?;
  let mut custom_type2 = FieldType::new();
  custom_type2.set_stored(true)?;
  doc.add(Field::new("f1", "v1", custom_type));
  doc.add(Field::new("f1", "v2", custom_type2.clone()));

  let mut custom_type3 = FieldType::from_ref(&*crate::core::document::text_field::TYPE_NOT_STORED)?;
  custom_type3.set_index_options(IndexOptions::Docs)?;
  let f = Field::new("f2", "v1", custom_type3);
  doc.add(f);
  doc.add(Field::new("f2", "v2", custom_type2));

  let analyzer = MockAnalyzer::new(&mut random);
  let writer = IndexWriter::new(
    dir.clone(),
    new_index_writer_config_with_analyzer(&mut random, analyzer),
  )?;
  writer.add_document(doc)?;
  writer.force_merge(1)?;
  writer.close()?;

  TestUtil::check_index(dir.clone())?;

  let reader = directory_reader::open(dir)?;
  let leaf = get_only_leaf_reader(&reader)?;
  let fi = leaf.get_field_infos()?;
  let f1 = fi
    .field_info_by_name("f1")
    .ok_or_else(|| LuceneError::illegal_state("f1 should exist"))?;
  assert!(!f1.has_norms(), "f1 should have no norms");
  assert_eq!(
    &IndexOptions::DocsAndFreqsAndPositions,
    f1.get_index_options(),
    "omitTermFreqAndPositions field bit should not be set for f1"
  );
  let f2 = fi
    .field_info_by_name("f2")
    .ok_or_else(|| LuceneError::illegal_state("f2 should exist"))?;
  assert!(f2.has_norms(), "f2 should have norms");
  assert_eq!(
    &IndexOptions::Docs,
    f2.get_index_options(),
    "omitTermFreqAndPositions field bit should be set for f2"
  );

  Ok(())
}

fn do_test_ram_usage<F>(field_supplier: F) -> Result<()>
where
  F: Fn(&str) -> Result<crate::core::document::fields::Fields>,
{
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut iwc = new_index_writer_config(&mut random);
  iwc.set_max_buffered_docs(10);
  iwc.set_ram_buffer_size_mb(DISABLE_AUTO_FLUSH as f64);
  let writer = IndexWriter::new(dir, iwc)?;

  let mut doc = Document::new();
  let num_fields = 100;
  for i in 0..num_fields {
    doc.add(field_supplier(&format!("f{}", i))?);
  }
  writer.add_document(doc)?;
  assert!(writer.has_changes_in_ram());
  // TODO: memory calculation not implement
  // assert!(writer.doc_writer.ram_bytes_used()? < num_fields * 16384);
  writer.close()?;
  Ok(())
}

#[test]
fn test_ram_usage_stored() -> Result<()> {
  do_test_ram_usage(|field| {
    Ok(StoredField::from_binary(field, BytesRef::from_string("Lucene").bytes)?.into())
  })
}

#[test]
fn test_ram_usage_indexed() -> Result<()> {
  do_test_ram_usage(|field| {
    Ok(StringField::from_bytes_ref(field, BytesRef::from_string("Lucene"), Store::No)?.into())
  })
}

#[test]
fn test_ram_usage_point() -> Result<()> {
  do_test_ram_usage(|field| Ok(IntPoint::new(field, [42])?.into()))
}

#[test]
fn test_ram_usage_numeric_doc_value() -> Result<()> {
  do_test_ram_usage(|field| Ok(NumericDocValuesField::new(field, 42).into()))
}

#[test]
fn test_ram_usage_sorted_doc_value() -> Result<()> {
  do_test_ram_usage(|field| {
    Ok(SortedDocValuesField::new(field, BytesRef::from_string("Lucene")).into())
  })
}

#[test]
fn test_ram_usage_binary_doc_value() -> Result<()> {
  do_test_ram_usage(|field| {
    Ok(BinaryDocValuesField::new(field, BytesRef::from_string("Lucene")).into())
  })
}

#[test]
fn test_ram_usage_sorted_numeric_doc_value() -> Result<()> {
  do_test_ram_usage(|field| Ok(SortedNumericDocValuesField::new(field, 42).into()))
}

#[test]
fn test_ram_usage_sorted_set_doc_value() -> Result<()> {
  do_test_ram_usage(|field| {
    Ok(SortedSetDocValuesField::new(field, BytesRef::from_string("Lucene")).into())
  })
}

#[test]
fn test_ram_usage_vector() -> Result<()> {
  do_test_ram_usage(|field| {
    Ok(
      KnnFloatVectorField::with_similarity_function(
        field,
        vec![1.0, 2.0, 3.0, 4.0],
        VectorSimilarityFunction::Euclidean,
      )?
      .into(),
    )
  })
}

#[test]
fn test_index_binary_value_without_token_stream() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let mut illegal_field_types = Vec::new();
  {
    let mut illegal_ft = FieldType::new();
    illegal_ft.set_tokenized(true)?;
    illegal_ft.set_index_options(IndexOptions::Docs)?;
    illegal_ft.freeze();
    illegal_field_types.push(illegal_ft);
  }
  {
    let mut illegal_ft = FieldType::new();
    illegal_ft.set_tokenized(false)?;
    illegal_ft.set_index_options(IndexOptions::DocsAndFreqsAndPositions)?;
    illegal_ft.freeze();
    illegal_field_types.push(illegal_ft);
  }
  {
    let mut illegal_ft = FieldType::new();
    illegal_ft.set_tokenized(false)?;
    illegal_ft.set_index_options(IndexOptions::Docs)?;
    illegal_ft.set_store_term_vectors(true)?;
    illegal_ft.set_store_term_vector_positions(true)?;
    illegal_ft.freeze();
    illegal_field_types.push(illegal_ft);
  }
  {
    let mut illegal_ft = FieldType::new();
    illegal_ft.set_tokenized(false)?;
    illegal_ft.set_index_options(IndexOptions::Docs)?;
    illegal_ft.set_store_term_vectors(true)?;
    illegal_ft.set_store_term_vector_offsets(true)?;
    illegal_ft.freeze();
    illegal_field_types.push(illegal_ft);
  }

  for ft in illegal_field_types {
    let mut iwc = new_index_writer_config(&mut random);
    iwc.set_open_mode(OpenMode::Create);
    let writer = IndexWriter::new(dir.clone(), iwc)?;
    let field = MockIndexableField::new("field", Some(BytesRef::from_string("a")), ft);
    let mut doc = Document::new();
    doc.add(field);
    let res = writer.add_document(doc);
    assert!(
      matches!(res, Err(LuceneError::IllegalArgument(_))),
      "expected IllegalArgument but got: {:?}",
      res
    );
    writer.close()?;
  }

  {
    let mut iwc = new_index_writer_config(&mut random);
    iwc.set_open_mode(OpenMode::Create);
    let writer = IndexWriter::new(dir.clone(), iwc)?;
    let field = MockIndexableField::new(
      "field",
      None,
      crate::core::document::string_field::TYPE_NOT_STORED.clone(),
    );
    let mut doc = Document::new();
    doc.add(field);
    let res = writer.add_document(doc);
    assert!(
      matches!(res, Err(LuceneError::IllegalArgument(_))),
      "expected IllegalArgument but got: {:?}",
      res
    );
    writer.close()?;
  }

  let mut legal_field_types = Vec::new();
  {
    let mut ft = FieldType::new();
    ft.set_tokenized(false)?;
    ft.set_index_options(IndexOptions::Docs)?;
    ft.set_omit_norms(false)?;
    ft.freeze();
    legal_field_types.push(ft);
  }
  {
    let mut ft = FieldType::new();
    ft.set_tokenized(false)?;
    ft.set_index_options(IndexOptions::DocsAndFreqs)?;
    ft.set_omit_norms(false)?;
    ft.freeze();
    legal_field_types.push(ft);
  }
  {
    let mut ft = FieldType::new();
    ft.set_tokenized(false)?;
    ft.set_index_options(IndexOptions::Docs)?;
    ft.set_omit_norms(true)?;
    ft.freeze();
    legal_field_types.push(ft);
  }
  {
    let mut ft = FieldType::new();
    ft.set_tokenized(false)?;
    ft.set_index_options(IndexOptions::DocsAndFreqs)?;
    ft.set_omit_norms(true)?;
    ft.freeze();
    legal_field_types.push(ft);
  }
  {
    let mut ft = FieldType::new();
    ft.set_tokenized(false)?;
    ft.set_index_options(IndexOptions::Docs)?;
    ft.set_store_term_vectors(true)?;
    ft.freeze();
    legal_field_types.push(ft);
  }
  {
    let mut ft = FieldType::new();
    ft.set_tokenized(false)?;
    ft.set_index_options(IndexOptions::DocsAndFreqs)?;
    ft.set_store_term_vectors(true)?;
    ft.freeze();
    legal_field_types.push(ft);
  }

  for ft in legal_field_types {
    {
      let mut iwc = new_index_writer_config(&mut random);
      iwc.set_open_mode(OpenMode::Create);
      let writer = IndexWriter::new(dir.clone(), iwc)?;
      let field = MockIndexableField::new("field", Some(BytesRef::from_string("a")), ft.clone());
      let mut doc = Document::new();
      doc.add(field.clone());
      doc.add(field);
      writer.add_document(doc)?;
      writer.close()?;
    }

    let reader = directory_reader::open(dir.clone())?;
    let leaf_reader = get_only_leaf_reader(&reader)?;

    {
      let terms = leaf_reader
        .terms("field")?
        .ok_or_else(|| LuceneError::illegal_state("field terms should exist"))?;
      assert_eq!(1, terms.get_sum_doc_freq()?);
      if *ft.index_options() >= IndexOptions::DocsAndFreqs {
        assert_eq!(2, terms.get_sum_total_term_freq()?);
      } else {
        assert_eq!(1, terms.get_sum_total_term_freq()?);
      }
      let mut terms_enum = terms.iterator()?;
      assert!(terms_enum.seek_exact(&BytesRef::from_string("a"))?);
      let mut pe = terms_enum.postings_with_flags(None, ALL as i32)?;
      assert_eq!(0, pe.next_doc()?);
      if *ft.index_options() >= IndexOptions::DocsAndFreqs {
        assert_eq!(2, pe.freq()?);
      } else {
        assert_eq!(1, pe.freq()?);
      }
      assert_eq!(-1, pe.next_position()?);
      assert_eq!(NO_MORE_DOCS, pe.next_doc()?);
    }

    let mut term_vectors = leaf_reader.term_vectors()?;
    if ft.store_term_vectors() {
      let tv_fields = term_vectors
        .get(0)?
        .ok_or_else(|| LuceneError::illegal_state("term vectors should exist"))?;
      let tv_terms = tv_fields
        .terms("field")?
        .ok_or_else(|| LuceneError::illegal_state("field term vectors should exist"))?;
      assert_eq!(1, tv_terms.get_sum_doc_freq()?);
      assert_eq!(2, tv_terms.get_sum_total_term_freq()?);
      let mut tv_terms_enum = tv_terms.iterator()?;
      assert!(tv_terms_enum.seek_exact(&BytesRef::from_string("a"))?);
      let mut pe = tv_terms_enum.postings_with_flags(None, ALL as i32)?;
      assert_eq!(0, pe.next_doc()?);
      assert_eq!(2, pe.freq()?);
      assert_eq!(-1, pe.next_position()?);
      assert_eq!(NO_MORE_DOCS, pe.next_doc()?);
    } else {
      assert!(term_vectors.get(0)?.is_none());
    }
  }

  Ok(())
}

struct PreAnalyzedTokenStream {
  attrs: Attributes,
  tokens: [&'static str; 4],
  index: usize,
}

impl PreAnalyzedTokenStream {
  fn new() -> Self {
    Self {
      attrs: Attributes::default(),
      tokens: ["term1", "term2", "term3", "term2"],
      index: 0,
    }
  }
}

impl TokenStream for PreAnalyzedTokenStream {
  fn increment_token(&mut self) -> Result<bool> {
    if self.index == self.tokens.len() {
      return Ok(false);
    }

    self.attrs.clear_attributes();
    self.attrs.append_str(Some(self.tokens[self.index]))?;
    self.index += 1;
    Ok(true)
  }

  fn end(&mut self) -> Result<()> {
    self.default_end()
  }

  fn get_attribute_source(&self) -> &Attributes {
    &self.attrs
  }

  fn get_attribute_source_mut(&mut self) -> &mut Attributes {
    &mut self.attrs
  }
}

#[derive(Clone)]
pub struct MockIndexableField {
  field: String,
  value: Option<BytesRef<Vec<u8>>>,
  field_type: FieldType,
}

impl MockIndexableField {
  fn new(field: &str, value: Option<BytesRef<Vec<u8>>>, field_type: FieldType) -> Self {
    Self {
      field: field.to_string(),
      value,
      field_type,
    }
  }
}

impl Display for MockIndexableField {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "MockIndexableField<{}>", self.field)
  }
}

impl IndexableField for MockIndexableField {
  fn name(&self) -> &str {
    &self.field
  }

  type FieldType = FieldType;

  fn field_type(&self) -> &Self::FieldType {
    &self.field_type
  }

  fn token_stream<'a, A>(
    &'a mut self,
    _analyzer: &'a A,
    _reuse_token_stream: &'a mut Option<ReusedIndexingTokenStream>,
  ) -> Result<IndexingTokenStream<'a>>
  where
    A: Analyzer,
  {
    Ok(None)
  }

  fn binary_value(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    Ok(self.value.as_ref().map(Cow::Borrowed))
  }

  fn take_binary_value(&mut self) -> Result<Option<BytesRef<Vec<u8>>>> {
    Ok(self.value.take())
  }

  fn string_value(&self) -> Result<Option<Cow<'_, String>>> {
    Ok(None)
  }

  fn take_string_value(&mut self) -> Result<Option<String>> {
    Ok(None)
  }

  fn take_reader_value(&mut self) -> Result<Option<ReaderEnum>> {
    Ok(None)
  }

  fn numeric_value(&self) -> Result<Option<Number>> {
    Ok(None)
  }

  fn stored_value(&self) -> Option<&FieldDataEnum> {
    None
  }

  fn invertable_type(&self) -> &InvertableType {
    &InvertableType::BINARY
  }

  fn init_token_stream<A>(&mut self, _analyzer: &A) -> Result<()>
  where
    A: Analyzer,
  {
    Ok(())
  }
}
