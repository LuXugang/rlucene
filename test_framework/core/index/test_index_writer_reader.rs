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
use crate::core::index::BytesRef;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::multi_bits::get_live_docs;
use crate::core::index::term::Term;
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, NO_MORE_DOCS};
use crate::core::store::directory::Directory;
use crate::core::util::bits::Bits;
use crate::test_framework::core::index::doc_helper::DocHelper;
use crate::test_framework::core::util::test_util::TestUtil;

#[allow(dead_code)] // for quick search
struct TestIndexWriterReader;

pub(crate) fn count<R, IR>(
  random: &mut R,
  t: &Term,
  r: IR,
) -> crate::core::util::error::lucene_error::Result<i32>
where
  R: rand::Rng + ?Sized,
  IR: IndexReader + Clone,
{
  let mut count = 0;
  let term_bytes = BytesRef::from_string(&t.text()?);
  let mut td = TestUtil::docs_with_reader(random, r.clone(), t.field(), &term_bytes, None, 0)?;

  if let Some(td) = td.as_mut() {
    let live_docs = get_live_docs(r)?;
    while td.next_doc()? != NO_MORE_DOCS {
      let doc_id = td.doc_id();
      if live_docs
        .as_ref()
        .is_none_or(|bits| bits.get(doc_id as usize).expect(""))
      {
        count += 1;
      }
    }
  }

  Ok(count)
}
pub(crate) fn create_index_no_close<D>(
  multi_segment: bool,
  index_name: &str,
  w: &IndexWriter<D>,
) -> crate::core::util::error::lucene_error::Result<()>
where
  D: Directory + 'static,
{
  for i in 0..100 {
    w.add_document(DocHelper::create_document(i, index_name, 4))?;
  }
  if !multi_segment {
    w.force_merge(1)?;
  }
  Ok(())
}
