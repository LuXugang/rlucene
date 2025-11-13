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
use crate::core::document::fields::Fields;
use crate::core::index::index_writer::{DefaultIndexWriterType, IndexWriter};
use crate::core::index::index_writer_config::IndexWriterConfig;
use crate::core::index::standard_directory_reader::StandardDirectoryReaderType;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;
use rand::Rng;
use std::sync::Arc;

pub struct RandomIndexWriter<D>
where
    D: Directory,
{
    w: DefaultIndexWriterType<D>,
}

impl<D> RandomIndexWriter<D>
where
    D: Directory,
{
    pub fn new<R: Rng + ?Sized>(_r: &mut R, dir: Arc<D>) -> Self
    where
        D: Directory,
    {
        Self {
            w: IndexWriter::new(dir, IndexWriterConfig::new()).expect("should not fail"),
        }
    }
    pub fn get_reader(&self) -> Result<StandardDirectoryReaderType<D>> {
        self.w.get_reader(true, false)
    }
    pub fn add_document<DF>(&self, doc: DF) -> Result<i64>
    where
        DF: IntoIterator<Item = Fields>,
    {
        self.w.add_document(doc)
    }
    pub fn close(&self) -> Result<()> {
        self.w.close()
    }
    pub fn flush(&self) -> Result<()> {
        self.w.flush()
    }
    pub fn commit(&self) -> Result<i64> {
        self.w.commit()
    }
}
