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
use crate::core::index::byte_vector_values::ByteVectorValues;
use crate::core::index::float_vector_values::FloatVectorValues;
use crate::core::index::index_reader::Identity;
use crate::core::index::vector_encoding::VectorEncoding;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::util::bits::{Bits, BitsEnum2};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::{HasIdentity, TryIntoInt};

/// This struct abstracts addressing of document vector values indexed as
/// `KnnFloatVectorField` or `KnnByteVectorField`.
pub trait KnnVectorValues {
  /// Return the dimension of the vectors
  fn dimension(&self) -> usize;
  /// Return the number of vectors for this field.
  ///
  /// # Returns
  /// The number of vectors returned by this iterator.
  fn size(&self) -> usize;
  /// Return the docid of the document indexed with the given vector ordinal.
  /// This default implementation returns the argument and is appropriate for
  /// dense values implementations where every doc has a single value.
  fn ord_to_doc(&self, ord: usize) -> Result<usize> {
    Ok(ord)
  }
  type KnnVectorValues: KnnVectorValues;
  /// Creates a new copy of this [`KnnVectorValues`]. This is helpful when you
  /// need to access different values at once, to avoid overwriting the
  /// underlying vector returned.
  fn copy(&self) -> Result<Self::KnnVectorValues> {
    Err(LuceneError::unsupported_operation(""))
  }
  /// Returns the vector byte length, defaults to dimension multiplied by
  /// float byte size
  fn get_vector_byte_length(&self) -> usize {
    self.dimension() * self.get_encoding().byte_size()
  }
  /// The vector encoding of these values.
  fn get_encoding(&self) -> VectorEncoding;

  type Bits<'a, B>: Bits
  where
    B: Bits,
    Self: 'a;
  /// Returns a Bits accepting docs accepted by the argument and having a
  /// vector value
  fn get_accept_ords<'a, B>(&'a self, accept_docs: Option<B>) -> Option<Self::Bits<'a, B>>
  where
    B: Bits;
  fn default_get_accept_ords<B>(&self, accept_docs: Option<B>) -> Option<BitsImpl1<B>>
  where
    B: Bits,
  {
    accept_docs.map(|accept_docs| BitsImpl1::new(accept_docs, self.size()))
  }

  type DocIndexIterator: DocIndexIterator;
  ///  Create an iterator for this instance.
  fn iterator(&self) -> Result<Self::DocIndexIterator> {
    Err(LuceneError::unsupported_operation(""))
  }
}

impl<T> KnnVectorValues for &T
where
  T: KnnVectorValues,
{
  fn dimension(&self) -> usize {
    (**self).dimension()
  }

  fn size(&self) -> usize {
    (**self).size()
  }

  fn ord_to_doc(&self, ord: usize) -> Result<usize> {
    (**self).ord_to_doc(ord)
  }

  type KnnVectorValues = T::KnnVectorValues;

  fn copy(&self) -> Result<Self::KnnVectorValues> {
    (**self).copy()
  }

  fn get_vector_byte_length(&self) -> usize {
    (**self).get_vector_byte_length()
  }

  fn get_encoding(&self) -> VectorEncoding {
    (**self).get_encoding()
  }

  type Bits<'a, B>
    = T::Bits<'a, B>
  where
    B: Bits,
    Self: 'a,
    T: 'a;

  fn get_accept_ords<'a, B>(&'a self, accept_docs: Option<B>) -> Option<Self::Bits<'a, B>>
  where
    B: Bits,
  {
    (**self).get_accept_ords(accept_docs)
  }

  type DocIndexIterator = T::DocIndexIterator;

  fn iterator(&self) -> Result<Self::DocIndexIterator> {
    (**self).iterator()
  }
}
pub struct BitsImpl1<B>
where
  B: Bits,
{
  accept_docs: B,
  id: Identity,
  length: usize,
}
impl<B> BitsImpl1<B>
where
  B: Bits,
{
  pub(crate) fn new(accept_docs: B, length: usize) -> Self {
    Self {
      accept_docs,
      id: Identity::new(),
      length,
    }
  }
}

