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
pub mod array_index_out_of_bounds;
pub mod corrupt_index;
pub mod data_io_error_enum;
pub mod eof;
pub mod illegal_argument;
pub mod illegal_state;
pub mod index_format_too_new;
pub mod index_format_too_old;
pub mod integer_overflow;
mod lock_already_held;
mod lock_held_by_other;
mod not_found;
pub mod parse;
mod unsupported_operation;
