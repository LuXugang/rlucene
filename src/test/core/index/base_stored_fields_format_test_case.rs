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
use crate::core::document::document::Document;
use crate::core::document::field::{Field, Store};
use crate::core::document::field_type::FieldType;
use crate::core::document::int_point::IntPoint;
use crate::core::document::numeric_doc_values_field::NumericDocValuesField;
use crate::core::document::stored_field::StoredField;
use crate::core::document::string_field::StringField;
use crate::core::index::composite_reader::get_context;
use crate::core::index::directory_reader::directory_reader_util;
use crate::core::index::index_options::IndexOptions;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::indexable_field::IndexableField;
use crate::core::index::indexable_field_type::IndexableFieldType;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::index::stored_fields::StoredFields;
use crate::core::index::term::Term;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::term_query::TermQuery;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::number::Number;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::index::base_index_file_format_test_case::BaseIndexFileFormatTestCase;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  at_least, new_directory_shared, new_field, new_index_writer_config,
  new_index_writer_config_with_analyzer, new_searcher_with_reader, new_string_field,
};
use crate::test::core::util::test_util::TestUtil;
use rand::Rng;
use rand::RngExt;
use rand::seq::SliceRandom;
use std::collections::{HashMap, HashSet};

/// Base class aiming at testing [`StoredFieldsFormat`] stored fields formats. To test a new
/// format, all you need is to register a new [`Codec`] which uses it and extend this class and
/// override [`Self::get_codec`].
///
/// @lucene.experimental
pub trait BaseStoredFieldsFormatTestCase: BaseIndexFileFormatTestCase {
  fn test_random_stored_fields<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
    let directory = new_directory_shared(random)?;
    let analyzer = MockAnalyzer::new(random);
    let mut iwc = new_index_writer_config_with_analyzer(random, analyzer);
    iwc.set_max_buffered_docs(TestUtil::next_int(random, 5, 20));
    let writer = RandomIndexWriter::with_config(random, directory, iwc);

    let doc_count = at_least(random, 200);
    let field_count = TestUtil::next_int(random, 1, 5);
    let mut field_ids: Vec<i32> = (0..field_count).collect();
    let mut field_types = HashMap::new();
    let mut docs = HashMap::<String, Document>::new();

    let mut stored_only = FieldType::new();
    stored_only.set_stored(true)?;
    stored_only.freeze();

    for i in 0..doc_count {
      let id = i.to_string();
      let mut doc = Document::new();
      doc.add(new_string_field(
        random,
        "id",
        id.clone(),
        Store::No,
        &mut field_types,
      )?);

      for field in &field_ids {
        if random.random_range(0..4) != 3 {
          let value = TestUtil::random_unicode_string_with_len(random, 1000);
          doc.add(new_field(
            random,
            format!("f{field}"),
            value,
            &stored_only,
            &mut field_types,
          )?);
        }
      }

      docs.insert(id.clone(), doc.clone());
      writer.add_document(doc)?;

      if random.random_range(0..50) == 17 {
        field_ids.shuffle(random);
      }
      if random.random_range(0..5) == 3 && i > 0 {
        let del_id = random.random_range(0..i).to_string();
        writer.delete_documents_with_terms(vec![Term::from_text("id", del_id.clone())])?;
        docs.remove(&del_id);
      }
    }

    if !docs.is_empty() {
      let ids_list = docs.keys().cloned().collect::<Vec<_>>();
      for _ in 0..2 {
        let reader = self
          .maybe_wrap_with_merging_reader(directory_reader_util::open_from_writer(&writer.w)?)?;
        let searcher = new_searcher_with_reader(reader)?;
        let mut stored_fields = searcher.stored_fields()?;

        for _ in 0..at_least(random, 100) {
          let test_id = ids_list[random.random_range(0..ids_list.len())].clone();
          let hits = searcher.search(TermQuery::new(Term::from_text("id", test_id.clone())), 1)?;
          assert_eq!(1, hits.total_hits.value());

          let doc = stored_fields.document(hits.score_docs[0].doc)?;
          let expected = docs.get(&test_id).unwrap();
          for i in 0..field_count {
            assert_eq!(
              expected.get(&format!("f{i}"))?.map(|v| v.into_owned()),
              doc.get(&format!("f{i}"))?.map(|v| v.into_owned()),
              "doc {test_id}, field f{i} is wrong",
            );
          }
        }
        writer.force_merge(1)?;
      }
    }

