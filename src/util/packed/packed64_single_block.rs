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
pub(crate)struct Packed64SingleBlock; 
impl Packed64SingleBlock{
    /// Supported bits per value
    const SUPPORTED_BITS_PER_VALUE: [u32; 14] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 16, 21, 32];

    /// Checks if the given `bits_per_value` is supported.
    pub fn is_supported(bits_per_value: u32) -> bool {
        Self::SUPPORTED_BITS_PER_VALUE.binary_search(&bits_per_value).is_ok()
    } 
}