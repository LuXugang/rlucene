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
use std::hash::Hasher;

pub trait Checksum {
    fn update(&mut self, b: u8);
    fn update_bytes(&mut self, bytes: &[u8], offset: u32, len: u32);
    fn get_value(&mut self) -> u32;
    fn reset(&mut self);
}

pub struct HasherChecksum<T: Hasher> {
    hasher: T,
    initial_state: T,
}

impl<T: Hasher + Clone> HasherChecksum<T> {
    pub fn new(hasher: T) -> Self {
        Self {
            initial_state: hasher.clone(),
            hasher,
        }
    }
}

impl<T: Hasher + Clone> Checksum for HasherChecksum<T> {
    fn update(&mut self, b: u8) {
        self.hasher.write(&[b]);
    }

    fn update_bytes(&mut self, bytes: &[u8], offset: u32, len: u32) {
        let offset = offset as usize;
        let len = len as usize;
        self.hasher.write(&bytes[offset..offset + len]);
    }

    fn get_value(&mut self) -> u32 {
        (self.hasher.finish() & 0xFFFFFFFF) as u32
    }

    fn reset(&mut self) {
        self.hasher = self.initial_state.clone();
    }
}
