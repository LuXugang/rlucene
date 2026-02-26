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
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::terms::{Terms, TermsPosting};
use crate::core::index::terms_enum::TermsEnum;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::query::QueryBase;
use crate::core::util::error::lucene_error::Result;

pub trait MultiTermQuery: QueryBase {
    fn get_field(&self) -> &str;
    type TermsEnum<T>: TermsEnum<PostingsEnum = TermsPosting<T>>
    where
        T: Terms;
    fn get_terms_enum<T>(&self, terms: T) -> Result<Self::TermsEnum<T>>
    where
        T: Terms + Clone;
    fn get_terms_count(&self) -> i64;
}

pub trait RewriteMethod {
    fn rewrite<IRC, Q>(self, index_searcher: &IndexSearcher<IRC>, query: Q) -> Result<Q>
    where
        IRC: IndexReaderContext,
        Q: MultiTermQuery + Sized;
}
