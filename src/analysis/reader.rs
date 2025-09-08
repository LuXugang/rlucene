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
use crate::util::error::lucene_error::Result;
pub trait Reader {
    /// Reads a single character. Returns -1 on EOF
    fn read(&mut self) -> Result<i32> {
        let mut cb: Vec<char> = vec![char::from(0); 1];
        if self.read_range(&mut cb, 0, 1)? == -1 {
            return Ok(-1);
        }
        Ok(cb[0] as i32)
    }
    /// Reads characters into the buffer, starting at `off`,
    /// up to `len` characters. Returns the number of chars read,
    /// or -1 on EOF.
    fn read_range(&mut self, buf: &mut [char], off: usize, len: usize) -> Result<i32>;
    fn close(&mut self) -> Result<()>;
}
