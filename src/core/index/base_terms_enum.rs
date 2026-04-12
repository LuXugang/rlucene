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
use crate::core::index::term_state::TermState;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::fmt::{Debug, Display, Formatter};

/// # Note
/// See [`JavaIntermediateBaseClass`](crate::migration_notes::JavaIntermediateBaseClass)
pub struct BaseTermsEnum;

#[derive(Debug, Clone, Default)]
pub struct BaseTermsEnumTermStateImpl;
impl Display for BaseTermsEnumTermStateImpl {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}
impl TermState for BaseTermsEnumTermStateImpl {
  fn copy_from(&mut self, _other: &Self) -> Result<()> {
    Err(LuceneError::unsupported_operation(""))
  }
}
