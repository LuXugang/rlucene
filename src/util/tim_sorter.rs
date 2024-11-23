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
use crate::util::Sorter;

/**
 * Sorter implementation based on the
 * href="http://svn.python.org/projects/python/trunk/Objects/listsort.txt">TimSort</a> algorithm. It
 * sorts small arrays with a binary sort.
 *
 * This algorithm is stable. It's especially good at sorting partially-sorted arrays.
 *
 * NOTE:There are a few differences with the original implementation:
 *
 * `maxTempSlots` The extra amount of memory to perform merges is configurable. This
 *       allows small merges to be very fast while large merges will be performed in-place (slightly
 *       slower). You can make sure that the fast merge routine will always be used by having
 *       `maxTempSlots` equal to half of the length of the slice of data to sort.
 *   <li>Only the fast merge routine can gallop (the one that doesn't run in-place) and it only
 *       gallops on the longest slice.
 * </ul>
 *
 */
pub trait TimSorter: Sorter {}
