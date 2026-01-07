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
use crc32fast::Hasher;

pub trait Checksum {
    fn update(&mut self, b: u8);
    fn update_bytes(&mut self, bytes: &[u8], offset: usize, len: usize);
    fn get_value(&mut self) -> i64;
    fn reset(&mut self);
}

pub struct HasherChecksum {
    hasher: Hasher,
    initial_state: Hasher,
}

impl HasherChecksum {
    pub fn new(hasher: Hasher) -> Self {
        Self {
            hasher: hasher.clone(),
            initial_state: hasher,
        }
    }
}

impl Checksum for HasherChecksum {
    fn update(&mut self, b: u8) {
        self.hasher.update(&[b]);
    }

    fn update_bytes(&mut self, bytes: &[u8], offset: usize, len: usize) {
        self.hasher.update(&bytes[offset..offset + len]);
    }

    fn get_value(&mut self) -> i64 {
        self.hasher.clone().finalize() as i64
    }

    fn reset(&mut self) {
        self.hasher = self.initial_state.clone();
    }
}
