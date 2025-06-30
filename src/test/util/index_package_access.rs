/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */
use crate::index::field_info::FieldInfo;
use crate::index::field_infos::build::Builder;
use crate::index::field_infos::{FieldInfos, FieldNumbers};
use crate::util::error::lucene_error::Result;
use parking_lot::lock_api::Mutex;
use std::rc::Rc;
use std::sync::Arc;

pub(crate) trait IndexPackageAccess {
    // type CacheKey;
    type FieldInfosBuilder: FieldInfosBuilder;
    // fn new_cache_key(&self) -> Self::CacheKey;
    // fn set_index_writer_max_docs(&mut self, limit: i32);
    fn new_field_infos_builder(
        &self,
        soft_deletes_field_name: Option<String>,
        parent_field_name: Option<String>,
    ) -> Result<Self::FieldInfosBuilder>;
    // fn check_impacts(&self, impacts: Impacts, max: i32);
}
pub(crate) trait FieldInfosBuilder {
    fn add(&mut self, fi: Rc<FieldInfo>) -> Result<&mut Self>;
    fn finish(&mut self) -> Result<FieldInfos>;
}

pub(crate) struct IndexPackageAccessImpl;
impl IndexPackageAccess for IndexPackageAccessImpl {
    type FieldInfosBuilder = FieldInfosBuilderImpl;

    fn new_field_infos_builder(
        &self,
        soft_deletes_field_name: Option<String>,
        parent_field_name: Option<String>,
    ) -> Result<Self::FieldInfosBuilder> {
        FieldInfosBuilderImpl::new(soft_deletes_field_name, parent_field_name)
    }
}

pub(crate) struct FieldInfosBuilderImpl {
    builder: Builder,
}
impl FieldInfosBuilderImpl {
    pub fn new(
        soft_deletes_field_name: Option<String>,
        parent_field_name: Option<String>,
    ) -> Result<Self> {
        let field_number = FieldNumbers::new(soft_deletes_field_name, parent_field_name)?;
        Ok(FieldInfosBuilderImpl {
            builder: Builder::new(Arc::new(Mutex::new(field_number))),
        })
    }
}
impl FieldInfosBuilder for FieldInfosBuilderImpl {
    fn add(&mut self, fi: Rc<FieldInfo>) -> Result<&mut Self> {
        self.builder.add(fi)?;
        Ok(self)
    }

    fn finish(&mut self) -> Result<FieldInfos> {
        self.builder.finish()
    }
}
