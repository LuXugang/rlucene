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
use crate::core::analysis::analyzer::AnalyzerEnum;
use crate::core::document::document::Document;
use crate::core::document::field::Field;
use crate::core::document::field::Store::{No, Yes};
use crate::core::document::field_type::FieldType;
use crate::core::document::fields::Fields;
use crate::core::document::stored_field::StoredField;
use crate::core::document::string_field::{StringField, string_field_type};
use crate::core::document::text_field::{TextField, text_field_type};
use crate::core::index::index_options::IndexOptions;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::index_writer_config::IndexWriterConfig;
use crate::core::index::indexable_field::IndexableField;
use crate::core::index::indexable_field_type::IndexableFieldType;
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::search::index_searcher::get_default_similarity;
use crate::core::search::similarities_impl::similarities::SimilarityEnum;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::mock_analyzer;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use rand::Rng;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::LazyLock;

pub static FIELD_1_TEXT: &str = "field one text";
pub static TEXT_FIELD_1_KEY: &str = "textField1";

pub static FIELD_2_TEXT: &str = "field field field two text";
pub static FIELD_2_FREQS: [i32; 3] = [3, 1, 1];
pub static TEXT_FIELD_2_KEY: &str = "textField2";

pub static CUSTOM_TYPE: LazyLock<FieldType> = LazyLock::new(|| {
  let mut ft = FieldType::from_ref(&*text_field_type::TYPE_STORED).expect("should not fail");
  ft.freeze();
  ft
});

pub static TEXT_FIELD_1: LazyLock<Field> =
  LazyLock::new(|| Field::new(TEXT_FIELD_1_KEY, FIELD_1_TEXT, CUSTOM_TYPE.clone()));

pub static TEXT_TYPE_STORED_WITH_TVS: LazyLock<FieldType> = LazyLock::new(|| {
  let mut ft = FieldType::from_ref(&*text_field_type::TYPE_STORED).expect("should not fail");
  ft.set_store_term_vectors(true).expect("should not fail");
  ft.set_store_term_vector_positions(true)
    .expect("should not fail");
  ft.set_store_term_vector_offsets(true)
    .expect("should not fail");
  ft.freeze();
  ft
});

pub static TEXT_FIELD_2: LazyLock<Field> = LazyLock::new(|| {
  Field::new(
    TEXT_FIELD_2_KEY,
    FIELD_2_TEXT,
    TEXT_TYPE_STORED_WITH_TVS.clone(),
  )
});

pub static FIELD_3_TEXT: &str = "aaaNoNorms aaaNoNorms bbbNoNorms";
pub static TEXT_FIELD_3_KEY: &str = "textField3";

pub static CUSTOM_TYPE3: LazyLock<FieldType> = LazyLock::new(|| {
  let mut ft = FieldType::from_ref(&*text_field_type::TYPE_STORED).expect("should not fail");
  ft.set_omit_norms(true).expect("should not fail");
  ft.freeze();
  ft
});

pub static TEXT_FIELD_3: LazyLock<Field> =
  LazyLock::new(|| Field::new(TEXT_FIELD_3_KEY, FIELD_3_TEXT, CUSTOM_TYPE3.clone()));

pub static KEYWORD_TEXT: &str = "Keyword";
pub static KEYWORD_FIELD_KEY: &str = "keyField";

pub static KEY_FIELD: LazyLock<StringField> = LazyLock::new(|| {
  StringField::from_string(KEYWORD_FIELD_KEY, KEYWORD_TEXT, Yes).expect("should not fail")
});

pub static NO_NORMS_TEXT: &str = "omitNormsText";
pub static NO_NORMS_KEY: &str = "omitNorms";

pub static CUSTOM_TYPE5: LazyLock<FieldType> = LazyLock::new(|| {
  let mut ft = FieldType::from_ref(&*text_field_type::TYPE_STORED).expect("should not fail");
  ft.set_omit_norms(true).expect("should not fail");
  ft.set_tokenized(false).expect("should not fail");
  ft.freeze();
  ft
});

pub static NO_NORMS_FIELD: LazyLock<Field> =
  LazyLock::new(|| Field::new(NO_NORMS_KEY, NO_NORMS_TEXT, CUSTOM_TYPE5.clone()));

pub static NO_TF_TEXT: &str = "analyzed with no tf and positions";
pub static NO_TF_KEY: &str = "omitTermFreqAndPositions";

