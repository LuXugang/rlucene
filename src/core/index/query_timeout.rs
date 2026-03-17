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
use crate::core::index::query_timeout_impl::QueryTimeoutImpl;
use std::fmt::{Display, Formatter};

/// Query timeout abstraction that controls whether a query should continue or be stopped.
///
/// Can be set to the searcher through `IndexSearcher::set_timeout`,
/// in which case bulk scoring will be time-bound.
/// Can also be used in combination with `ExitableDirectoryReader`.
pub trait QueryTimeout {
  /// Called to determine whether to stop processing a query.
  ///
  /// # Returns
  /// `true` if the query should stop, `false` otherwise.
  fn should_exit(&self) -> bool;
}

pub type DynQueryTimeout = dyn QueryTimeout + Send + Sync;
pub type CustomQueryTimeout = Box<DynQueryTimeout>;

pub enum QueryTimeoutEnum {
  Builtin(QueryTimeoutImpl),
  Custom(CustomQueryTimeout),
}

impl QueryTimeoutEnum {
  pub fn custom<T>(t: T) -> Self
  where
    T: QueryTimeout + Send + Sync + 'static,
  {
    Self::Custom(Box::new(t))
  }
}

impl Display for QueryTimeoutEnum {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Builtin(inner) => write!(f, "{}", inner),
      Self::Custom(_) => write!(f, "CustomQueryTimeout"),
    }
  }
}

impl QueryTimeout for QueryTimeoutEnum {
  fn should_exit(&self) -> bool {
    match self {
      Self::Builtin(inner) => inner.should_exit(),
      Self::Custom(inner) => inner.should_exit(),
    }
  }
}
