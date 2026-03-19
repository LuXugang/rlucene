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
use crate::core::document::sorted_set_doc_values_field::SortedSetDocValuesField;
use crate::core::document::string_field::StringField;
use crate::core::index::BytesRef;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::term::Term;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::DefaultIndexSearchCR;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  at_least, new_directory_shared, new_index_writer_config_with_analyzer, new_searcher_with_reader,
};
use crate::test::core::util::test_util::TestUtil;
use rand::{Rng, RngExt};

#[allow(dead_code)] // for quick search
pub struct TestDocValuesRewriteMethod;

fn set_up<R: Rng + ?Sized>(random: &mut R) -> Result<(String, DefaultIndexSearchCR)> {
  let dir = new_directory_shared(random)?;

  let field_name = if random.random_bool(0.5) {
    "field".to_string()
  } else {
    "".to_string()
  };

  // TODO 需要使用MockAnalyzer的另一个构造方法
  let analyzer = MockAnalyzer::new(random);

  let mut iwc = new_index_writer_config_with_analyzer(random, analyzer);
  iwc.set_max_buffered_docs(TestUtil::next_int(random, 50, 1000));

  let writer = RandomIndexWriter::with_config(random, dir.clone(), iwc);

  let mut terms: Vec<String> = Vec::new();

  let num = at_least(random, 200);

  for i in 0..num {
    let mut doc = Document::new();

    doc.add(StringField::from_string("id", i.to_string(), Store::No)?);

    let num_terms = random.random_range(0..4);

    for _ in 0..num_terms {
      let s = TestUtil::random_unicode_string(random);

      doc.add(StringField::from_string(&field_name, s.clone(), Store::No)?);

      doc.add(SortedSetDocValuesField::new(
        &field_name,
        BytesRef::from_string(&s),
      ));

      doc.add(SortedSetDocValuesField::indexed_field(
        &(field_name.clone() + "_with-skip"),
        BytesRef::from_string(&s),
      ));

      terms.push(s);
    }

    writer.add_document(doc)?;
  }

  let num_deletions = random.random_range(0..(num / 10).max(1));

  for _ in 0..num_deletions {
    let id = random.random_range(0..num);
    writer.delete_documents_with_terms(vec![Term::from_text("id", id.to_string())])?;
  }

  let reader = writer.get_reader()?;
  let searcher = new_searcher_with_reader(reader)?;

  writer.close()?;

  Ok((field_name, searcher))
}

#[test]
fn test_regexps() -> Result<()> {
  // TODO DocValuesRewriteMethod 未实现
  Ok(())
}
#[test]
fn test_equals() -> Result<()> {
  // TODO DocValuesRewriteMethod 未实现
  Ok(())
}
