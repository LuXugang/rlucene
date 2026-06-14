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
use crate::core::util::automation::transition::Transition;

/// Trait for accessing an automaton's transitions.
pub trait TransitionAccessor {
  /// Initialize the provided `Transition` to iterate through all transitions
  /// leaving the specified state. Returns the number of transitions
  /// leaving this state.
  fn init_transition(&self, state: i32, t: &mut Transition) -> i32;

  /// Advance the provided `Transition` to the next transition.
  fn get_next_transition(&self, t: &mut Transition);

  /// How many transitions this state has.
  fn get_num_transitions_with_state(&self, state: i32) -> i32;

  /// Fill the provided `Transition` with the index‑th transition leaving the
  /// specified state.
  fn get_transition(&self, state: i32, index: i32, t: &mut Transition);
}
