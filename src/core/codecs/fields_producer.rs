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
use crate::core::codecs::DefaultPostingsFormat;
use crate::core::codecs::postings_format::PostingsFormat;
use crate::core::index::fields::{FieldIterEnum2, Fields};
use crate::core::index::terms::TermsEnum2;
use crate::core::util::close::Closeable;
use crate::core::util::error::lucene_error::Result;
use std::sync::Arc;
pub trait FieldsProducer: Fields + Closeable {
  /// Checks consistency of this reader.
  ///
  /// Note that this may be costly in terms of I/O, e.g. may involve computing
  /// a checksum value against large data files.
  fn check_integrity(&self) -> Result<()>;
  /// Returns an instance optimized for merging. This instance may only be
  /// cloned # Note
  /// Returning None means returning itself.
  fn get_merge_instance(&self) -> Result<Option<Self>>
  where
    Self: Sized,
  {
    Ok(None)
  }
}
pub type DefaultFieldsProducer<I> = <DefaultPostingsFormat as PostingsFormat>::FieldsProducer<I>;

macro_rules! either_fields_producer {
    ($vis:vis $name:ident { $( $Variant:ident : $T:ident ),+ $(,)? }) => {
        $vis enum $name<$( $T ),+> {
            $( $Variant($T), )+
        }

        impl<$( $T ),+> Closeable for $name<$( $T ),+>
        where
            $( $T: FieldsProducer ),+
        {
            fn close(&mut self) -> Result<()> {
                match self {
                    $( Self::$Variant(inner) => inner.close(), )+
                }
            }
        }

        impl<$( $T ),+> Fields for $name<$( $T ),+>
        where
            $( $T: FieldsProducer ),+
        {
            type FieldIter<'a> =
                FieldIterEnum2<'a, $( <$T as Fields>::FieldIter<'a> ),+>
            where
                $( $T: 'a ),+;

            type Terms = TermsEnum2<$( <$T as Fields>::Terms ),+>;

            fn iterator(&self) -> Result<Self::FieldIter<'_>> {
                match self {
                    $(
                        Self::$Variant(inner) => {
                            let it = inner.iterator()?;
                            Ok(FieldIterEnum2::$Variant(it))
                        }
                    ),+
                }
            }

            fn terms(&self, field: &str) -> Result<Option<Self::Terms>> {
                match self {
                    $(
                        Self::$Variant(inner) => {
                            let terms = inner.terms(field)?;
                            Ok(terms.map(TermsEnum2::$Variant))
                        }
                    ),+
                }
            }

            fn size(&self) -> Result<i32> {
                match self {
                    $( Self::$Variant(inner) => inner.size(), )+
                }
            }
        }

        impl<$( $T ),+> FieldsProducer for $name<$( $T ),+>
        where
            $( $T: FieldsProducer ),+
        {
            fn check_integrity(&self) -> Result<()> {
                match self {
                    $( Self::$Variant(inner) => inner.check_integrity(), )+
                }
            }

            fn get_merge_instance(&self) -> Result<Option<Self>>
            where
                Self: Sized,
            {
                match self {
                    $(
                        Self::$Variant(inner) => match inner.get_merge_instance()? {
                            Some(value) => Ok(Some(Self::$Variant(value))),
                            None => Ok(None),
                        },
                    )+
                }
            }
        }
    };
}

either_fields_producer!(pub FieldsProducerEnum2 { A: A, B: B });

impl<T> FieldsProducer for Arc<T>
where
  T: FieldsProducer,
{
  fn check_integrity(&self) -> Result<()> {
    (**self).check_integrity()
  }

  fn get_merge_instance(&self) -> Result<Option<Self>>
  where
    Self: Sized,
  {
    let v = match (**self).get_merge_instance()? {
      Some(v) => Arc::new(v),
      None => return Ok(None),
    };
    Ok(Some(v))
  }
}
