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
use crate::core::index::dummy::dummy_fields::DummyFields;
use crate::core::index::fields::{Either2Fields, Fields};
use crate::core::util::error::lucene_error::Result;
/// API for reading term vectors.
///
/// **NOTE**: This struct is not thread-safe and should only be consumed in the thread where it
/// was acquired.
pub trait TermVectors {
    /// Optional method: Give a hint to this [`TermVectors`] instance that the given document will
    /// be read in the near future. This typically delegates to [`IndexInput::prefetch`](crate::core::store::index_input::IndexInput::prefetch) and is
    /// useful to parallelize I/O across multiple documents.
    ///
    /// NOTE: This API is expected to be called on a small enough set of doc IDs that they could all
    /// fit in the page cache. If you plan on retrieving a very large number of documents, it may be a
    /// good idea to perform calls to [`Self::prefetch`] and [`Self::get`] in batches instead of
    /// prefetching all documents up-front.
    fn prefetch(&mut self, _doc_id: i32) -> Result<()> {
        Ok(())
    }
    /// The associated `Fields` type.
    type Fields: Fields;
    /// Returns term vectors for this document, or `None` if term vectors were not indexed.
    ///
    /// The returned [`Fields`] instance acts like a single-document inverted index (the `doc_id` will be
    /// `0`). If offsets are available they are in an [`OffsetAttribute`](crate::core::analysis::token_attributes::offset_attribute::OffsetAttribute) available from the [`PostingsEnum`](crate::core::index::postings_enum::PostingsEnum).
    fn get(&mut self, doc: i32) -> Result<Option<Self::Fields>>;
    /// Returns term vectors for this document, or `None` if term vectors were not indexed.
    ///
    /// The returned [`Fields`] instance acts like a single-document inverted index (the `doc_id` will be
    /// `0`). If offsets are available they are in an [`OffsetAttribute`](crate::core::analysis::token_attributes::offset_attribute::OffsetAttribute) available from the [`PostingsEnum`](crate::core::index::postings_enum::PostingsEnum).
    fn get_field_terms(
        &mut self,
        doc: i32,
        field: &str,
    ) -> Result<Option<<Self::Fields as Fields>::Terms>> {
        match self.get(doc)? {
            Some(fields) => fields.terms(field),
            None => Ok(None),
        }
    }
}
/// Instance that never returns term vectors
pub struct EmptyTermVectors;

impl TermVectors for EmptyTermVectors {
    type Fields = DummyFields;

    fn get(&mut self, _doc: i32) -> Result<Option<Self::Fields>> {
        Ok(None)
    }
}
macro_rules! either_term_vectors {
    ($vis:vis $name:ident => { fe: $fe:ident } { $( $Variant:ident : $T:ident ),+ $(,)? }) => {
        $vis enum $name<$( $T ),+> {
            $( $Variant($T), )+
        }

        impl<$( $T ),+> TermVectors for $name<$( $T ),+>
        where
            $( $T: TermVectors ),+
        {
            type Fields = $fe<$( <$T as TermVectors>::Fields ),+>;

            fn prefetch(&mut self, doc_id: i32) -> Result<()> {
                match self {
                    $( Self::$Variant(inner) => inner.prefetch(doc_id), )+
                }
            }

            fn get(&mut self, doc: i32) -> Result<Option<Self::Fields>> {
                match self {
                    $(
                        Self::$Variant(inner) => {
                            let fields = inner.get(doc)?;
                            Ok(fields.map($fe::$Variant))
                        }
                    ),+
                }
            }
        }
    };
}

either_term_vectors!(
    pub Either2TermVectors => { fe: Either2Fields } { A: A, B: B }
);
