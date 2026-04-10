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
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::util::error::lucene_error::Result;

/// Computes the similarity score between a given query vector and different
/// document vectors. This is used for exact searching and scoring.
pub trait VectorScorer {
  /// Compute the score for the current document ID.
  ///
  /// # Returns
  /// The score for the current document ID
  ///
  /// # Errors
  /// Returns an error if an exception occurs during score computation.
  fn score(&self) -> Result<f32>;

  type DocIdSetIteratorRef<'a>: DocIdSetIterator
  where
    Self: 'a;
  /// Returns an iterator over the document IDs.
  fn iterator(&self) -> Self::DocIdSetIteratorRef<'_>;
  type DocIdSetIteratorMut<'a>: DocIdSetIterator
  where
    Self: 'a;
  fn iterator_mut(&mut self) -> Self::DocIdSetIteratorMut<'_>;
}

#[macro_export]
macro_rules! either_vector_scorer {
    (
        $vis:vis $name:ident {
            iter_ref = $iter_ref_ty:ident,
            iter_mut = $iter_mut_ty:ident;
            $( $Variant:ident : $T:ident ),+ $(,)?
        }
    ) => {
        $vis enum $name<$( $T ),+> {
            $( $Variant($T), )+
        }

        impl<$( $T ),+> $crate::core::search::vector_scorer::VectorScorer for $name<$( $T ),+>
        where
            $( $T: $crate::core::search::vector_scorer::VectorScorer ),+
        {
            #[inline]
            fn score(&self) -> $crate::core::util::error::lucene_error::Result<f32> {
                match self {
                    $( Self::$Variant(inner) => inner.score(), )+
                }
            }

            type DocIdSetIteratorRef<'a> =
                $crate::core::search::doc_id_set_iterator::$iter_ref_ty<
                    $( < $T as $crate::core::search::vector_scorer::VectorScorer >::DocIdSetIteratorRef<'a> ),+
                >
            where
                Self: 'a;

            #[inline]
            fn iterator(&self) -> Self::DocIdSetIteratorRef<'_> {
                match self {
                    $( Self::$Variant(inner) => $crate::core::search::doc_id_set_iterator::$iter_ref_ty::$Variant(inner.iterator()), )+
                }
            }

            type DocIdSetIteratorMut<'a> =
                $crate::core::search::doc_id_set_iterator::$iter_mut_ty<
                    $( < $T as $crate::core::search::vector_scorer::VectorScorer >::DocIdSetIteratorMut<'a> ),+
                >
            where
                Self: 'a;

            #[inline]
            fn iterator_mut(&mut self) -> Self::DocIdSetIteratorMut<'_> {
                match self {
                    $( Self::$Variant(inner) => $crate::core::search::doc_id_set_iterator::$iter_mut_ty::$Variant(inner.iterator_mut()), )+
                }
            }
        }
    };
}

either_vector_scorer!(
    pub VectorScorerEnum2 {
        iter_ref = DocIdSetIteratorEnum2,
        iter_mut = DocIdSetIteratorEnum2;
        A: A, B: B,
    }
);
#[cfg(test)]
mod tests {
  use crate::core::document::document::Document;
  use crate::core::document::field::Store;
  use crate::core::document::knn_byte_vector_field::KnnByteVectorField;
  use crate::core::document::knn_float_vector_field::KnnFloatVectorField;
  use crate::core::document::string_field::StringField;
  use crate::core::index::byte_vector_values::ByteVectorValues;
  use crate::core::index::composite_reader::get_context;
  use crate::core::index::directory_reader::directory_reader_util;
  use crate::core::index::float_vector_values::FloatVectorValues;
  use crate::core::index::index_reader_context::IndexReaderContext;
  use crate::core::index::leaf_reader::LeafReader;
  use crate::core::index::vector_encoding::VectorEncoding;
  use crate::core::index::vector_similarity_function::VectorSimilarityFunction::Euclidean;
  use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
  use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
  use crate::core::search::vector_scorer::VectorScorer;
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::index::random_index_writer::RandomIndexWriter;
  use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
    new_directory_shared, random,
  };
  use rand::Rng;
  use std::sync::Arc;

  #[allow(dead_code)] // for quick search
  struct TestVectorScorer;
  #[test]
  fn test_find_all() -> Result<()> {
    let mut random = random();
    let encoding = VectorEncoding::random(&mut random);
    let index_store = get_index_store(
      &mut random,
      encoding,
      &[&[0.0, 1.0], &[1.0, 2.0], &[0.0, 0.0]],
    )?;
    let reader = directory_reader_util::open(index_store)?;
    let reader = get_context(reader)?;
    let leafs = reader.leaves()?;
    assert_eq!(1, leafs.len());
    let leaf = leafs.first().unwrap().reader();

    let mut num_docs = 0;
    match encoding {
      VectorEncoding::BYTE(_) => {
        let vector_values = leaf.get_byte_vector_values("field")?.unwrap();
        let mut scorer = vector_values.scorer(vec![1_u8, 2_u8])?.unwrap();
        let mut iterator = scorer.iterator_mut();
        while iterator.next_doc()? != NO_MORE_DOCS {
          num_docs += 1;
        }
      },
      VectorEncoding::FLOAT32(_) => {
        let vector_values = leaf.get_float_vector_values("field")?.unwrap();
        let mut scorer = vector_values.scorer(vec![1.0_f32, 2.0_f32])?.unwrap();
        let mut iterator = scorer.iterator_mut();
        while iterator.next_doc()? != NO_MORE_DOCS {
          num_docs += 1;
        }
      },
    }

    assert_eq!(3, num_docs);
    Ok(())
  }
  fn get_index_store<R>(
    random: &mut R,
    encoding: VectorEncoding,
    contents: &[&[f32]],
  ) -> Result<Arc<crate::core::store::directory::DirEnum>>
  where
    R: Rng + ?Sized,
  {
    let index_store = new_directory_shared(random)?;
    let writer = RandomIndexWriter::new(random, index_store.clone());

    for (i, vector) in contents.iter().enumerate() {
      let mut doc = Document::new();
      match encoding {
        VectorEncoding::BYTE(_) => {
          let vector = vector.iter().map(|value| *value as u8).collect();
          doc.add(KnnByteVectorField::with_similarity_function(
            "field", vector, Euclidean,
          )?);
        },
        VectorEncoding::FLOAT32(_) => {
          doc.add(KnnFloatVectorField::with_similarity_function(
            "field",
            vector.to_vec(),
            Euclidean,
          )?);
        },
      }
      doc.add(StringField::from_string(
        "id",
        format!("id{i}"),
        Store::Yes,
      )?);
      writer.add_document(doc)?;
    }

    for _ in 0..5 {
      let mut doc = Document::new();
      doc.add(StringField::from_string("other", "value", Store::No)?);
      writer.add_document(doc)?;
    }

    writer.force_merge(1)?;
    writer.close()?;
    Ok(index_store)
  }
}
