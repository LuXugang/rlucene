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
pub mod automata;
pub mod automaton;
pub mod byte_runnable;
mod finite_strings_iterator;
mod frozen_int_set;
mod int_set;
mod minimization_operation;
pub mod operations;
pub mod run_automaton;
pub mod state_pair;
mod state_set;
mod strings_to_automaton;
pub mod transition;
pub(crate) mod transition_accessor;
