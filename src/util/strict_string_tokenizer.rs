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
use crate::util::error::illegal_state::IllegalState;

pub struct StrictStringTokenizer<'a> {
    s: &'a str,
    delimiter: char,
    pos: Option<usize>,
}

impl<'a> StrictStringTokenizer<'a> {
    pub fn new(s: &'a str, delimiter: char) -> Self {
        Self {
            s,
            delimiter,
            pos: Some(0),
        }
    }

    pub fn next_token(&mut self) -> Result<&'a str, IllegalState> {
        if let Some(start) = self.pos {
            if start >= self.s.len() {
                self.pos = None;
                return Err(IllegalState::new("no more tokens"));
            }

            if let Some(end) = self.s[start..].find(self.delimiter) {
                let token = &self.s[start..start + end];
                self.pos = Some(start + end + 1);
                Ok(token)
            } else {
                let token = &self.s[start..];
                self.pos = None;
                Ok(token)
            }
        } else {
            Err(IllegalState::new("no more tokens"))
        }
    }

    pub fn has_more_tokens(&self) -> bool {
        self.pos.is_some()
    }
}
