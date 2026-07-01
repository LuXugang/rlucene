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
use crate::core::document::string_field::StringField;
use crate::core::index::composite_reader::get_context;
use crate::core::index::composite_reader_context::CompositeReaderContext;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::index_writer_config::IndexWriterConfig;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::segment_reader::DefaultLeafReader;
use crate::core::index::standard_directory_reader::StandardDirectoryReaderType;
use crate::core::search::index_searcher::{DefaultIndexSearcher, IndexSearcher};
use crate::core::store::directory::DirEnum;
use crate::core::store::nio_fs_directory::NIOFSDirectory;
use crate::core::store::{FSDirectory, NativeFSLockFactory};
use crate::core::util::error::lucene_error::Result;
use std::sync::Arc;
use tempfile::TempDir;

pub mod automaton;
pub mod bkd;
pub mod english;
pub mod index_package_access;
pub mod line_file_docs;
pub mod lucene_test_case;
pub mod null_info_stream;
pub mod test_util;
pub mod test_vector_util;
pub mod throttled_index_output;

pub type DefaultCRReaderShared = Arc<StandardDirectoryReaderType<DirEnum>>;
pub type DefaultCRReader = StandardDirectoryReaderType<DirEnum>;
pub type DefaultLRReader = DefaultLeafReader<DirEnum>;
pub type DefaultIRCRC = CompositeReaderContext<DefaultCRReader>;
pub type DefaultIRCLR = LeafReaderContext<DefaultLRReader>;
pub type DefaultIndexSearchCRShared =
  DefaultIndexSearcher<CompositeReaderContext<DefaultCRReaderShared>>;
pub type DefaultIndexSearchCR = DefaultIndexSearcher<CompositeReaderContext<DefaultCRReader>>;
pub type DefaultIndexSearchLR = DefaultIndexSearcher<LeafReaderContext<DefaultLRReader>>;
pub type DummyCR = StandardDirectoryReaderType<FSDirectory<NativeFSLockFactory, NIOFSDirectory>>;

pub(crate) fn dummy_directory() -> Result<Arc<FSDirectory<NativeFSLockFactory, NIOFSDirectory>>> {
  let temp_dir = TempDir::new()?;
  Ok(Arc::new(NIOFSDirectory::new(temp_dir.keep())?))
}

pub(crate) fn dummy_index_searcher(
  dir: Arc<FSDirectory<NativeFSLockFactory, NIOFSDirectory>>,
) -> Result<DefaultIndexSearcher<CompositeReaderContext<DummyCR>>> {
  let iw = IndexWriter::new(dir, IndexWriterConfig::new()?)?;
  let mut doc = Document::new();
  doc.add(StringField::from_string(
    "id",
    format!("doc-{}", 0),
    Store::No,
  )?);
  iw.add_document(doc)?;
  let reader = iw.get_reader(true, true)?;
  let irc = get_context(reader)?;
  iw.close()?;
  IndexSearcher::new(irc)
}
