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
use std::env;

#[derive(Clone)]
pub enum ReadAdvice {
    
    ///  Normal behavior. Data is expected to be read mostly sequentially. The system is expected to
     /// cache the hottest pages.
    Normal,
    ///Data is expected to be read in a random-access fashion, either by `IndexInput#seek(i64)`
    ///seeking often and reading relatively i16 sequences of bytes at once, or by reading data
    ///through the `RandomAccessInput` abstraction in random order.
    Random,
     /// Data is expected to be read sequentially with very little seeking at most. The system may read
     /// ahead aggressively and free pages soon after they are accessed.
     
    Sequential,
   ///
   ///Data is treated as random-access memory in practice. `Directory` implementations may
   ///explicitly load the content of the file in memory, or provide hints to the system so that it
   ///loads the content of the file into the page cache at open time. This should only be used on
   ///very small files that can be expected to fit in RAM with very high confidence.
   ///
    RandomPreload,
}

impl ReadAdvice {
    pub fn from_str_custom(s: &str) -> Option<ReadAdvice> {
        match s.to_uppercase().as_str() {
            "NORMAL" => Some(ReadAdvice::Normal),
            "RANDOM" => Some(ReadAdvice::Random),
            "SEQUENTIAL" => Some(ReadAdvice::Sequential),
            "RANDOM PRELOAD" => Some(ReadAdvice::RandomPreload),
            _ => None,
        }
    }
    pub fn default_read_advice() -> ReadAdvice {
        env::var("lucene.store.defaultReadAdvice")
            .ok()
            .and_then(|value| ReadAdvice::from_str_custom(&value))
            .unwrap_or(ReadAdvice::Random)
    }
}
