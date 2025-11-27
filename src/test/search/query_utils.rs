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
use crate::core::util::CoreHelper;
use std::hash::Hash;

pub struct QueryUtils;
impl QueryUtils {
    pub fn check_equal<Q>(q1: &Q, q2: &Q)
    where
        Q: Eq + Hash + PartialEq,
    {
        assert!(q1 == q2);

        let hash1 = CoreHelper::calculate_hash(q1);
        let hash2 = CoreHelper::calculate_hash(q2);
        assert_eq!(hash1, hash2);
    }
    pub fn check_unequal<Q>(q1: &Q, q2: &Q)
    where
        Q: Eq + Hash + PartialEq + std::fmt::Debug,
    {
        assert_ne!(q1, q2);

        assert_ne!(q2, q1);
    }
}