pub static CUSTOM_TYPE6: LazyLock<FieldType> = LazyLock::new(|| {
  let mut ft = FieldType::from_ref(&*text_field_type::TYPE_STORED).expect("should not fail");
  ft.set_index_options(IndexOptions::Docs)
    .expect("should not fail");
  ft.freeze();
  ft
});

pub static NO_TF_FIELD: LazyLock<Field> =
  LazyLock::new(|| Field::new(NO_TF_KEY, NO_TF_TEXT, CUSTOM_TYPE6.clone()));

pub static UNINDEXED_FIELD_TEXT: &str = "unindexed field text";
pub static UNINDEXED_FIELD_KEY: &str = "unIndField";

pub static CUSTOM_TYPE7: LazyLock<FieldType> = LazyLock::new(|| {
  let mut ft = FieldType::new();
  ft.set_stored(true).expect("should not fail");
  ft.freeze();
  ft
});

pub static UNINDEXED_FIELD: LazyLock<Field> = LazyLock::new(|| {
  Field::new(
    UNINDEXED_FIELD_KEY,
    UNINDEXED_FIELD_TEXT,
    CUSTOM_TYPE7.clone(),
  )
});

pub static STRING_TYPE_STORED_WITH_TVS: LazyLock<FieldType> = LazyLock::new(|| {
  let mut ft = FieldType::from_ref(&*string_field_type::TYPE_STORED).expect("should not fail");
  ft.set_store_term_vectors(true).expect("should not fail");
  ft.set_store_term_vector_positions(true)
    .expect("should not fail");
  ft.set_store_term_vector_offsets(true)
    .expect("should not fail");
  ft.freeze();
  ft
});

pub static UNSTORED_1_FIELD_TEXT: &str = "unstored field text";
pub static UNSTORED_FIELD_1_KEY: &str = "unStoredField1";

pub static UNSTORED_FIELD_1: LazyLock<TextField> = LazyLock::new(|| {
  TextField::from_string(UNSTORED_FIELD_1_KEY, UNSTORED_1_FIELD_TEXT, No).expect("should not fail")
});

pub static UNSTORED_2_FIELD_TEXT: &str = "unstored field text";
pub static UNSTORED_FIELD_2_KEY: &str = "unStoredField2";

pub static CUSTOM_TYPE8: LazyLock<FieldType> = LazyLock::new(|| {
  let mut ft = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED).expect("should not fail");
  ft.set_store_term_vectors(true).expect("should not fail");
  ft.freeze();
  ft
});

pub static UNSTORED_FIELD_2: LazyLock<Field> = LazyLock::new(|| {
  Field::new(
    UNSTORED_FIELD_2_KEY,
    UNSTORED_2_FIELD_TEXT,
    CUSTOM_TYPE8.clone(),
  )
});

pub static LAZY_FIELD_BINARY_KEY: &str = "lazyFieldBinary";
pub static LAZY_FIELD_BINARY_BYTES: LazyLock<Vec<u8>> =
  LazyLock::new(|| "These are some binary field bytes".as_bytes().to_vec());

pub static LAZY_FIELD_BINARY: LazyLock<StoredField> = LazyLock::new(|| {
  StoredField::from_binary(LAZY_FIELD_BINARY_KEY, LAZY_FIELD_BINARY_BYTES.clone())
    .expect("should not fail")
});

pub static LAZY_FIELD_KEY: &str = "lazyField";
pub static LAZY_FIELD_TEXT: &str = "These are some field bytes";

pub static LAZY_FIELD: LazyLock<Field> =
  LazyLock::new(|| Field::new(LAZY_FIELD_KEY, LAZY_FIELD_TEXT, CUSTOM_TYPE.clone()));

pub static LARGE_LAZY_FIELD_KEY: &str = "largeLazyField";

pub static LARGE_LAZY_FIELD_TEXT: LazyLock<String> = LazyLock::new(|| {
  let mut s = String::with_capacity(10000 * 55);
  for _ in 0..10000 {
    s.push_str("Lazily loading lengths of language in lieu of laughing ");
  }
  s
});

pub static LARGE_LAZY_FIELD: LazyLock<Field> = LazyLock::new(|| {
  Field::new(
    LARGE_LAZY_FIELD_KEY,
    LARGE_LAZY_FIELD_TEXT.clone(),
    CUSTOM_TYPE.clone(),
  )
});

