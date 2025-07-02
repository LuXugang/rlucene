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
use crate::util::attribute::Attribute;
use crate::util::error::lucene_error::Result;

/// Determines the position of this token relative to the previous `Token` in a `TokenStream`,  
/// used in phrase searching.
///
/// The default value is `1`.
///
/// Some common uses for this are:
/// - Set it to zero to put multiple terms in the same position. This is useful if, e.g., a word  
///   has multiple stems. Searches for phrases including either stem will match. In this case,  
///   all but the first stem's increment should be set to zero: the increment of the first  
///   instance should be one. Repeating a token with an increment of zero can also be used to  
///   boost the scores of matches on that token.
/// - Set it to values greater than one to inhibit exact phrase matches. If, for example, one  
///   does not want phrases to match across removed stop words, then one could build a stop word  
///   filter that removes stop words and also sets the increment to the number of stop words  
///   removed before each non-stop word. Then exact phrase queries will only match when the terms  
///   occur with no intervening stop words.
///
/// # See
/// [`PostingsEnum`](crate::index::postings_enum::PostingsEnum)
pub trait PositionIncrementAttribute: Attribute {
    /// Set the position increment. The default value is `1`.
    ///
    /// # Parameters
    ///
    /// - `position_increment`: the distance from the prior term; must be non-negative.
    ///
    /// # Error
    ///
    /// Error if `position_increment < 0`.
    fn set_position_increment(&mut self, position_increment: i32) -> Result<()>;

    /// Returns the position increment of this Token.
    ///
    /// # See
    ///
    /// [`set_position_increment`](PositionIncrementAttribute::set_position_increment)
    fn get_position_increment(&self) -> i32;
}
