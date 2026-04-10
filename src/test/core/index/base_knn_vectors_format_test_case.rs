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
use crate::core::document::knn_float_vector_field::KnnFloatVectorField;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::indexable_field::IndexableField;
use crate::core::index::indexable_field_type::IndexableFieldType;
use crate::core::index::vector_encoding::VectorEncoding;
use crate::core::index::vector_similarity_function::VectorSimilarityFunction;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::core::index::base_index_file_format_test_case::BaseIndexFileFormatTestCase;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  new_directory_shared, new_index_writer_config,
};
use rand::Rng;

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