impl<B> HasIdentity for BitsImpl1<B>
where
  B: Bits,
{
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl<B> Bits for BitsImpl1<B>
where
  B: Bits,
{
  fn get(&self, index: usize) -> Result<bool> {
    self.accept_docs.get(index)
  }

  fn length(&self) -> usize {
    self.length
  }
}

pub(crate) struct BitsImpl<B, T>
where
  B: Bits,
  T: OrdToDoc,
{
  accept_docs: B,
  size: usize,
  map: T,
  id: Identity,
}
impl<B, T> BitsImpl<B, T>
where
  B: Bits,
  T: OrdToDoc,
{
  pub(crate) fn new(accept_docs: B, size: usize, map: T) -> Self {
    Self {
      accept_docs,
      size,
      map,
      id: Identity::new(),
    }
  }
}

impl<B, T> HasIdentity for BitsImpl<B, T>
where
  B: Bits,
  T: OrdToDoc,
{
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl<B, T> Bits for BitsImpl<B, T>
where
  B: Bits,
  T: OrdToDoc,
{
  fn get(&self, index: usize) -> Result<bool> {
    self.accept_docs.get(self.map.ord_to_doc(index) as usize)
  }

  fn length(&self) -> usize {
    self.size
  }
}

/// A DocIdSetIterator that also provides an index() method tracking a distinct
/// ordinal for a vector associated with each doc.
pub trait DocIndexIterator: DocIdSetIterator {
  /// return the value index (aka "ordinal" or "ord") corresponding to the
  /// current doc
  fn index(&self) -> Result<i32>;
}

#[macro_export]
macro_rules! either_doc_index_iterator_named {
    ($vis:vis $name:ident { $( $Variant:ident : $T:ident ),+ $(,)? }) => {
        $vis enum $name<$( $T ),+> {
            $( $Variant($T), )+
        }

        impl<$( $T ),+> $crate::core::search::doc_id_set_iterator::DocIdSetIterator for $name<$( $T ),+>
        where
            $( $T: $crate::core::index::knn_vector_values::DocIndexIterator ),+
        {
            fn doc_id(&self) -> i32 {
                match self {
                    $( Self::$Variant(inner) => inner.doc_id(), )+
                }
            }

            fn next_doc(&mut self) -> Result<i32> {
                match self {
                    $( Self::$Variant(inner) => inner.next_doc(), )+
                }
            }

            fn advance(&mut self, target: i32) -> Result<i32> {
                match self {
                    $( Self::$Variant(inner) => inner.advance(target), )+
                }
            }

            fn slow_advance(&mut self, target: i32) -> Result<i32> {
                match self {
                    $( Self::$Variant(inner) => inner.slow_advance(target), )+
                }
            }

            fn cost(&self) -> Result<i64> {
                match self {
                    $( Self::$Variant(inner) => inner.cost(), )+
                }
            }
        }

        impl<$( $T ),+> $crate::core::index::knn_vector_values::DocIndexIterator for $name<$( $T ),+>
        where
            $( $T: $crate::core::index::knn_vector_values::DocIndexIterator ),+
        {
            fn index(&self) -> Result<i32> {
                match self {
                    $( Self::$Variant(inner) => inner.index(), )+
                }
            }
        }
    };
}

either_doc_index_iterator_named!(pub DocIndexIteratorEnum2 { A: A, B: B });

pub struct DenseDocIndexIterator {
  doc: i32,
  size: i32,
}
impl DenseDocIndexIterator {
  pub(crate) fn new(size: i32) -> Self {
    Self { doc: -1, size }
  }
}

impl DocIdSetIterator for DenseDocIndexIterator {
  fn doc_id(&self) -> i32 {
    self.doc
  }

  fn next_doc(&mut self) -> Result<i32> {
    if self.doc >= self.size - 1 {
      self.doc = NO_MORE_DOCS;
    } else {
      self.doc += 1;
    }
    Ok(self.doc)
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    if target >= self.size {
      self.doc = NO_MORE_DOCS;
    } else {
      self.doc = target;
    }
    Ok(self.doc)
  }

  fn cost(&self) -> Result<i64> {
    Ok(self.size as i64)
  }
}

impl DocIndexIterator for DenseDocIndexIterator {
  fn index(&self) -> Result<i32> {
    Ok(self.doc)
  }
}

pub(crate) struct DocIndexIteratorImpl2<D> {
  ord: i32,
  docs_with_field: D,
}

impl<D> DocIndexIteratorImpl2<D>
where
  D: DocIdSetIterator,
{
  pub(crate) fn new(docs_with_field: D) -> Self {
    Self {
      ord: -1,
      docs_with_field,
    }
  }
}

impl<D> DocIdSetIterator for DocIndexIteratorImpl2<D>
where
  D: DocIdSetIterator,
{
  fn doc_id(&self) -> i32 {
    self.docs_with_field.doc_id()
  }

  fn next_doc(&mut self) -> Result<i32> {
    if self.doc_id() == NO_MORE_DOCS {
      return Ok(NO_MORE_DOCS);
    }
    self.ord += 1;
    self.docs_with_field.next_doc()
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    self.docs_with_field.advance(target)
  }

  fn cost(&self) -> Result<i64> {
    self.docs_with_field.cost()
  }
}

impl<D> DocIndexIterator for DocIndexIteratorImpl2<D>
where
  D: DocIdSetIterator,
{
  fn index(&self) -> Result<i32> {
    Ok(self.ord)
  }
}

pub(crate) struct SparseDocIndexIterator<T>
where
  T: OrdToDoc,
{
  ord: i32,
  size: usize,
  map: T,
}

impl<T> SparseDocIndexIterator<T>
where
  T: OrdToDoc,
{
  pub(crate) fn new(size: usize, ord_to_doc: T) -> Self {
    Self {
      ord: -1,
      size,
      map: ord_to_doc,
    }
  }
}

impl<T> DocIdSetIterator for SparseDocIndexIterator<T>
where
  T: OrdToDoc,
{
  fn doc_id(&self) -> i32 {
    if self.ord == -1 {
      -1
    } else if self.ord == NO_MORE_DOCS {
      NO_MORE_DOCS
    } else {
      self.map.ord_to_doc(self.ord as usize)
    }
  }

  fn next_doc(&mut self) -> Result<i32> {
    if (self.ord + 1).try_convert()? >= self.size {
      self.ord = NO_MORE_DOCS;
    } else {
      self.ord += 1;
    }
    Ok(self.doc_id())
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    self.slow_advance(target)
  }

  fn cost(&self) -> Result<i64> {
    Ok(self.size as i64)
  }
}

impl<T> DocIndexIterator for SparseDocIndexIterator<T>
where
  T: OrdToDoc,
{
  fn index(&self) -> Result<i32> {
    Ok(self.ord)
  }
}

pub trait OrdToDoc {
  fn ord_to_doc(&self, ord: usize) -> i32;
}

pub(crate) fn create_dense_iterator(size: i32) -> DenseDocIndexIterator {
  DenseDocIndexIterator::new(size)
}

/// creates an iterator from a docidsetiterator indicating which docs have
/// values, and for which ordinals increase monotonically with docid.
pub(crate) fn from_disi<D>(disi: D) -> DocIndexIteratorImpl2<D>
where
  D: DocIdSetIterator,
{
  DocIndexIteratorImpl2::new(disi)
}

///  Creates an iterator from this instance's ordinal-to-docid mapping which
/// must be monotonic (docid increases when ordinal does).
pub(crate) fn create_sparse_iterator<T>(size: usize, map: T) -> SparseDocIndexIterator<T>
where
  T: OrdToDoc,
{
  SparseDocIndexIterator::new(size, map)
}
pub enum KnnVectorValuesType<B, F>
where
  B: ByteVectorValues,
  F: FloatVectorValues,
{
  Byte(B),
  Float(F),
}

#[macro_export]
macro_rules! either_knn_vector_values_named {
    (
        $vis:vis $name:ident {
            iter = $iter_ty:ident,
            bits = $bits_ty:ident;
            $( $Variant:ident : $T:ident ),+ $(,)?
        }
    ) => {
        $vis enum $name<$( $T ),+> {
            $( $Variant($T), )+
        }

        impl<$( $T ),+> $crate::core::index::knn_vector_values::KnnVectorValues for $name<$( $T ),+>
        where
            $( $T: $crate::core::index::knn_vector_values::KnnVectorValues ),+
        {
            #[inline]
            fn dimension(&self) -> usize {
                match self {
                    $( Self::$Variant(inner) => inner.dimension(), )+
                }
            }

            #[inline]
            fn size(&self) -> usize {
                match self {
                    $( Self::$Variant(inner) => inner.size(), )+
                }
            }

            #[inline]
            fn ord_to_doc(&self, ord: usize) -> $crate::core::util::error::lucene_error::Result<usize> {
                match self {
                    $( Self::$Variant(inner) => inner.ord_to_doc(ord), )+
                }
            }

            type KnnVectorValues =
                $name<$( < $T as $crate::core::index::knn_vector_values::KnnVectorValues >::KnnVectorValues ),+>;

            #[inline]
            fn copy(&self) -> $crate::core::util::error::lucene_error::Result<Self::KnnVectorValues> {
                match self {
                    $( Self::$Variant(inner) => inner.copy().map($name::$Variant), )+
                }
            }

            #[inline]
            fn get_vector_byte_length(&self) -> usize {
                match self {
                    $( Self::$Variant(inner) => inner.get_vector_byte_length(), )+
                }
            }

            #[inline]
            fn get_encoding(&self) -> $crate::core::index::vector_encoding::VectorEncoding {
                match self {
                    $( Self::$Variant(inner) => inner.get_encoding(), )+
                }
            }

            type Bits<'a, AcceptDocs> =
                $bits_ty<$( < $T as $crate::core::index::knn_vector_values::KnnVectorValues >::Bits<'a, AcceptDocs> ),+>
            where
                AcceptDocs: $crate::core::util::bits::Bits,
                Self: 'a;

            #[inline]
            fn get_accept_ords<'a, AcceptDocs>(&'a self, accept_docs: Option<AcceptDocs>) -> Option<Self::Bits<'a, AcceptDocs>>
            where
                AcceptDocs: $crate::core::util::bits::Bits,
            {
                match self {
                    $( Self::$Variant(inner) => inner.get_accept_ords(accept_docs).map($bits_ty::$Variant), )+
                }
            }

            type DocIndexIterator =
                $iter_ty<$( < $T as $crate::core::index::knn_vector_values::KnnVectorValues >::DocIndexIterator ),+>;

            #[inline]
            fn iterator(&self) -> $crate::core::util::error::lucene_error::Result<Self::DocIndexIterator> {
                match self {
                    $( Self::$Variant(inner) => inner.iterator().map($iter_ty::$Variant), )+
                }
            }
        }
    };
}

either_knn_vector_values_named!(
    pub KnnVectorValuesEnm2 {
        iter = DocIndexIteratorEnum2,
        bits = BitsEnum2;
        A: A, B: B,
    }
);
