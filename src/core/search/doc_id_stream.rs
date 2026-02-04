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
use crate::core::search::scorable::Scorable;
use crate::core::util::error::lucene_error::Result;
/// Consumer for [`DocIdStream`] items.
pub trait DocIdStreamConsumer {
    fn visit(&mut self, doc: i32, scorer: &mut dyn Scorable) -> Result<()>;
}

/// A stream of doc IDs. Most methods on [`DocIdStream`]s are terminal,
/// meaning that the [`DocIdStream`] may not be further used.
///
/// @lucene.experimental
pub trait DocIdStream {
    fn scorer(&mut self) -> &mut dyn Scorable;
    /// Iterate over doc IDs contained in this stream in order,
    /// calling the given consumer on them.
    /// This is a terminal operation.
    fn for_each(&mut self, f: &mut dyn DocIdStreamConsumer) -> Result<()>;

    /// Count the number of entries in this stream.
    /// This is a terminal operation.
    fn count(&mut self) -> Result<i32>;
    fn default_count(&mut self) -> Result<i32> {
        struct CountConsumer {
            cnt: i32,
        }

        impl DocIdStreamConsumer for CountConsumer {
            fn visit(&mut self, _doc: i32, _scorer: &mut dyn Scorable) -> Result<()> {
                self.cnt += 1;
                Ok(())
            }
        }

        let mut counter = CountConsumer { cnt: 0 };
        self.for_each(&mut counter)?;
        Ok(counter.cnt)
    }
}
