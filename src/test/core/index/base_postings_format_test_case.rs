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
use crate::core::index::index_options::IndexOptions;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::index::base_index_file_format_test_case::BaseIndexFileFormatTestCase;
use crate::test::core::index::random_postings_tester::RandomPostingsTester;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::create_temp_dir_with_prefix;

pub trait BasePostingsFormatTestCase: BaseIndexFileFormatTestCase {
  fn create_postings<R>(&self, random: &mut R) -> RandomPostingsTester
  where
    R: rand::Rng + ?Sized;

  fn test_docs_only<R>(&self, random: &mut R) -> Result<()>
  where
    R: rand::Rng + ?Sized,
  {
    let mut postings_tester = self.create_postings(random);
    postings_tester.test_full(
      random,
      &self.get_codec()?,
      create_temp_dir_with_prefix("testPostingsFormat.testExact")?,
      IndexOptions::Docs,
      false,
    )
  }
}
