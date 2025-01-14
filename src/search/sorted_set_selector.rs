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
pub struct SortedSetSelector;

/// Type of selection to perform.
///
/// # Limitations
/// - Fields containing `i32::MAX` or more unique values are unsupported.
/// - Selectors other than [`SortedSetSelectorType::Min`] require optional codec support. However, several
///   codecs provided by Lucene, including the current default codec, support this.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortedSetSelectorType {
    /// Selects the minimum value in the set.
    Min,
    /// Selects the maximum value in the set.
    Max,
    /// Selects the middle value in the set.
    ///
    /// If the set has an even number of values, the lower of the middle two is chosen.
    MiddleMin,
    /// Selects the middle value in the set.
    ///
    /// If the set has an even number of values, the higher of the middle two is chosen.
    MiddleMax,
}
