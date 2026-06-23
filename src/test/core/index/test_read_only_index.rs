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
use crate::core::document::field::Store;
use crate::core::document::field_type::FieldType;
use crate::core::index::directory_reader;
use crate::core::index::stored_fields::StoredFields;
use crate::core::index::term::Term;
use crate::core::search::phrase_query::PhraseQuery;
use crate::core::search::term_query::TermQuery;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::lucene_test_case::{
  create_temp_dir_with_prefix, new_fs_directory, new_index_writer_config_with_analyzer,
  new_searcher_with_reader, new_text_field, random,
};
use std::collections::HashMap;

#[allow(dead_code)] // for quick search
struct TestReadOnlyIndex;

const LONG_TERM: &str = "longtermlongtermlongtermlongtermlongtermlongtermlongtermlongtermlongtermlongtermlongtermlongtermlongtermlongtermlongtermlongtermlongtermlongterm";
const TEXT_PREFIX: &str = "This is the text to be indexed. ";

#[test]
fn test_read_only_index() -> crate::core::util::error::lucene_error::Result<()> {
  let mut random = random();
  let text = format!("{TEXT_PREFIX}{LONG_TERM}");
  let index_path = create_temp_dir_with_prefix("readonlyindex")?;
  let directory = new_fs_directory(&mut random, index_path)?;

  {
    let analyzer = MockAnalyzer::new(&mut random);
    let iwc = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
    // TODO IMPORTANT setCodec未实现
    let iwriter = RandomIndexWriter::with_config(&mut random, directory.clone(), iwc);
    let mut doc = Document::new();
    let mut field_to_type: HashMap<String, FieldType> = HashMap::new();
    doc.add(new_text_field(
      &mut random,
      "fieldname",
      text.clone(),
      Store::Yes,
      &mut field_to_type,
    )?);
    iwriter.add_document(&mut random, doc)?;
    iwriter.close(&mut random)?;
  }

  do_test_read_only_index(directory, &text)
}

fn do_test_read_only_index(
  directory: std::sync::Arc<crate::core::store::directory::DirEnum>,
  text: &str,
) -> crate::core::util::error::lucene_error::Result<()> {
  let ireader = directory_reader::open(directory)?;
  let isearcher = new_searcher_with_reader(ireader)?;

  assert_eq!(
    1,
    isearcher.count(TermQuery::new(Term::from_text("fieldname", LONG_TERM)))?
  );
  let query = TermQuery::new(Term::from_text("fieldname", "text"));
  let hits = isearcher.search(query, 1)?;
  assert_eq!(1, hits.total_hits.value());

  let mut stored_fields = isearcher.stored_fields()?;
  for hit in hits.score_docs {
    let hit_doc = stored_fields.document(hit.doc)?;
    assert_eq!(
      text,
      hit_doc
        .get("fieldname")?
        .expect("fieldname must exist")
        .as_ref()
    );
  }

  let phrase_query = PhraseQuery::from_terms_no_slop("fieldname", &["to", "be"])?;
  assert_eq!(1, isearcher.count(phrase_query)?);

  Ok(())
}
