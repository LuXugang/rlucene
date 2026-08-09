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
pub(crate) mod analysis;
pub(crate) mod codecs;
pub(crate) mod document;
pub(crate) mod geo;
pub(crate) mod index;
pub(crate) mod internal;
pub(crate) mod mockfile;
pub(crate) mod search;
pub(crate) mod store;
pub(crate) mod util;

mod test_assertions;
mod test_demo;
mod test_merge_scheduler_external;
mod test_module_resource_loader;
mod test_runtime_dependencies_sane;
mod test_search;
mod test_search_for_duplicates;
