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
use crate::core::util::error::lucene_error::Result;

// These Java classes are self-tests for JUnit, RandomizedRunner, JVM permissions, or JVM object
// inspection. Declaring the class names here keeps their method mappings explicit even though the
// Rust test harness has no corresponding lifecycle/rule/runner extension points.
#[allow(dead_code)]
struct TestBeforeAfterOverrides;
#[allow(dead_code)]
struct TestCodecReported;
#[allow(dead_code)]
struct TestExceptionInBeforeClassHooks;
#[allow(dead_code)]
struct TestExpectThrows;
#[allow(dead_code)]
struct TestFailIfDirectoryNotClosed;
#[allow(dead_code)]
struct TestFailIfUnreferencedFiles;
#[allow(dead_code)]
struct TestGroupFiltering;
#[allow(dead_code)]
struct TestJUnitRuleOrder;
#[allow(dead_code)]
struct TestJvmInfo;
#[allow(dead_code)]
struct TestMaxFailuresRule;
#[allow(dead_code)]
struct TestPleaseFail;
#[allow(dead_code)]
struct TestRamUsageTesterOnWildAnimals;
#[allow(dead_code)]
struct TestReproduceMessage;
#[allow(dead_code)]
struct TestReproduceMessageWithRepeated;
#[allow(dead_code)]
struct TestRunWithRestrictedPermissions;
#[allow(dead_code)]
struct TestSeedFromUncaught;
#[allow(dead_code)]
struct TestSetupTeardownChaining;
#[allow(dead_code)]
struct TestSysoutsLimits;
#[allow(dead_code)]
struct TestWorstCaseTestBehavior;

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_empty() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_before() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_after() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_dummy() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_correct_codec_reported() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test1() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test2() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test3() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_exception_in_before_class_fails_the_test() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_exception_within_test_fails_the_test() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_exception_within_before() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_pass() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_fail() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_error() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_nested_fail() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_nested_assume() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_expecting_nested_fail() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_expecting_nested_assume() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_fail_if_directory_not_closed() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_fail_if_unreferenced_files() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_foo() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_foo_bar() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_bar() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_jira() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_rule_order() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_echo_jvm_info() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_fail_sometimes() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_max_failures() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_leave_zombie() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_zombie_thread_failures() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_overflow_max_chain_length() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_assume_before_class() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_assume_initializer() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_assume_rule() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_assume_before() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_assume_test() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_assume_after() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_assume_after_class() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_failure_before_class() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_failure_initializer() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_failure_rule() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_failure_before() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_failure_test() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_failure_after() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_failure_after_class() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_error_before_class() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_error_initializer() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_error_rule() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_error_before() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_error_test() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_error_after() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_error_after_class() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_me() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_repeated_message() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_defaults_pass() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_normally_allowed_stuff() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_completely_forbidden1() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_completely_forbidden2() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_uncaught_dumps_seed() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_setup_chaining() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_teardown_chaining() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_write() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_over_soft_limit() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_under_limit() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn over_hard_limit() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_thread_leak() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_laaaaaarge_output() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_progressive_output() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_uncaught_exception() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_timeout() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this exercises the JUnit/RandomizedRunner or JVM test harness"]
fn test_zombie() -> Result<()> {
  test_not_required_in_rust_lucene!();
}
