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
use crate::core::index::field_infos::FieldInfos;
use crate::core::index::index_writer_config::IndexWriterConfig;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::segment_info::SegmentInfo;
use crate::core::index::segment_write_state::SegmentWriteState;
use crate::core::index::sorter::DocMapImpl;
use crate::core::index::stored_fields_consumer::{StoredFieldsConsumer, StoredFieldsConsumerHook};
use crate::core::store::IOContext;
use crate::core::store::directory::DirEnum;
use crate::core::store::flush_info::FlushInfo;
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::info_stream::get_default_info_stream;
use crate::core::util::{LATEST, StringHelper};
use crate::test_framework::core::index::test_stored_fields_consumer::TestStoredFieldsConsumerHook;
use crate::test_framework::core::util::lucene_test_case::{new_directory_shared, random};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};

#[test]
fn test_finish() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let iwc = IndexWriterConfig::<Arc<DirEnum>>::new()?;
  let codec = iwc.get_codec().clone();
  let mut si = SegmentInfo::new(
    Arc::clone(&dir),
    Some((*LATEST).clone()),
    None,
    "_0",
    -1,
    false,
    false,
    Some(codec.clone()),
    HashMap::new(),
    StringHelper::random_id(),
    HashMap::new(),
    None,
  )?;

  let start_doc_counter = Arc::new(AtomicI32::new(0));
  let finish_doc_counter = Arc::new(AtomicI32::new(0));
  let hook = TestStoredFieldsConsumerHook::new(
    Arc::clone(&start_doc_counter),
    Arc::clone(&finish_doc_counter),
  );
  let mut consumer = StoredFieldsConsumer::new(
    codec,
    Arc::clone(&dir),
    StoredFieldsConsumerHook::TestStoredFieldsConsumer(hook),
  );

  let num_docs = 3;
  consumer.finish(num_docs, &mut si)?;

  si.set_max_doc(num_docs)?;
  let field_infos = Arc::new(FieldInfos::new(Vec::new())?);
  let io_context = IOContext::with_flush(FlushInfo::new(num_docs, 10))?;
  let state = SegmentWriteState::new(get_default_info_stream(), &dir, field_infos, &io_context);
  consumer.flush::<DocMapImpl, _>(&state, None, &mut si)?;
  dir.close()?;

  assert_eq!(num_docs, start_doc_counter.load(Ordering::SeqCst));
  assert_eq!(num_docs, finish_doc_counter.load(Ordering::SeqCst));
  Ok(())
}
