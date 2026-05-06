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
use crate::core::codecs::DefaultTermVectorsFormat;
use crate::core::codecs::term_vectors_format::TermVectorsFormat;
use crate::core::index::fields::{Fields, FieldsEnum2};
use crate::core::index::term_vectors::{RawTermVectors, TermVectors};
use crate::core::index::terms::TermsEnum2;
use crate::core::util::error::lucene_error::Result;
/// Codec API for reading term vectors:
pub trait TermVectorsReader: TermVectors + Clone {
  /// Checks consistency of this reader.
  ///
  /// Note that this may be costly in terms of I/O, e.g. may involve computing
  /// a checksum value against large data files.
  fn check_integrity(&self) -> Result<()>;

  /// Returns an instance optimized for merging.
  ///
  /// This instance may only be used from the thread that acquires it.
  fn get_merge_instance(&self) -> Result<Option<Self>>
  where
    Self: Sized,
  {
    Ok(None)
  }
}
pub type DefaultTermVectorsReader<I> =
  <DefaultTermVectorsFormat as TermVectorsFormat>::TermVectorsReader<I>;

macro_rules! either_term_vectors_reader {
    ($vis:vis $name:ident => { fe: $fe:ident, te: $te:ident } { $Variant1:ident : $T1:ident, $( $Variant:ident : $T:ident ),+ $(,)? }) => {
        $vis enum $name<$T1, $( $T ),+> {
            $Variant1($T1),
            $( $Variant($T), )+
        }

        impl<$T1, $( $T ),+> TermVectors for $name<$T1, $( $T ),+>
        where
            $T1: TermVectorsReader,
            $( $T: TermVectorsReader + RawTermVectors<IndexInput = <$T1 as RawTermVectors>::IndexInput> ),+
        {
            type Fields = $fe<
                <$T1 as TermVectors>::Fields,
                $( <$T as TermVectors>::Fields ),+
            >;

            type Terms = $te<
                <<$T1 as TermVectors>::Fields as Fields>::Terms,
                $( <<$T as TermVectors>::Fields as Fields>::Terms ),+
            >;

            fn prefetch(&mut self, doc_id: i32) -> Result<()> {
                match self {
                    Self::$Variant1(inner) => inner.prefetch(doc_id),
                    $( Self::$Variant(inner) => inner.prefetch(doc_id), )+
                }
            }

            fn get(&mut self, doc: i32) -> Result<Option<Self::Fields>> {
                match self {
                    Self::$Variant1(inner) => {
                        let fields = inner.get(doc)?;
                        Ok(fields.map($fe::$Variant1))
                    }
                    $(
                        Self::$Variant(inner) => {
                            let fields = inner.get(doc)?;
                            Ok(fields.map($fe::$Variant))
                        }
                    ),+
                }
            }

            fn get_field_terms(
                &mut self,
                doc: i32,
                field: &str,
            ) -> Result<Option<<Self::Fields as Fields>::Terms>> {
                match self {
                    Self::$Variant1(inner) => {
                        let terms = inner.get_field_terms(doc, field)?;
                        Ok(terms.map($te::$Variant1))
                    }
                    $(
                        Self::$Variant(inner) => {
                            let terms = inner.get_field_terms(doc, field)?;
                            Ok(terms.map($te::$Variant))
                        }
                    ),+
                }
            }
        }

        impl<$T1, $( $T ),+> Clone for $name<$T1, $( $T ),+>
        where
            $T1: TermVectorsReader,
            $( $T: TermVectorsReader ),+
        {
            fn clone(&self) -> Self {
                match self {
                    Self::$Variant1(inner) => Self::$Variant1(inner.clone()),
                    $( Self::$Variant(inner) => Self::$Variant(inner.clone()), )+
                }
            }
        }

        impl<$T1, $( $T ),+> TermVectorsReader for $name<$T1, $( $T ),+>
        where
            $T1: TermVectorsReader,
            $( $T: TermVectorsReader + RawTermVectors<IndexInput = <$T1 as RawTermVectors>::IndexInput> ),+
        {
            fn check_integrity(&self) -> Result<()> {
                match self {
                    Self::$Variant1(inner) => inner.check_integrity(),
                    $( Self::$Variant(inner) => inner.check_integrity(), )+
                }
            }

            fn get_merge_instance(&self) -> Result<Option<Self>>
            where
                Self: Sized,
            {
                match self {
                    Self::$Variant1(inner) => match inner.get_merge_instance()? {
                        Some(value) => Ok(Some(Self::$Variant1(value))),
                        None => Ok(None),
                    },
                    $( Self::$Variant(inner) => match inner.get_merge_instance()? {
                        Some(value) => Ok(Some(Self::$Variant(value))),
                        None => Ok(None),
                    }, )+
                }
            }
        }
    };
}