    writer.close()?;
    Ok(())
  }

  fn test_stored_fields_order<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
    let directory = new_directory_shared(random)?;
    let iwc = new_index_writer_config(random);
    let writer = RandomIndexWriter::with_config(random, directory, iwc);

    let mut stored_only = FieldType::new();
    stored_only.set_stored(true)?;
    stored_only.freeze();

    let mut doc = Document::new();
    doc.add(Field::new("zzz", "a b c", stored_only.clone()));
    doc.add(Field::new("aaa", "a b c", stored_only.clone()));
    doc.add(Field::new("zzz", "1 2 3", stored_only));
    writer.add_document(doc)?;

    let reader = self.maybe_wrap_with_merging_reader(writer.get_reader()?)?;
    let doc = reader.stored_fields()?.document(0)?;
    let fields = doc.get_fields();

    assert_eq!(3, fields.len());
    assert_eq!("zzz", fields[0].name());
    assert_eq!(
      Some("a b c"),
      fields[0].string_value()?.as_deref().map(|s| s.as_str())
    );
    assert_eq!("aaa", fields[1].name());
    assert_eq!(
      Some("a b c"),
      fields[1].string_value()?.as_deref().map(|s| s.as_str())
    );
    assert_eq!("zzz", fields[2].name());
    assert_eq!(
      Some("1 2 3"),
      fields[2].string_value()?.as_deref().map(|s| s.as_str())
    );

    reader.close()?;
    writer.close()?;
    Ok(())
  }

  fn test_binary_field_offset_length<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
    let directory = new_directory_shared(random)?;
    let iwc = new_index_writer_config(random);
    let writer = RandomIndexWriter::with_config(random, directory, iwc);

    let mut bytes = vec![0u8; 50];
    for (i, b) in bytes.iter_mut().enumerate() {
      *b = (i as u8) + 77;
    }

    let field = StoredField::from_binary_with_range("binary", bytes, 10, 17)?;
    let binary = field.binary_value()?.unwrap();
    assert_eq!(50, binary.bytes.len());
    assert_eq!(10, binary.offset);
    assert_eq!(17, binary.length);

    let mut doc = Document::new();
    doc.add(field);
    writer.add_document(doc)?;

    let reader = self.maybe_wrap_with_merging_reader(writer.get_reader()?)?;
    let doc = reader.stored_fields()?.document(0)?;
    let field = doc.get_field("binary").unwrap();
    let binary = field.binary_value()?.unwrap();
    assert_eq!(17, binary.length);
    assert_eq!(87, binary.bytes[binary.offset]);

    reader.close()?;
    writer.close()?;
    Ok(())
  }

  fn test_numeric_field<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
    let directory = new_directory_shared(random)?;
    let writer = RandomIndexWriter::new(random, directory);
    let num_docs = at_least(random, 500) as usize;
    let mut answers = vec![Number::I32(0); num_docs];
    let mut type_answers = vec![""; num_docs];

    for id in 0..num_docs {
      let (nf, answer, type_answer) = if random.random_bool(0.5) {
        if random.random_bool(0.5) {
          let value = random.random::<f32>();
          (
            StoredField::from_f32("nf", value)?,
            Number::F32(value),
            "f32",
          )
        } else {
          let value = random.random::<f64>();
          (
            StoredField::from_f64("nf", value)?,
            Number::F64(value),
            "f64",
          )
        }
      } else if random.random_bool(0.5) {
        let value = random.random::<i32>();
        (
          StoredField::from_i32("nf", value)?,
          Number::I32(value),
          "i32",
        )
      } else {
        let value = random.random::<i64>();
        (
          StoredField::from_i64("nf", value)?,
          Number::I64(value),
          "i64",
        )
      };

      let mut doc = Document::new();
      doc.add(nf);
      doc.add(StoredField::from_i32("id", id as i32)?);
      doc.add(IntPoint::new("id", [id as i32])?);
      doc.add(NumericDocValuesField::new("id", id as i64));
      answers[id] = answer;
      type_answers[id] = type_answer;
      writer.add_document(doc)?;
    }

    let reader =
      self.maybe_wrap_with_merging_reader(directory_reader_util::open_from_writer(&writer.w)?)?;
    writer.close()?;
    assert_eq!(num_docs as i32, reader.num_docs()?);

    for leaf in get_context(reader)?.leaves()? {
      let sub = leaf.reader().clone();
      let mut ids = sub.get_numeric_doc_values("id")?.unwrap();
      let mut stored_fields = sub.stored_fields()?;
      for doc_id in 0..sub.num_docs()? {
        let doc = stored_fields.document(doc_id)?;
        let field = doc.get_field("nf").unwrap();
        assert_eq!(doc_id, ids.next_doc()?);
        let idx = ids.long_value()? as usize;
        let actual = field.numeric_value()?.unwrap();
        assert_eq!(answers[idx], actual);
        let actual_type = match actual {
          Number::F32(_) => "f32",
          Number::F64(_) => "f64",
          Number::I32(_) => "i32",
          Number::I64(_) => "i64",
          Number::U8(_) => "u8",
          Number::I16(_) => "i16",
        };
        assert_eq!(type_answers[idx], actual_type);
      }
    }

    Ok(())
  }

  fn test_indexed_bit<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
    let directory = new_directory_shared(random)?;
    let writer = RandomIndexWriter::new(random, directory);

    let mut only_stored = FieldType::new();
    only_stored.set_stored(true)?;
    only_stored.freeze();

    let mut doc = Document::new();
    doc.add(Field::new("field", "value", only_stored));
    doc.add(StringField::from_string("field2", "value", Store::Yes)?);
    writer.add_document(doc)?;

    let reader = self.maybe_wrap_with_merging_reader(writer.get_reader()?)?;
    let doc = reader.stored_fields()?.document(0)?;
    assert_eq!(
      IndexOptions::None,
      *doc.get_field("field").unwrap().field_type().index_options()
    );
    assert_ne!(
      IndexOptions::None,
      *doc
        .get_field("field2")
        .unwrap()
        .field_type()
        .index_options()
    );

    reader.close()?;
    writer.close()?;
    Ok(())
  }

  fn test_read_skip<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
    let directory = new_directory_shared(random)?;
    let analyzer = MockAnalyzer::new(random);
    let mut iwc = new_index_writer_config_with_analyzer(random, analyzer);
    iwc.set_max_buffered_docs(TestUtil::next_int(random, 2, 30));
    let writer = RandomIndexWriter::with_config(random, directory, iwc);

    let mut ft = FieldType::new();
    ft.set_stored(true)?;
    ft.freeze();

    let string = TestUtil::random_simple_string_with_len(random, 50);
    let bytes = string.as_bytes().to_vec();
    let long_value = if random.random_bool(0.5) {
      random.random_range(0..42) as i64
    } else {
      random.random::<i64>()
    };
    let int_value = if random.random_bool(0.5) {
      random.random_range(0..42)
    } else {
      random.random::<i32>()
    };
    let float_value = random.random::<f32>();
    let double_value = random.random::<f64>();

    for _ in 0..100 {
      let mut doc = Document::new();
      doc.add(Field::from_binary("bytes", bytes.clone(), ft.clone())?);
      doc.add(Field::new("string", string.clone(), ft.clone()));
      doc.add(StoredField::from_i64("long", long_value)?);
      doc.add(StoredField::from_i32("int", int_value)?);
      doc.add(StoredField::from_f32("float", float_value)?);
      doc.add(StoredField::from_f64("double", double_value)?);
      writer.add_document(doc)?;
    }
    writer.commit()?;

    let reader = self.maybe_wrap_with_merging_reader(writer.get_reader()?)?;
    let mut stored_fields = reader.stored_fields()?;
    let doc_id = random.random_range(0..100);

    for field_name in ["bytes", "string", "long", "int", "float", "double"] {
      let mut fields = HashSet::new();
      fields.insert(field_name.to_string());
      let doc = stored_fields.document_with_fields(doc_id, &fields)?;
      let field = doc.get_field(field_name).unwrap();
      match field_name {
        "bytes" => {
          let binary = field.binary_value()?.unwrap();
          let actual = binary.bytes[binary.offset..binary.offset + binary.length].to_vec();
          assert_eq!(bytes, actual);
        },
        "string" => {
          assert_eq!(
            Some(string.as_str()),
            field.string_value()?.as_deref().map(|s| s.as_str())
          );
        },
        "long" => assert_eq!(Some(Number::I64(long_value)), field.numeric_value()?),
        "int" => assert_eq!(Some(Number::I32(int_value)), field.numeric_value()?),
        "float" => assert_eq!(Some(Number::F32(float_value)), field.numeric_value()?),
        "double" => assert_eq!(Some(Number::F64(double_value)), field.numeric_value()?),
        _ => unreachable!(),
      }
    }

    reader.close()?;
    writer.close()?;
    Ok(())
  }

  fn test_empty_docs<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
    let directory = new_directory_shared(random)?;
    let analyzer = MockAnalyzer::new(random);
    let mut iwc = new_index_writer_config_with_analyzer(random, analyzer);
    iwc.set_max_buffered_docs(TestUtil::next_int(random, 2, 30));
    let writer = RandomIndexWriter::with_config(random, directory, iwc);

    let num_docs = if random.random_bool(0.5) {
      1
    } else {
      at_least(random, 1000)
    };

    for _ in 0..num_docs {
      writer.add_document(Document::new())?;
    }
    writer.commit()?;

    let reader = self.maybe_wrap_with_merging_reader(writer.get_reader()?)?;
    let mut stored_fields = reader.stored_fields()?;
    for i in 0..num_docs {
      let doc = stored_fields.document(i)?;
      assert!(doc.get_fields().is_empty());
    }

    reader.close()?;
    writer.close()?;
    Ok(())
  }
  fn test_concurrent_reads<R: Rng + ?Sized>(&self, _random: &mut R) -> Result<()> {
    // TODO 多线程未实现
    Ok(())
  }
  // TODO  还有其他测试未实现
}
