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
use crate::core::document::date_tools::{DateTools, Resolution};
use crate::core::document::document::Document;
use crate::core::document::field::Store;
use crate::core::document::field_type::FieldType;
use crate::core::document::sorted_doc_values_field::SortedDocValuesField;
use crate::core::index::BytesRef;
use crate::core::index::stored_fields::StoredFields;
use crate::core::index::term::Term;
use crate::core::search::sort::Sort;
use crate::core::search::sort_field::{SortField, SortFieldType};
use crate::core::search::term_query::TermQuery;
use crate::core::search::top_docs::TopDocsLike;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::util::DefaultIndexSearchCR;
use crate::test_framework::core::util::lucene_test_case::{
  is_light_mode, new_directory_shared, new_searcher_with_reader, new_string_field, new_text_field,
  random,
};
use rand_chacha::rand_core::Rng;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

const TEXT_FIELD: &str = "text";
const DATE_TIME_FIELD: &str = "dateTime";
#[allow(dead_code)] // for quick search
pub struct TestDateSort;

static LIGHT_SEARCHER: LazyLock<Arc<DefaultIndexSearchCR>> = LazyLock::new(|| {
  let mut random = random();
  Arc::new(build_set_up(&mut random).expect("failed to initialize TestDateSort"))
});

fn set_up<R>(random: &mut R) -> Result<Arc<DefaultIndexSearchCR>>
where
  R: Rng + ?Sized,
{
  if is_light_mode() {
    return Ok(LIGHT_SEARCHER.clone());
  }

  Ok(Arc::new(build_set_up(random)?))
}

fn build_set_up<R>(random: &mut R) -> Result<DefaultIndexSearchCR>
where
  R: Rng + ?Sized,
{
  let directory = new_directory_shared(random)?;
  let writer = RandomIndexWriter::new(random, directory)?;
  let mut field_to_type = HashMap::new();
  // Add the first document.  text = "Document 1"  dateTime = Oct 10 03:25:22 EDT 2007
  let doc = create_document(random, "Document 1", 1192001122000, &mut field_to_type)?;
  writer.add_document(random, doc)?;
  // Add the second document.  text = "Document 2"  dateTime = Oct 10 03:25:26 EDT 2007
  let doc = create_document(random, "Document 2", 1192001126000, &mut field_to_type)?;
  writer.add_document(random, doc)?;
  // Add the third document.  text = "Document 3"  dateTime = Oct 11 07:12:13 EDT 2007
  let doc = create_document(random, "Document 3", 1192101133000, &mut field_to_type)?;
  writer.add_document(random, doc)?;
  // Add the fourth document.  text = "Document 4"  dateTime = Oct 11 08:02:09 EDT 2007
  let doc = create_document(random, "Document 4", 1192104129000, &mut field_to_type)?;
  writer.add_document(random, doc)?;
  // Add the fifth document.  text = "Document 5"  dateTime = Oct 12 13:25:43 EDT 2007
  let doc = create_document(random, "Document 5", 1192209943000, &mut field_to_type)?;
  writer.add_document(random, doc)?;

  let reader = writer.get_reader(random)?;
  writer.close(random)?;

  new_searcher_with_reader(reader)
}
#[test]
fn test_reverse_date_sort() -> Result<()> {
  let mut random = random();
  let searcher = set_up(&mut random)?;

  let sort = Sort::with_fields(vec![SortField::with_reverse(
    Some(DATE_TIME_FIELD),
    SortFieldType::String,
    true,
  )?])?;

  let query = TermQuery::new(Term::from_text(TEXT_FIELD, "document"));

  let v = searcher.search_with_sort(query, 1000, sort)?;
  let hits = v.score_docs();

  let mut actual_order = Vec::new();
  let mut stored_fields = searcher.stored_fields()?;

  for hit in hits {
    let document = stored_fields.document(hit.doc())?;
    actual_order.push(document.get(TEXT_FIELD)?.unwrap().as_ref().to_string());
  }

  let expected_order = vec![
    "Document 5".to_string(),
    "Document 4".to_string(),
    "Document 3".to_string(),
    "Document 2".to_string(),
    "Document 1".to_string(),
  ];

  assert_eq!(expected_order, actual_order);

  Ok(())
}
fn create_document<R>(
  random: &mut R,
  text: &str,
  time: i64,
  field_to_type: &mut HashMap<String, FieldType>,
) -> Result<Document>
where
  R: Rng + ?Sized,
{
  let mut document = Document::new();

  document.add(new_text_field(
    random,
    TEXT_FIELD,
    text,
    Store::Yes,
    field_to_type,
  )?);

  let date_time_string = DateTools::time_to_string(time, Resolution::SECOND)?;
  document.add(new_string_field(
    random,
    DATE_TIME_FIELD,
    &date_time_string,
    Store::Yes,
    field_to_type,
  )?);

  document.add(SortedDocValuesField::new(
    DATE_TIME_FIELD,
    BytesRef::from_string(&date_time_string),
  ));
  Ok(document)
}
