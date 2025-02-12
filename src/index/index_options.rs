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
/// Controls how much information is stored in the postings lists.
///
/// # Experimental
pub enum IndexOptions {
    /// Not indexed
    NONE,
    /// Only documents are indexed: term frequencies and positions are omitted.
    /// Phrase and other positional queries on the field will throw an exception,
    /// and scoring will behave as if any term in the document appears only once.
    DOCS,
    /// Only documents and term frequencies are indexed: positions are omitted.
    /// This enables normal scoring, but Phrase and other positional queries will throw an Error.
    DocsAndFreqs,
    /// Indexes documents, frequencies, and positions.
    /// This is the typical default for full-text search: full scoring is enabled, and positional queries are supported.
    DocsAndFreqsAndPositions,
    /// Indexes documents, frequencies, positions, and offsets.
    /// Character offsets are encoded alongside the positions.
    DocsAndFreqsAndPositionsAndOffsets,
}
