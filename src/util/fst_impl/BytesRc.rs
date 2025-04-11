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
use std::rc::Rc;

pub struct BytesRc {
    pub bytes: Rc<Vec<u8>>,
    pub offset: i32,
    pub length: i32,
}
impl BytesRc {
    pub fn from_vec(bytes: Vec<u8>, offset: i32, length: i32) -> BytesRc {
        BytesRc {
            bytes: Rc::new(bytes),
            offset,
            length,
        }
    }
    pub fn from_bytes(bytes: Vec<u8>) -> BytesRc {
        debug_assert!(bytes.len() <= i32::MAX as usize);
        let length = bytes.len() as i32;
        Self::from_vec(bytes, 0, length)
    }
}
