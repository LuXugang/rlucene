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
use crate::analysis::reader::Reader;
use crate::util::error::lucene_error::{LuceneError, Result};

pub(crate) struct IllegalStateReader;
impl Reader for IllegalStateReader {
    fn read_range(&mut self, _buf: &mut [char], _off: usize, _len: usize) -> Result<i32> {
        Err(LuceneError::illegal_state(
            "TokenStream contract violation: reset()/close() call missing, \
reset() called multiple times, or subclass does not call super.reset(). \
Please see Javadocs of TokenStream class for more information about the correct consuming workflow.",
        ))
    }

    fn close(&mut self) -> Result<()> {
        Ok(())
    }
}