either_term_vectors_reader!(
    pub TermVectorsReaderEnum2 => { fe: FieldsEnum2, te: TermsEnum2 } { A: A, B: B }
);

impl<A, B> RawTermVectors for TermVectorsReaderEnum2<A, B>
where
  A: RawTermVectors,
  B: RawTermVectors<IndexInput = A::IndexInput>,
{
  type IndexInput = A::IndexInput;

  fn raw_term_vectors_mut(&mut self) -> Result<&mut DefaultTermVectorsReader<Self::IndexInput>> {
    match self {
      Self::A(inner) => inner.raw_term_vectors_mut(),
      Self::B(inner) => inner.raw_term_vectors_mut(),
    }
  }

  fn raw_term_vectors(&self) -> Result<&DefaultTermVectorsReader<Self::IndexInput>> {
    match self {
      Self::A(inner) => inner.raw_term_vectors(),
      Self::B(inner) => inner.raw_term_vectors(),
    }
  }
}

#[cfg(test)]
mod tests {
  use crate::core::analysis::analyzer::{
    Analyzer, AnalyzerEnum, BoxedAnalyzer, TokenStreamComponents,
  };
  use crate::core::analysis::reader::ReaderEnum;
  use crate::core::analysis::token_stream::{TokenStream, default_attribute};
  use crate::core::analysis::tokenizer::{Tokenizer, TokenizerBase};
  use crate::core::codecs::codec::{Codec, LATEST_CODEC};
  use crate::core::codecs::term_vectors_format::TermVectorsFormat;
  use crate::core::document::document::Document;
  use crate::core::document::field::Field;
  use crate::core::document::field_type::FieldType;
  use crate::core::document::stored_field::stored_field_type;
  use crate::core::document::text_field::text_field_type;
  use crate::core::index::composite_reader::get_context;
  use crate::core::index::directory_reader;
  use crate::core::index::field_infos::FieldInfos;
  use crate::core::index::fields::Fields;
  use crate::core::index::index_reader_context::IndexReaderContext;
  use crate::core::index::index_writer::{IndexWriter, read_field_infos};
  use crate::core::index::leaf_reader::LeafReader;
  use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
  use crate::core::index::log_merge_policy::LogMergePolicy;
  use crate::core::index::postings_enum::{ALL, NONE, PostingsEnum};
  use crate::core::index::segment_commit_info::SegmentCommitInfo;
  use crate::core::index::term_vectors::TermVectors;
  use crate::core::index::terms::Terms;
  use crate::core::index::terms_enum::TermsEnum;
  use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, NO_MORE_DOCS};
  use crate::core::store::directory::DirEnum;
  use crate::core::util::attribute_source::{AttributeSource, Attributes};
  use crate::core::util::bytes_ref_iterator::BytesRefIterator;
  use crate::core::util::error::lucene_error::{LuceneError, Result};
  use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
  use crate::test::core::index::random_index_writer::RandomIndexWriter;
  use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
    new_directory_shared, new_index_writer_config_with_analyzer, new_io_context, random,
  };
  use rand::RngExt;
  use std::sync::Arc;

  #[allow(dead_code)] // for quick search
  struct TestTermVectorsReader;

  const TERM_FREQ: usize = 3;

  struct TestTermVectorsReaderSetup {
    test_fields: Vec<&'static str>,
    test_fields_store_pos: Vec<bool>,
    test_fields_store_off: Vec<bool>,
    test_terms: Vec<&'static str>,
    positions: Vec<Vec<i32>>,
    dir: Arc<DirEnum>,
    seg: SegmentCommitInfo<DirEnum>,
    field_infos: Arc<FieldInfos>,
    tokens: Vec<TestToken>,
  }

  impl TestTermVectorsReader {
    fn setup() -> Result<TestTermVectorsReaderSetup> {
      let mut test_fields = vec!["f1", "f2", "f3", "f4"];
      let test_fields_store_pos = vec![true, false, true, false];
      let test_fields_store_off = vec![true, false, false, true];
      let mut test_terms = vec!["this", "is", "a", "test"];
      let mut positions = vec![Vec::new(); test_terms.len()];
      let mut tokens = Vec::with_capacity(test_terms.len() * TERM_FREQ);

      test_fields.sort_unstable();
      test_terms.sort_unstable();

      let mut random = random();
      for (i, term) in test_terms.iter().enumerate() {
        for j in 0..TERM_FREQ {
          let position = (j as i32 * 10) + (random.random::<f64>() * 10.0) as i32;
          positions[i].push(position);
          tokens.push(TestToken {
            text: (*term).to_string(),
            pos: position,
            start_offset: j as i32 * 10,
            end_offset: j as i32 * 10 + term.len() as i32,
          });
        }
      }
      tokens.sort();

      let dir = new_directory_shared(&mut random)?;
      let mut config =
        new_index_writer_config_with_analyzer(&mut random, MyAnalyzer::new(tokens.clone()));
      config.set_max_buffered_docs(-1);
      config.set_merge_policy(LogMergePolicy::log_doc());
      config.set_use_compound_file(false);
      let writer = IndexWriter::new(dir.clone(), config)?;

      let mut doc = Document::new();
      for i in 0..test_fields.len() {
        let mut custom_type = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
        custom_type.set_store_term_vectors(true)?;
        if test_fields_store_pos[i] {
          custom_type.set_store_term_vector_positions(true)?;
        }
        if test_fields_store_off[i] {
          custom_type.set_store_term_vector_offsets(true)?;
        }
        doc.add(Field::new(test_fields[i], "", custom_type));
      }

      for _ in 0..5 {
        writer.add_document(doc.clone())?;
      }
      writer.commit()?;

      let segment_infos = writer.clone_segment_infos()?;
      let seg = segment_infos
        .iter()
        .last()
        .cloned()
        .ok_or_else(|| LuceneError::illegal_state("expected at least one segment"))?;
      writer.close()?;

      let field_infos = Arc::new(read_field_infos(&seg)?);

      Ok(TestTermVectorsReaderSetup {
        test_fields,
        test_fields_store_pos,
        test_fields_store_off,
        test_terms,
        positions,
        dir,
        seg,
        field_infos,
        tokens,
      })
    }
  }

  #[test]
  fn test() -> Result<()> {
    let setup = TestTermVectorsReader::setup()?;
    let reader = directory_reader::open(setup.dir.clone())?;
    let reader = get_context(reader)?;
    let leaves = reader.leaves()?;

    for ctx in leaves {
      let sr = ctx.reader();
      assert!(sr.get_field_infos()?.has_term_vectors());
    }

    Ok(())
  }
  #[test]
  fn test_reader() -> Result<()> {
    let setup = TestTermVectorsReader::setup()?;
    let mut random = random();
    let mut reader = LATEST_CODEC.term_vectors_format().vectors_reader(
      setup.dir.as_ref(),
      &setup.seg.info,
      setup.field_infos.clone(),
      &new_io_context(&mut random)?,
    )?;

    for j in 0..5 {
      let fields = reader.get(j)?.expect("term vectors must exist");
      let vector = fields
        .terms(setup.test_fields[0])?
        .expect("term vector field must exist");
      assert_eq!(setup.test_terms.len() as i64, vector.size()?);
      let mut terms_enum = vector.iterator()?;
      for expected in &setup.test_terms {
        let text = terms_enum.next()?.expect("term must exist");
        assert_eq!(*expected, text.utf8_to_string()?);
      }
      assert!(terms_enum.next()?.is_none());
    }
    Ok(())
  }
  #[test]
  fn test_docs_enum() -> Result<()> {
    let setup = TestTermVectorsReader::setup()?;
    let mut random = random();
    let mut reader = LATEST_CODEC.term_vectors_format().vectors_reader(
      setup.dir.as_ref(),
      &setup.seg.info,
      setup.field_infos.clone(),
      &new_io_context(&mut random)?,
    )?;

    for j in 0..5 {
      let fields = reader.get(j)?.expect("term vectors must exist");
      let vector = fields
        .terms(setup.test_fields[0])?
        .expect("term vector field must exist");
      assert_eq!(setup.test_terms.len() as i64, vector.size()?);
      let mut terms_enum = vector.iterator()?;
      for expected in &setup.test_terms {
        let text = terms_enum.next()?.expect("term must exist");
        assert_eq!(*expected, text.utf8_to_string()?);

        let mut postings_enum = terms_enum.postings_with_flags(None, NONE as i32)?;
        assert_eq!(-1, postings_enum.doc_id());
        assert_ne!(NO_MORE_DOCS, postings_enum.next_doc()?);
        assert_eq!(NO_MORE_DOCS, postings_enum.next_doc()?);
      }
      assert!(terms_enum.next()?.is_none());
    }
    Ok(())
  }

  #[test]
  fn test_position_reader() -> Result<()> {
    let setup = TestTermVectorsReader::setup()?;
    let mut random = random();
    let mut reader = LATEST_CODEC.term_vectors_format().vectors_reader(
      setup.dir.as_ref(),
      &setup.seg.info,
      setup.field_infos.clone(),
      &new_io_context(&mut random)?,
    )?;

    let fields = reader.get(0)?.expect("term vectors must exist");
    let vector = fields
      .terms(setup.test_fields[0])?
      .expect("term vector field must exist");
    assert_eq!(setup.test_terms.len() as i64, vector.size()?);
    let mut terms_enum = vector.iterator()?;
    for (i, expected) in setup.test_terms.iter().enumerate() {
      let text = terms_enum.next()?.expect("term must exist");
      assert_eq!(*expected, text.utf8_to_string()?);

      let mut dp_enum = terms_enum.postings_with_flags(None, ALL as i32)?;
      assert_eq!(-1, dp_enum.doc_id());
      assert_ne!(NO_MORE_DOCS, dp_enum.next_doc()?);
      assert_eq!(setup.positions[i].len() as i32, dp_enum.freq()?);
      for position in &setup.positions[i] {
        assert_eq!(*position, dp_enum.next_position()?);
      }
      assert_eq!(NO_MORE_DOCS, dp_enum.next_doc()?);

      let mut dp_enum = terms_enum.postings_with_flags(None, ALL as i32)?;
      assert_eq!(-1, dp_enum.doc_id());
      assert_ne!(NO_MORE_DOCS, dp_enum.next_doc()?);
      assert_eq!(setup.positions[i].len() as i32, dp_enum.freq()?);
      for (j, position) in setup.positions[i].iter().enumerate() {
        assert_eq!(*position, dp_enum.next_position()?);
        assert_eq!(j as i32 * 10, dp_enum.start_offset()?);
        assert_eq!(
          j as i32 * 10 + setup.test_terms[i].len() as i32,
          dp_enum.end_offset()?
        );
      }
      assert_eq!(NO_MORE_DOCS, dp_enum.next_doc()?);
    }

    let freq_vector = fields
      .terms(setup.test_fields[1])?
      .expect("freq term vector field must exist");
    assert_eq!(setup.test_terms.len() as i64, freq_vector.size()?);
    let mut terms_enum = freq_vector.iterator()?;
    for expected in &setup.test_terms {
      let text = terms_enum.next()?.expect("term must exist");
      assert_eq!(*expected, text.utf8_to_string()?);
      let _ = terms_enum.postings(None)?;
      let _ = terms_enum.postings_with_flags(None, ALL as i32)?;
    }
    Ok(())
  }
  #[test]
  fn test_offset_reader() -> Result<()> {
    let setup = TestTermVectorsReader::setup()?;
    let mut random = random();
    let mut reader = LATEST_CODEC.term_vectors_format().vectors_reader(
      setup.dir.as_ref(),
      &setup.seg.info,
      setup.field_infos.clone(),
      &new_io_context(&mut random)?,
    )?;

    let fields = reader.get(0)?.expect("term vectors must exist");
    let vector = fields
      .terms(setup.test_fields[0])?
      .expect("term vector field must exist");
    assert_eq!(setup.test_terms.len() as i64, vector.size()?);
    let mut terms_enum = vector.iterator()?;
    for (i, expected) in setup.test_terms.iter().enumerate() {
      let text = terms_enum.next()?.expect("term must exist");
      assert_eq!(*expected, text.utf8_to_string()?);

      let mut dp_enum = terms_enum.postings_with_flags(None, ALL as i32)?;
      assert_ne!(NO_MORE_DOCS, dp_enum.next_doc()?);
      assert_eq!(setup.positions[i].len() as i32, dp_enum.freq()?);
      for position in &setup.positions[i] {
        assert_eq!(*position, dp_enum.next_position()?);
      }
      assert_eq!(NO_MORE_DOCS, dp_enum.next_doc()?);

      let mut dp_enum = terms_enum.postings_with_flags(None, ALL as i32)?;
      assert_ne!(NO_MORE_DOCS, dp_enum.next_doc()?);
      assert_eq!(setup.positions[i].len() as i32, dp_enum.freq()?);
      for (j, position) in setup.positions[i].iter().enumerate() {
        assert_eq!(*position, dp_enum.next_position()?);
        assert_eq!(j as i32 * 10, dp_enum.start_offset()?);
        assert_eq!(
          j as i32 * 10 + setup.test_terms[i].len() as i32,
          dp_enum.end_offset()?
        );
      }
      assert_eq!(NO_MORE_DOCS, dp_enum.next_doc()?);
    }
    Ok(())
  }
  #[test]
  fn test_illegal_payloads_without_positions() -> Result<()> {
    let mut random = random();

    let dir = new_directory_shared(&mut random)?;

    let mock = MockAnalyzer::new(&mut random);
    let w = RandomIndexWriter::with_analyzer(&mut random, dir.clone(), mock);

    let mut ft = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
    ft.set_store_term_vectors(true)?;
    ft.set_store_term_vector_payloads(true)?;

    let mut doc = Document::new();
    doc.add(Field::new("field", "value", ft));

    let err = w.add_document(doc).unwrap_err();
    match err {
      LuceneError::IllegalArgument(msg) => {
        assert_eq!(
          msg.to_string(),
          "cannot index term vector payloads without term vector positions (field=\"field\")"
        );
      },
      _ => unreachable!("{:?}", err),
    }

    w.close()?;
    Ok(())
  }
  #[test]
  fn test_illegal_offsets_without_vectors() -> Result<()> {
    let mut random = random();

    let dir = new_directory_shared(&mut random)?;

    let mut a = MockAnalyzer::new(&mut random);
    a.set_enable_checks(false);
    let w = RandomIndexWriter::with_analyzer(&mut random, dir.clone(), a);

    let mut ft = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
    ft.set_store_term_vectors(false)?;
    ft.set_store_term_vector_offsets(true)?;

    let mut doc = Document::new();
    doc.add(Field::new("field", "value", ft));

    let err = w.add_document(doc).unwrap_err();
    match err {
      LuceneError::IllegalArgument(msg) => {
        assert_eq!(
          msg.to_string(),
          "cannot index term vector offsets when term vectors are not indexed (field=\"field\")"
        );
      },
      _ => unreachable!("{:?}", err),
    }

    w.close()?;
    Ok(())
  }
  #[test]
  fn test_illegal_positions_without_vectors() -> Result<()> {
    let mut random = random();

    let dir = new_directory_shared(&mut random)?;

    let mock = MockAnalyzer::new(&mut random);
    let w = RandomIndexWriter::with_analyzer(&mut random, dir.clone(), mock);

    let mut ft = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
    ft.set_store_term_vectors(false)?;
    ft.set_store_term_vector_positions(true)?;

    let mut doc = Document::new();
    doc.add(Field::new("field", "value", ft));

    let err = w.add_document(doc).unwrap_err();
    match err {
      LuceneError::IllegalArgument(msg) => {
        assert_eq!(
          msg.to_string(),
          "cannot index term vector positions when term vectors are not indexed (field=\"field\")"
        );
      },
      _ => unreachable!("{:?}", err),
    }

    w.close()?;
    Ok(())
  }
  #[test]
  fn test_illegal_vector_payloads_without_vectors() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;
    let mut a = MockAnalyzer::new(&mut random);
    a.set_enable_checks(false);
    let w = RandomIndexWriter::with_analyzer(&mut random, dir.clone(), a);

    let mut ft = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
    ft.set_store_term_vectors(false)?;
    ft.set_store_term_vector_payloads(true)?;

    let mut doc = Document::new();
    doc.add(Field::new("field", "value", ft));

    let err = w.add_document(doc).unwrap_err();
    match err {
      LuceneError::IllegalArgument(msg) => {
        assert_eq!(
          msg.to_string(),
          "cannot index term vector payloads when term vectors are not indexed (field=\"field\")"
        );
      },
      _ => unreachable!("{err:?}"),
    }

    w.close()?;
    Ok(())
  }

  #[test]
  fn test_illegal_vectors_without_indexed() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;
    let mut a = MockAnalyzer::new(&mut random);
    a.set_enable_checks(false);
    let w = RandomIndexWriter::with_analyzer(&mut random, dir.clone(), a);

    let mut ft = FieldType::from_ref(&*stored_field_type::TYPE)?;
    ft.set_store_term_vectors(true)?;

    let mut doc = Document::new();
    doc.add(Field::new("field", "value", ft));

    let err = w.add_document(doc).unwrap_err();
    match err {
      LuceneError::IllegalArgument(msg) => {
        assert_eq!(
          msg.to_string(),
          "cannot store term vectors for a field that is not indexed (field=\"field\")"
        );
      },
      _ => unreachable!("{err:?}"),
    }

    w.close()?;
    Ok(())
  }

  #[test]
  fn test_illegal_vector_positions_without_indexed() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;
    let mut a = MockAnalyzer::new(&mut random);
    a.set_enable_checks(false);
    let w = RandomIndexWriter::with_analyzer(&mut random, dir.clone(), a);

    let mut ft = FieldType::from_ref(&*stored_field_type::TYPE)?;
    ft.set_store_term_vector_positions(true)?;

    let mut doc = Document::new();
    doc.add(Field::new("field", "value", ft));

    let err = w.add_document(doc).unwrap_err();
    match err {
      LuceneError::IllegalArgument(msg) => {
        assert_eq!(
          msg.to_string(),
          "cannot store term vector positions for a field that is not indexed (field=\"field\")"
        );
      },
      _ => unreachable!("{err:?}"),
    }

    w.close()?;
    Ok(())
  }

  #[test]
  fn test_illegal_vector_offsets_without_indexed() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;
    let mut a = MockAnalyzer::new(&mut random);
    a.set_enable_checks(false);
    let w = RandomIndexWriter::with_analyzer(&mut random, dir.clone(), a);

    let mut ft = FieldType::from_ref(&*stored_field_type::TYPE)?;
    ft.set_store_term_vector_offsets(true)?;

    let mut doc = Document::new();
    doc.add(Field::new("field", "value", ft));

    let err = w.add_document(doc).unwrap_err();
    match err {
      LuceneError::IllegalArgument(msg) => {
        assert_eq!(
          msg.to_string(),
          "cannot store term vector offsets for a field that is not indexed (field=\"field\")"
        );
      },
      _ => unreachable!("{err:?}"),
    }

    w.close()?;
    Ok(())
  }

  #[test]
  fn test_illegal_vector_payloads_without_indexed() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;
    let mut a = MockAnalyzer::new(&mut random);
    a.set_enable_checks(false);
    let w = RandomIndexWriter::with_analyzer(&mut random, dir.clone(), a);

    let mut ft = FieldType::from_ref(&*stored_field_type::TYPE)?;
    ft.set_store_term_vector_payloads(true)?;

    let mut doc = Document::new();
    doc.add(Field::new("field", "value", ft));

    let err = w.add_document(doc).unwrap_err();
    match err {
      LuceneError::IllegalArgument(msg) => {
        assert_eq!(
          msg.to_string(),
          "cannot store term vector payloads for a field that is not indexed (field=\"field\")"
        );
      },
      _ => unreachable!("{err:?}"),
    }

    w.close()?;
    Ok(())
  }

  struct MyAnalyzer {
    tokens: Vec<TestToken>,
  }

  impl MyAnalyzer {
    fn new(tokens: Vec<TestToken>) -> Self {
      Self { tokens }
    }
  }

  impl From<MyAnalyzer> for AnalyzerEnum {
    fn from(analyzer: MyAnalyzer) -> Self {
      let tokens = analyzer.tokens;
      AnalyzerEnum::Custom(BoxedAnalyzer::new(move |_field_name| {
        Ok(TokenStreamComponents::new(
          Box::new(MyTokenizer::new(tokens.clone())) as Box<dyn TokenStream + Send + Sync>,
          None,
        ))
      }))
    }
  }

  impl Analyzer for MyAnalyzer {
    fn create_components(&self, _field_name: &str) -> Result<TokenStreamComponents> {
      Ok(TokenStreamComponents::new(
        Box::new(MyTokenizer::new(self.tokens.clone())) as Box<dyn TokenStream + Send + Sync>,
        None,
      ))
    }

    type TokenStream<TS>
      = TS
    where
      TS: TokenStream;

    fn normalize_from_ts<TS>(&self, field_name: &str, in_: TS) -> Result<Self::TokenStream<TS>>
    where
      TS: TokenStream,
    {
      self.default_normalize_from_ts(field_name, in_)
    }

    fn get_offset_gap(&self, field_name: &str) -> i32 {
      self.default_get_offset_gap(field_name)
    }
  }

  #[derive(Clone, Eq, PartialEq)]
  struct TestToken {
    text: String,
    start_offset: i32,
    end_offset: i32,
    pos: i32,
  }

  impl Ord for TestToken {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
      self.pos.cmp(&other.pos)
    }
  }

  impl PartialOrd for TestToken {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
      Some(self.cmp(other))
    }
  }

  struct MyTokenizer {
    token_upto: usize,
    tokens: Vec<TestToken>,
    tokenizer_base: TokenizerBase,
  }

  impl MyTokenizer {
    fn new(tokens: Vec<TestToken>) -> Self {
      Self {
        token_upto: 0,
        tokens,
        tokenizer_base: TokenizerBase::new(default_attribute()),
      }
    }
  }

  impl TokenStream for MyTokenizer {
    fn increment_token(&mut self) -> Result<bool> {
      if self.token_upto >= self.tokens.len() {
        return Ok(false);
      }

      let test_token = &self.tokens[self.token_upto];
      self.tokenizer_base.token_stream_base.att.clear_attributes();
      self
        .tokenizer_base
        .token_stream_base
        .att
        .append_str(Some(&test_token.text))?;
      self
        .tokenizer_base
        .token_stream_base
        .att
        .set_offset(test_token.start_offset, test_token.end_offset)?;

      let position_increment = if self.token_upto > 0 {
        test_token.pos - self.tokens[self.token_upto - 1].pos
      } else {
        test_token.pos + 1
      };
      AttributeSource::set_position_increment(
        &mut self.tokenizer_base.token_stream_base.att,
        position_increment,
      )?;

      self.token_upto += 1;
      Ok(true)
    }

    fn end(&mut self) -> Result<()> {
      self.tokenizer_base.end()
    }

    fn reset(&mut self) -> Result<()> {
      self.tokenizer_base.reset()?;
      self.token_upto = 0;
      Ok(())
    }

    fn close(&mut self) -> Result<()> {
      self.tokenizer_base.close()
    }

    fn get_attribute_source(&self) -> &Attributes {
      self.tokenizer_base.get_attribute_source()
    }

    fn get_attribute_source_mut(&mut self) -> &mut Attributes {
      self.tokenizer_base.get_attribute_source_mut()
    }

    fn set_reader(&mut self, input: ReaderEnum) -> Result<()> {
      self.tokenizer_base.set_reader(input)
    }
  }

  impl Tokenizer for MyTokenizer {
    fn get_tokenizer_base_mut(&mut self) -> &mut TokenizerBase {
      &mut self.tokenizer_base
    }

    fn get_tokenizer_base(&self) -> &TokenizerBase {
      &self.tokenizer_base
    }
  }
}
