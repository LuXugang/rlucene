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
use crate::core::internal::vectorization::default_vector_util_support::DefaultVectorization;
use crate::core::internal::vectorization::vector_util_support::VectorUtilSupport;
use once_cell::sync::Lazy;

pub static VECTOR_UTIL: Lazy<VectorUtil> = Lazy::new(VectorUtil::default);
#[derive(Default)]
pub struct VectorUtil {
    impl_: DefaultVectorization,
}
impl VectorUtil {
    pub fn find_next_geq(&self, buffer: &[i32], target: i32, from: usize, to: usize) -> usize {
        debug_assert!({
            let mut ok = true;
            for i in 0..to.saturating_sub(1) {
                if buffer[i] > buffer[i + 1] {
                    ok = false;
                    break;
                }
            }
            ok
        });

        self.impl_.find_next_geq(buffer, target, from, to)
    }
}
