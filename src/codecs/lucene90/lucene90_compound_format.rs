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
pub struct Lucene90CompoundFormat;
impl Lucene90CompoundFormat {
    pub const DATA_EXTENSION: &'static str = "cfs";
    pub const ENTRIES_EXTENSION: &'static str = "cfe";
    pub const DATA_CODEC: &'static str = "Lucene90CompoundData";
    pub const ENTRY_CODEC: &'static str = "Lucene90CompoundEntries";
    pub const VERSION_START: i32 = 0;
    pub const VERSION_CURRENT: i32 = Self::VERSION_START;
}
