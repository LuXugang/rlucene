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
use crate::core::codecs::knn_field_vectors_writer::VectorValueEnum;
use crate::core::codecs::knn_vectors_format::KnnVectorsFormat;
use crate::core::codecs::{Codec, LATEST_CODEC};
use crate::core::document::document::Document;
use crate::core::document::field::FieldBase;
use crate::core::document::field::Store;
use crate::core::document::knn_byte_vector_field::KnnByteVectorField;
use crate::core::document::knn_float_vector_field::KnnFloatVectorField;
use crate::core::document::string_field::StringField;
use crate::core::index::directory_reader::directory_reader_util;
use crate::core::index::index_writer::{IndexWriter, IndexWriterBase};
use crate::core::index::indexable_field::IndexableField;
use crate::core::index::indexable_field_type::IndexableFieldType;
use crate::core::index::knn_vector_values::KnnVectorValues;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::merge_policy::MergePolicyEnum;
use crate::core::index::merge_scheduler::MergeSchedulerEnum;
use crate::core::index::merge_scheduler::{MergeScheduler, MergeSource};
use crate::core::index::merge_trigger::MergeTrigger;
use crate::core::index::term::Term;
use crate::core::index::vector_encoding::VectorEncoding;
use crate::core::index::vector_similarity_function::VectorSimilarityFunction;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::search::knn_collector::KnnCollector;
use crate::core::search::top_knn_collector::TopKnnCollector;
use crate::core::store::directory::Directory;
use crate::core::util::close::Closeable;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::core::index::base_index_file_format_test_case::BaseIndexFileFormatTestCase;
use crate::test::core::index::force_merge_policy::ForceMergePolicy;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  get_only_leaf_reader, new_directory_shared, new_index_writer_config,
};
use rand::Rng;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub trait BaseKnnVectorsFormatTestCase: BaseIndexFileFormatTestCase {
  fn get_vectors_max_dimensions(&self, field_name: &str) -> usize {
    LATEST_CODEC
      .knn_vectors_format()
      .unwrap()
      .get_max_dimensions(field_name)
  }

  fn test_field_constructor<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let v = vec![0.0_f32; 1];
    let field = KnnFloatVectorField::new("f", v.clone())?;
    assert_eq!(1, field.field_type().vector_dimension());
    assert_eq!(
      &VectorSimilarityFunction::Euclidean,
      field.field_type().vector_similarity_function()
    );
    match field.vector_value()? {
      VectorValueEnum::Float(actual) => assert_eq!(v.as_slice(), actual.as_slice()),
      _ => unreachable!(""),
    }
    Ok(())
  }

  fn test_field_constructor_exceptions<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let res = KnnFloatVectorField::new("f", vec![]);
    assert!(matches!(res, Err(LuceneError::IllegalArgument(_))));
    Ok(())
  }

  fn test_field_set_value<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut field = KnnFloatVectorField::new("f", vec![0.0])?;
    let v1 = vec![1.0_f32];
    field.set_vector_value(v1.clone())?;
    match field.vector_value()? {
      VectorValueEnum::Float(actual) => assert_eq!(v1.as_slice(), actual.as_slice()),
      _ => unreachable!(""),
    }

    let err = field.set_vector_value(vec![1.0, 2.0]).unwrap_err();
    assert_eq!(
      "value length 2 must match field dimension 1",
      err.to_string()
    );
    Ok(())
  }

  fn test_illegal_dim_change_two_docs<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    {
      let dir = new_directory_shared(random)?;
      let w = IndexWriter::new(dir, new_index_writer_config(random))?;

      let mut doc = Document::new();
      doc.add(KnnFloatVectorField::with_similarity_function(
        "f",
        vec![0.0; 4],
        VectorSimilarityFunction::DotProduct,
      )?);
      w.add_document(doc)?;

      let mut doc2 = Document::new();
      doc2.add(KnnFloatVectorField::with_similarity_function(
        "f",
        vec![0.0; 6],
        VectorSimilarityFunction::DotProduct,
      )?);
      let err = w.add_document(doc2).unwrap_err();
      assert_eq!(
        "Inconsistency of field data structures across documents for field [f] of doc [1].vector dimension: expected '4', but it has '6'.",
        err.to_string()
      );
    }

    {
      let dir = new_directory_shared(random)?;
      let w = IndexWriter::new(dir, new_index_writer_config(random))?;

      let mut doc = Document::new();
      doc.add(KnnFloatVectorField::with_similarity_function(
        "f",
        vec![0.0; 4],
        VectorSimilarityFunction::DotProduct,
      )?);
      w.add_document(doc)?;
      w.commit()?;

      let mut doc2 = Document::new();
      doc2.add(KnnFloatVectorField::with_similarity_function(
        "f",
        vec![0.0; 6],
        VectorSimilarityFunction::DotProduct,
      )?);
      let err = w.add_document(doc2).unwrap_err();
      assert_eq!(
        format!(
          "cannot change field \"f\" from vector dimension=4, vector encoding={:?}, vector similarity function={:?} to inconsistent vector dimension=6, vector encoding={:?}, vector similarity function={:?}",
          VectorEncoding::FLOAT32(4),
          VectorSimilarityFunction::DotProduct,
          VectorEncoding::FLOAT32(4),
          VectorSimilarityFunction::DotProduct
        ),
        err.to_string()
      );
    }

    Ok(())
  }

  fn test_illegal_similarity_function_change<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    {
      let dir = new_directory_shared(random)?;
      let w = IndexWriter::new(dir, new_index_writer_config(random))?;

      let mut doc = Document::new();
      doc.add(KnnFloatVectorField::with_similarity_function(
        "f",
        vec![0.0; 4],
        VectorSimilarityFunction::DotProduct,
      )?);
      w.add_document(doc)?;

      let mut doc2 = Document::new();
      doc2.add(KnnFloatVectorField::with_similarity_function(
        "f",
        vec![0.0; 4],
        VectorSimilarityFunction::Euclidean,
      )?);
      let err = w.add_document(doc2).unwrap_err();
      assert_eq!(
        format!(
          "Inconsistency of field data structures across documents for field [f] of doc [1].vector similarity function: expected '{}', but it has '{}'.",
          VectorSimilarityFunction::DotProduct,
          VectorSimilarityFunction::Euclidean
        ),
        err.to_string()
      );
    }

    {
      let dir = new_directory_shared(random)?;
      let w = IndexWriter::new(dir, new_index_writer_config(random))?;

      let mut doc = Document::new();
      doc.add(KnnFloatVectorField::with_similarity_function(
        "f",
        vec![0.0; 4],
        VectorSimilarityFunction::DotProduct,
      )?);
      w.add_document(doc)?;
      w.commit()?;

      let mut doc2 = Document::new();
      doc2.add(KnnFloatVectorField::with_similarity_function(
        "f",
        vec![0.0; 4],
        VectorSimilarityFunction::Euclidean,
      )?);
      let err = w.add_document(doc2).unwrap_err();
      assert_eq!(
        format!(
          "cannot change field \"f\" from vector dimension=4, vector encoding={:?}, vector similarity function={:?} to inconsistent vector dimension=4, vector encoding={:?}, vector similarity function={:?}",
          VectorEncoding::FLOAT32(4),
          VectorSimilarityFunction::DotProduct,
          VectorEncoding::FLOAT32(4),
          VectorSimilarityFunction::Euclidean
        ),
        err.to_string()
      );
    }

    Ok(())
  }

  fn test_illegal_dim_change_two_writers<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(random)?;

    {
      let w = IndexWriter::new(dir.clone(), new_index_writer_config(random))?;
      let mut doc = Document::new();
      doc.add(KnnFloatVectorField::with_similarity_function(
        "f",
        vec![0.0; 4],
        VectorSimilarityFunction::DotProduct,
      )?);
      w.add_document(doc)?;
      w.close()?;
    }

    {
      let w = IndexWriter::new(dir, new_index_writer_config(random))?;
      let mut doc = Document::new();
      doc.add(KnnFloatVectorField::with_similarity_function(
        "f",
        vec![0.0; 2],
        VectorSimilarityFunction::DotProduct,
      )?);
      let err = w.add_document(doc).unwrap_err();
      assert_eq!(
        format!(
          "cannot change field \"f\" from vector dimension=4, vector encoding={:?}, vector similarity function={:?} to inconsistent vector dimension=2, vector encoding={:?}, vector similarity function={:?}",
          VectorEncoding::FLOAT32(4),
          VectorSimilarityFunction::DotProduct,
          VectorEncoding::FLOAT32(4),
          VectorSimilarityFunction::DotProduct
        ),
        err.to_string()
      );
    }

    Ok(())
  }

  fn test_merging_with_different_knn_fields<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(random)?;
    let ex = Arc::new(AtomicBool::new(false));
    let merge_scheduler = TestMergeScheduler::new(ex.clone());
    let mut iwc = new_index_writer_config(random);
    iwc.set_merge_scheduler(MergeSchedulerEnum::KnnMergeScheduler(merge_scheduler));
    let mp = iwc.get_merge_policy().clone();
    iwc.set_merge_policy(MergePolicyEnum::Force(ForceMergePolicy::new(mp)));

    let writer = IndexWriter::new(dir, iwc)?;
    for i in 0..10 {
      let mut doc = Document::new();
      doc.add(KnnFloatVectorField::new(
        "field",
        vec![i as f32, (i + 1) as f32, (i + 2) as f32, (i + 3) as f32],
      )?);
      writer.add_document(doc)?;
    }
    writer.commit()?;

    for i in 0..10 {
      let mut doc = Document::new();
      doc.add(KnnFloatVectorField::new(
        "otherVector",
        vec![i as f32, i as f32, i as f32, i as f32],
      )?);
      writer.add_document(doc)?;
    }
    writer.commit()?;
    writer.force_merge(1)?;
    writer.close()?;

    assert!(!ex.load(Ordering::Relaxed));
    Ok(())
  }

  fn test_merging_with_different_byte_knn_fields<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(random)?;
    let ex = Arc::new(AtomicBool::new(false));
    let merge_scheduler = TestMergeScheduler::new(ex.clone());
    let mut iwc = new_index_writer_config(random);
    iwc.set_merge_scheduler(MergeSchedulerEnum::KnnMergeScheduler(merge_scheduler));
    let mp = iwc.get_merge_policy().clone();
    iwc.set_merge_policy(MergePolicyEnum::Force(ForceMergePolicy::new(mp)));

    let writer = IndexWriter::new(dir, iwc)?;
    for i in 0..10 {
      let mut doc = Document::new();
      doc.add(KnnByteVectorField::new(
        "field",
        vec![i as u8, i as u8, i as u8, i as u8],
      )?);
      writer.add_document(doc)?;
    }
    writer.commit()?;

    for i in 0..10 {
      let mut doc = Document::new();
      doc.add(KnnByteVectorField::new(
        "otherVector",
        vec![i as u8, i as u8, i as u8, i as u8],
      )?);
      writer.add_document(doc)?;
    }
    writer.commit()?;
    writer.force_merge(1)?;
    writer.close()?;

    assert!(!ex.load(Ordering::Relaxed));
    Ok(())
  }

  fn test_writer_ram_estimate<R>(&self, _random: &mut R) -> Result<()> {
    // TODO: memory calculation not implement
    Ok(())
  }

  fn test_illegal_similarity_function_change_two_writers<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(random)?;

    {
      let w = IndexWriter::new(dir.clone(), new_index_writer_config(random))?;
      let mut doc = Document::new();
      doc.add(KnnFloatVectorField::with_similarity_function(
        "f",
        vec![0.0; 4],
        VectorSimilarityFunction::DotProduct,
      )?);
      w.add_document(doc)?;
      w.close()?;
    }

    {
      let w = IndexWriter::new(dir, new_index_writer_config(random))?;
      let mut doc2 = Document::new();
      doc2.add(KnnFloatVectorField::with_similarity_function(
        "f",
        vec![0.0; 4],
        VectorSimilarityFunction::Euclidean,
      )?);
      let err = w.add_document(doc2).unwrap_err();
      assert_eq!(
        format!(
          "cannot change field \"f\" from vector dimension=4, vector encoding={:?}, vector similarity function={:?} to inconsistent vector dimension=4, vector encoding={:?}, vector similarity function={:?}",
          VectorEncoding::FLOAT32(4),
          VectorSimilarityFunction::DotProduct,
          VectorEncoding::FLOAT32(4),
          VectorSimilarityFunction::Euclidean
        ),
        err.to_string()
      );
    }

    Ok(())
  }

  fn test_add_indexes_directory0<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    // TODO add_indexes未实现
    Ok(())
  }

  fn test_add_indexes_directory1<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    // TODO add_indexes未实现
    Ok(())
  }

  fn test_add_indexes_directory01<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    // TODO add_indexes未实现
    Ok(())
  }

  fn test_illegal_dim_change_via_add_indexes_directory<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    // TODO add_indexes未实现
    Ok(())
  }

  fn test_illegal_similarity_function_change_via_add_indexes_directory<R>(
    &self,
    _random: &mut R,
  ) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    // TODO add_indexes未实现
    Ok(())
  }

  fn test_illegal_dim_change_via_add_indexes_codec_reader<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    // TODO add_indexes未实现
    Ok(())
  }

  fn test_illegal_similarity_function_change_via_add_indexes_codec_reader<R>(
    &self,
    _random: &mut R,
  ) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    // TODO add_indexes未实现
    Ok(())
  }

  /// TODO add_indexes_slowly未实现
  fn test_illegal_dim_change_via_add_indexes_slow_codec_reader<R>(
    &self,
    _random: &mut R,
  ) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    Ok(())
  }

  /// TODO add_indexes_slowly未实现
  fn test_illegal_similarity_function_change_via_add_indexes_slow_codec_reader<R>(
    &self,
    _random: &mut R,
  ) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    Ok(())
  }

  fn test_illegal_multiple_values<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(_random)?;
    let w = IndexWriter::new(dir, new_index_writer_config(_random))?;
    let mut doc = Document::new();
    doc.add(KnnFloatVectorField::with_similarity_function(
      "f",
      vec![0.0; 4],
      VectorSimilarityFunction::DotProduct,
    )?);
    doc.add(KnnFloatVectorField::with_similarity_function(
      "f",
      vec![0.0; 4],
      VectorSimilarityFunction::DotProduct,
    )?);
    let err = w.add_document(doc).unwrap_err();
    assert_eq!(
      "VectorValuesField \"f\" appears more than once in this document (only one value is allowed per field)",
      err.to_string()
    );
    Ok(())
  }

  fn test_illegal_dimension_too_large<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(_random)?;
    let w = IndexWriter::new(dir, new_index_writer_config(_random))?;
    let max_dim = self.get_vectors_max_dimensions("f");

    let mut doc = Document::new();
    doc.add(KnnFloatVectorField::with_similarity_function(
      "f",
      vec![0.0; max_dim + 1],
      VectorSimilarityFunction::DotProduct,
    )?);
    let exc = w.add_document(doc).unwrap_err();
    assert!(
      exc
        .to_string()
        .contains(&format!("vector's dimensions must be <= [{max_dim}]"))
    );

    let mut doc2 = Document::new();
    doc2.add(KnnFloatVectorField::with_similarity_function(
      "f",
      vec![0.0; 2],
      VectorSimilarityFunction::DotProduct,
    )?);
    w.add_document(doc2)?;

    let mut doc3 = Document::new();
    doc3.add(KnnFloatVectorField::with_similarity_function(
      "f",
      vec![0.0; max_dim + 1],
      VectorSimilarityFunction::DotProduct,
    )?);
    let exc = w.add_document(doc3).unwrap_err();
    let msg = exc.to_string();
    assert!(
      msg.contains("Inconsistency of field data structures across documents for field [f]")
        || msg.contains(&format!("vector's dimensions must be <= [{max_dim}]"))
    );
    w.flush()?;

    let mut doc4 = Document::new();
    doc4.add(KnnFloatVectorField::with_similarity_function(
      "f",
      vec![0.0; max_dim + 1],
      VectorSimilarityFunction::DotProduct,
    )?);
    let exc = w.add_document(doc4).unwrap_err();
    assert!(
      exc
        .to_string()
        .contains(&format!("vector's dimensions must be <= [{max_dim}]"))
    );
    Ok(())
  }

  fn test_illegal_empty_vector<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(_random)?;
    let w = IndexWriter::new(dir, new_index_writer_config(_random))?;

    let e = match KnnFloatVectorField::with_similarity_function(
      "f",
      vec![],
      VectorSimilarityFunction::Euclidean,
    ) {
      Ok(_) => unreachable!("expected empty vector creation to fail"),
      Err(err) => err,
    };
    assert_eq!("cannot index an empty vector", e.to_string());

    let mut doc2 = Document::new();
    doc2.add(KnnFloatVectorField::with_similarity_function(
      "f",
      vec![0.0; 2],
      VectorSimilarityFunction::Euclidean,
    )?);
    w.add_document(doc2)?;
    Ok(())
  }

  fn test_different_codecs1<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(random)?;

    {
      let w = IndexWriter::new(dir.clone(), new_index_writer_config(random))?;
      let mut doc = Document::new();
      doc.add(KnnFloatVectorField::with_similarity_function(
        "f",
        vec![0.0; 4],
        VectorSimilarityFunction::DotProduct,
      )?);
      w.add_document(doc)?;
      w.close()?;
    }

    {
      let iwc = new_index_writer_config(random);
      // TODO set_codec 未实现
      // iwc.set_codec(Codec::for_name("SimpleText")?);
      let w = IndexWriter::new(dir, iwc)?;
      let mut doc = Document::new();
      doc.add(KnnFloatVectorField::with_similarity_function(
        "f",
        vec![0.0; 4],
        VectorSimilarityFunction::DotProduct,
      )?);
      w.add_document(doc)?;
      w.force_merge(1)?;
      w.close()?;
    }

    Ok(())
  }

  fn test_different_codecs2<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let iwc = new_index_writer_config(random);
    // TODO set_codec 未实现
    // iwc.set_codec(Codec::for_name("SimpleText")?);

    let dir = new_directory_shared(random)?;

    {
      let w = IndexWriter::new(dir.clone(), iwc)?;
      let mut doc = Document::new();
      doc.add(KnnFloatVectorField::with_similarity_function(
        "f",
        vec![0.0; 4],
        VectorSimilarityFunction::DotProduct,
      )?);
      w.add_document(doc)?;
      w.close()?;
    }

    {
      let w = IndexWriter::new(dir, new_index_writer_config(random))?;
      let mut doc = Document::new();
      doc.add(KnnFloatVectorField::with_similarity_function(
        "f",
        vec![0.0; 4],
        VectorSimilarityFunction::DotProduct,
      )?);
      w.add_document(doc)?;
      w.force_merge(1)?;
      w.close()?;
    }

    Ok(())
  }

  fn test_invalid_knn_vector_field_usage<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut field = KnnFloatVectorField::with_similarity_function(
      "field",
      vec![0.0; 2],
      VectorSimilarityFunction::Euclidean,
    )?;

    assert!(field.set_int_value(14).is_err());

    let err = field.set_vector_value(vec![0.0; 1]).unwrap_err();
    assert!(matches!(err, LuceneError::IllegalArgument(_)));

    assert_eq!(None, field.numeric_value()?);
    Ok(())
  }

  fn test_delete_all_vector_docs<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(_random)?;
    let w = IndexWriter::new(dir, new_index_writer_config(_random))?;

    let mut doc = Document::new();
    doc.add(StringField::from_string("id", "0", Store::No)?);
    doc.add(KnnFloatVectorField::with_similarity_function(
      "v",
      vec![2.0, 3.0, 5.0, 6.0],
      VectorSimilarityFunction::DotProduct,
    )?);
    w.add_document(doc)?;
    w.add_document(Document::new())?;
    w.commit()?;

    {
      let reader = directory_reader_util::open_from_writer(&w)?;
      let leaf = get_only_leaf_reader(reader)?;
      let values = leaf.get_float_vector_values("v")?.expect("vector values");
      assert_eq!(1, values.size());
    }

    w.delete_documents_with_terms(vec![Term::from_text("id", "0")])?;
    w.force_merge(1)?;
    {
      let reader = directory_reader_util::open_from_writer(&w)?;
      let leaf = get_only_leaf_reader(reader)?;
      let values = leaf.get_float_vector_values("v")?.expect("vector values");
      assert_eq!(0, values.size());

      let mut collector = TopKnnCollector::new(1, i32::MAX as usize)?;
      leaf.search_nearest_vectors_f32(
        "v",
        vec![1.0, 0.0, 0.0, 0.0],
        &mut collector,
        leaf.get_live_docs()?,
      )?;
      let top_docs = collector.top_docs()?;
      assert_eq!(0, top_docs.score_docs.len());
      assert_eq!(NO_MORE_DOCS, values.iterator()?.next_doc()?);
    }
    Ok(())
  }

  fn test_knn_vector_field_missing_from_one_segment<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(_random)?;
    let w = IndexWriter::new(dir, new_index_writer_config(_random))?;

    let mut doc = Document::new();
    doc.add(StringField::from_string("id", "0", Store::No)?);
    doc.add(KnnFloatVectorField::with_similarity_function(
      "v0",
      vec![2.0, 3.0, 5.0, 6.0],
      VectorSimilarityFunction::DotProduct,
    )?);
    w.add_document(doc)?;
    w.commit()?;

    let mut doc = Document::new();
    doc.add(KnnFloatVectorField::with_similarity_function(
      "v1",
      vec![2.0, 3.0, 5.0, 6.0],
      VectorSimilarityFunction::DotProduct,
    )?);
    w.add_document(doc)?;
    w.force_merge(1)?;
    Ok(())
  }

  fn random_similarity<R>(&self, random: &mut R) -> VectorSimilarityFunction
  where
    R: Rng + ?Sized,
  {
    VectorSimilarityFunction::random(random)
  }

  fn random_vector_encoding<R>(&self, random: &mut R) -> VectorEncoding
  where
    R: Rng + ?Sized,
  {
    VectorEncoding::random(random)
  }
}
pub struct TestMergeScheduler {
  ex: Arc<AtomicBool>,
}
impl TestMergeScheduler {
  fn new(ex: Arc<AtomicBool>) -> Self {
    Self { ex }
  }
}

impl Closeable for TestMergeScheduler {}

impl MergeScheduler for TestMergeScheduler {
  fn merge<MS, D, L, B>(
    &self,
    merge_source: &MS,
    _trigger: MergeTrigger,
    writer: &IndexWriter<D, L, B>,
  ) -> Result<()>
  where
    MS: MergeSource,
    D: Directory,
    L: LiveIndexWriterConfig,
    B: IndexWriterBase,
  {
    while let Some(merge) = merge_source.get_next_merge(writer)? {
      let result: Result<()> = merge_source.merge(merge, writer);
      if result.is_err() {
        self.ex.store(true, Ordering::Relaxed);
        return result;
      }
    }
    Ok(())
  }

  type Directory<D>
    = D
  where
    D: Directory;

  fn wrap_for_merge<D>(&self, _in_: D) -> Result<Self::Directory<D>>
  where
    D: Directory,
  {
    Ok(_in_)
  }
}
