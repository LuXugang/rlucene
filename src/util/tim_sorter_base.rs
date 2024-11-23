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

pub trait TimSorterBase {
    /** Copy data from slot `src` to slot `dest`. */
    fn copy(&self, src: usize, dest: usize);

    /**
     * Save all elements between slots i and `i+len` into the temporary
     * storage.
     */
    fn save(&self, i: usize, len: usize);
    /** Restore element `j` from the temporary storage into slot `i`. */
    fn restore(&self, i: usize, j: usize);
    /**
     * Compare element `i` from the temporary storage with element `j` from the
     * slice to sort, similarly to #compare(usize, usize).
     */

    fn compare_saved(&self, i: usize, j: usize) -> i32;
}
