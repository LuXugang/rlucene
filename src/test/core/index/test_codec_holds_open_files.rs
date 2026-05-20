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
use crate::core::document::int_point::IntPoint;
use crate::core::document::numeric_doc_values_field::NumericDocValuesField;
use crate::core::index::composite_reader::get_context;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  at_least, new_directory_shared, new_text_field, random,
};
use crate::test::core::util::test_util::TestUtil;
use std::collections::HashMap;

#[allow(dead_code)] // for quick search
struct TestCodecHoldsOpenFiles;
#[test]
fn test() -> Result<()> {
  let mut random = random();

  let d = new_directory_shared(&mut random)?;
  // TODO IMPORTANT
  // d.set_check_index_on_close(false);

  let w = RandomIndexWriter::new(&mut random, d.clone());
  let num_docs = at_least(&mut random, 100);
  let mut field_to_type = HashMap::new();

  for i in 0..num_docs {
    let mut doc = Document::new();
    doc.add(new_text_field(
      &mut random,
      "foo",
      "bar",
      Store::No,
      &mut field_to_type,
    )?);
    doc.add(IntPoint::new("doc", vec![i])?);
    doc.add(IntPoint::new("doc2d", vec![i, i])?);
    doc.add(NumericDocValuesField::new("dv", i as i64));
    w.add_document(doc)?;
  }

  let r = w.get_reader()?;
  w.commit()?;
  w.close()?;

  for name in d.list_all()? {
    d.delete_file(&name)?;
  }

  let ctx = get_context(r)?;
  for cxt in ctx.leaves()? {
    TestUtil::check_reader(cxt.reader())?;
  }

  Ok(())
}
