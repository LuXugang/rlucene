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

/**
 * Expert: Collectors are primarily meant to be used to gather raw results from a search, and
 * implement sorting or custom result filtering, collation, etc.
 *
 * Lucene's core collectors are derived from `Collector` and `SimpleCollector`.
 * Likely your application can use one of these classes, or subclass `TopDocsCollector`,
 * instead of implementing Collector directly:
 *
 *   `TopDocsCollector` is an abstract base class that assumes you will retrieve the top N
 *       docs, according to some criteria, after collection is done.
 *   `TopScoreDocCollector` is a concrete subclass `TopDocsCollector` and sorts
 *       according to score + docID. This is used internally by the `IndexSearcher` search
 *       methods that do not take an explicit `Sort`. It is likely the most frequently used
 *       collector.
 *   `TopFieldCollector` subclasses `TopDocsCollector` and sorts according to a
 *       specified `Sort` object (sort by field). This is used internally by the
 *       `IndexSearcher` search methods that take an explicit `Sort`.
 *   `PositiveScoresOnlyCollector` wraps any other Collector and prevents collection of
 *       hits whose score is <= 0.0
 *
 */
pub trait Collector {}