pub static FIELD_UTF1_TEXT: &str = "field one 一text";
pub static TEXT_FIELD_UTF1_KEY: &str = "textField1Utf8";

pub static TEXT_UTF_FIELD_1: LazyLock<Field> =
  LazyLock::new(|| Field::new(TEXT_FIELD_UTF1_KEY, FIELD_UTF1_TEXT, CUSTOM_TYPE.clone()));

pub static FIELD_UTF2_TEXT: &str = "field field field 一two text";
pub static FIELD_UTF2_FREQS: [i32; 3] = [3, 1, 1];
pub static TEXT_FIELD_UTF2_KEY: &str = "textField2Utf8";

pub static TEXT_UTF_FIELD_2: LazyLock<Field> = LazyLock::new(|| {
  Field::new(
    TEXT_FIELD_UTF2_KEY,
    FIELD_UTF2_TEXT,
    TEXT_TYPE_STORED_WITH_TVS.clone(),
  )
});
#[derive(Clone, Debug)]
pub enum NameValue {
  Str(&'static str),
  Bytes(Vec<u8>),
  String(String),
}
pub static NAME_VALUES: LazyLock<HashMap<String, NameValue>> = LazyLock::new(|| {
  let mut m = HashMap::new();

  m.insert(TEXT_FIELD_1_KEY.to_string(), NameValue::Str(FIELD_1_TEXT));
  m.insert(TEXT_FIELD_2_KEY.to_string(), NameValue::Str(FIELD_2_TEXT));
  m.insert(TEXT_FIELD_3_KEY.to_string(), NameValue::Str(FIELD_3_TEXT));
  m.insert(KEYWORD_FIELD_KEY.to_string(), NameValue::Str(KEYWORD_TEXT));
  m.insert(NO_NORMS_KEY.to_string(), NameValue::Str(NO_NORMS_TEXT));
  m.insert(NO_TF_KEY.to_string(), NameValue::Str(NO_TF_TEXT));
  m.insert(
    UNINDEXED_FIELD_KEY.to_string(),
    NameValue::Str(UNINDEXED_FIELD_TEXT),
  );
  m.insert(
    UNSTORED_FIELD_1_KEY.to_string(),
    NameValue::Str(UNSTORED_1_FIELD_TEXT),
  );
  m.insert(
    UNSTORED_FIELD_2_KEY.to_string(),
    NameValue::Str(UNSTORED_2_FIELD_TEXT),
  );
  m.insert(LAZY_FIELD_KEY.to_string(), NameValue::Str(LAZY_FIELD_TEXT));
  m.insert(
    LAZY_FIELD_BINARY_KEY.to_string(),
    NameValue::Bytes(LAZY_FIELD_BINARY_BYTES.clone()),
  );
  m.insert(
    LARGE_LAZY_FIELD_KEY.to_string(),
    NameValue::String(LARGE_LAZY_FIELD_TEXT.clone()),
  );
  m.insert(
    TEXT_FIELD_UTF1_KEY.to_string(),
    NameValue::Str(FIELD_UTF1_TEXT),
  );
  m.insert(
    TEXT_FIELD_UTF2_KEY.to_string(),
    NameValue::Str(FIELD_UTF2_TEXT),
  );

  m
});
pub static FIELDS: LazyLock<Vec<Fields>> = LazyLock::new(|| {
  vec![
    TEXT_FIELD_1.clone().into(),
    TEXT_FIELD_2.clone().into(),
    TEXT_FIELD_3.clone().into(),
    KEY_FIELD.clone().into(),
    NO_NORMS_FIELD.clone().into(),
    NO_TF_FIELD.clone().into(),
    UNINDEXED_FIELD.clone().into(),
    UNSTORED_FIELD_1.clone().into(),
    UNSTORED_FIELD_2.clone().into(),
    TEXT_UTF_FIELD_1.clone().into(),
    TEXT_UTF_FIELD_2.clone().into(),
    LAZY_FIELD.clone().into(),
    LAZY_FIELD_BINARY.clone().into(),
    LARGE_LAZY_FIELD.clone().into(),
  ]
});
pub static DATA: LazyLock<Data> = LazyLock::new(|| {
  let mut data = Data::default();

  for f in FIELDS.iter() {
    let f = f.clone();
    add(&mut data.all, f.clone());
    let ft = f.field_type();
    if *ft.index_options() != IndexOptions::None {
      add(&mut data.indexed, f.clone());
    } else {
      add(&mut data.unindexed, f.clone());
    }
    if ft.store_term_vectors() {
      add(&mut data.term_vector, f.clone());
    }
    if *ft.index_options() != IndexOptions::None && !ft.store_term_vectors() {
      add(&mut data.no_term_vector, f.clone());
    }
    if ft.stored() {
      add(&mut data.stored, f.clone());
    } else {
      add(&mut data.unstored, f.clone());
    }
    if *ft.index_options() == IndexOptions::Docs {
      add(&mut data.no_tf, f.clone());
    }
    if ft.omit_norms() {
      add(&mut data.no_norms, f.clone());
    }
    if *ft.index_options() == IndexOptions::Docs {
      add(&mut data.no_tf, f.clone());
    }
  }

  data
});
#[derive(Default)]
pub struct Data {
  pub(crate) all: HashMap<String, Fields>,
  pub(crate) indexed: HashMap<String, Fields>,
  pub(crate) stored: HashMap<String, Fields>,
  pub(crate) unstored: HashMap<String, Fields>,
  pub(crate) unindexed: HashMap<String, Fields>,
  pub(crate) term_vector: HashMap<String, Fields>,
  pub(crate) no_term_vector: HashMap<String, Fields>,
  pub(crate) lazy: HashMap<String, Fields>,
  pub(crate) no_norms: HashMap<String, Fields>,
  pub(crate) no_tf: HashMap<String, Fields>,
}

fn add(map: &mut HashMap<String, Fields>, f: Fields) {
  let name = f.name().to_string();
  map.insert(name, f);
}
/// Helper functions for tests that handles documents
pub struct DocHelper;
impl DocHelper {
  /// Adds the fields above to a document
  pub fn setup_doc(doc: &mut Document) {
    for f in FIELDS.iter() {
      doc.add(f.clone());
    }
  }
  pub fn write_doc<D, R>(random: &mut R, dir: Arc<D>, doc: Document) -> Result<SegmentCommitInfo<D>>
  where
    D: Directory,
    R: Rng + ?Sized,
  {
    let mock = MockAnalyzer::with_automaton(random, mock_analyzer::WHITESPACE.clone(), false);
    Self::write_doc_with_analyzer(random, dir, mock, None::<SimilarityEnum>, doc)
  }
  pub fn write_doc_with_analyzer<D, R, A, S>(
    _random: &mut R,
    dir: Arc<D>,
    analyzer: A,
    similarity: Option<S>,
    doc: Document,
  ) -> Result<SegmentCommitInfo<D>>
  where
    D: Directory,
    R: Rng + ?Sized,
    A: Into<AnalyzerEnum>,
    S: Into<SimilarityEnum>,
  {
    let mut config = IndexWriterConfig::with_analyzer(analyzer);
    let s = match similarity {
      Some(v) => v.into(),
      None => get_default_similarity(),
    };
    config.set_similarity(s);

    let writer = IndexWriter::new(dir.clone(), config)?;
    writer.add_document(doc)?;
    writer.commit()?;
    writer.close()?;
    let inner = writer.inner.lock();
    let last = inner.segment_infos.segments.last().unwrap().clone();
    Ok(last)
  }
  pub(crate) fn num_fields(doc: &Document) -> usize {
    doc.get_fields().len()
  }
  pub fn create_document(n: i32, index_name: &str, num_fields: usize) -> Document {
    let mut doc = Document::new();

    doc.add(Field::new(
      "id",
      n.to_string(),
      STRING_TYPE_STORED_WITH_TVS.clone(),
    ));

    doc.add(Field::new(
      "indexname",
      index_name,
      STRING_TYPE_STORED_WITH_TVS.clone(),
    ));

    let mut sb = format!("a{}", n);
    doc.add(Field::new(
      "field1",
      sb.clone(),
      TEXT_TYPE_STORED_WITH_TVS.clone(),
    ));

    sb.push_str(&format!(" b{}", n));

    for i in 1..num_fields {
      let field_name = format!("field{}", i + 1);
      doc.add(Field::new(
        &field_name,
        sb.clone(),
        TEXT_TYPE_STORED_WITH_TVS.clone(),
      ));
    }

    doc
  }
}
