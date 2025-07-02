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
use crate::index::BytesRef;
use crate::util::attribute::Attribute;
/// The payload of a token.
///
/// The payload is stored in the index at each position, and can be used to
/// influence scoring when using payload-based queries.
///
/// **Note:** Because the payload will be stored at each position, it's usually
/// best to use the minimum number of bytes necessary. Some codec
/// implementations may optimize payload storage when all payloads have the same
/// length.
///
/// See also: [`PostingsEnum`](crate::index::postings_enum::PostingsEnum)
pub trait PayloadAttribute: Attribute {
    /// Returns this token's payload.
    ///
    /// See also: [`Self::set_payload`]
    fn get_payload(&self) -> &BytesRef<Vec<u8>>;
    /// Sets this token's payload.
    ///
    /// See also: [`Self::get_payload`]
    fn set_payload(&mut self, payload: BytesRef<Vec<u8>>);
}
