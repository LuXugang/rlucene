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
use crate::util::access::AccessVec;
use crate::util::bytes_ref_iterator::BytesRefIterator;

/// Iterates over terms across multiple fields. The caller must check [`field()`](FieldTermIterator::field) after each [`next()`](BytesRefIterator::next)
/// to see if the field changed, but `==` can be used since the iterator implementation ensures
/// it will use the same `String` instance for a given field.
pub trait FieldTermIterator<AV>: BytesRefIterator<AV>
where
    AV: AccessVec<u8>,
{
    /// Returns the current field. This method should not be called after iteration is done.
    /// Note that you may use `==` to detect a change in field.
    fn field(&self) -> &str;

    /// Returns the del generation of the current term.
    /// Note: In some cases, this represents the current iterator (e.g., when using
    /// `MergedPrefixCodedTermsIterator`) to identify which iterator is active.
    fn del_gen(&self) -> i64;
}
