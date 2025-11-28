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
use crate::core::index::sort::Sort;
use crate::core::search::sort_field::{SortField, SortFieldType};
use crate::core::search::top_field_collector::can_early_terminate;
use crate::core::util::error::lucene_error::Result;
#[allow(dead_code)] // for quick search
struct TestTopFieldCollectorEarlyTermination;

// TODO 还有其他测试未实现
#[test]
fn test_can_early_terminate_on_doc_id() -> Result<()> {
    assert!(can_early_terminate(
        &Sort::with_fields(vec![SortField::get_field_doc()?])?,
        Some(&Sort::with_fields(vec![SortField::get_field_doc()?])?)
    )?);

    assert!(can_early_terminate(
        &Sort::with_fields(vec![SortField::get_field_doc()?])?,
        None
    )?);

    assert!(!can_early_terminate(
        &Sort::with_fields(vec![SortField::with_reverse(
            Some("a"),
            SortFieldType::Long,
            false
        )?])?,
        None
    )?);

    assert!(!can_early_terminate(
        &Sort::with_fields(vec![SortField::with_reverse(
            Some("a"),
            SortFieldType::Long,
            false
        )?])?,
        Some(&Sort::with_fields(vec![SortField::with_reverse(
            Some("b"),
            SortFieldType::Long,
            false
        )?])?)
    )?);

    assert!(can_early_terminate(
        &Sort::with_fields(vec![SortField::get_field_doc()?])?,
        Some(&Sort::with_fields(vec![SortField::with_reverse(
            Some("b"),
            SortFieldType::Long,
            false
        )?])?)
    )?);

    assert!(can_early_terminate(
        &Sort::with_fields(vec![SortField::get_field_doc()?])?,
        Some(&Sort::with_fields(vec![
            SortField::with_reverse(Some("a"), SortFieldType::Long, false)?,
            SortField::get_field_doc()?
        ])?)
    )?);

    assert!(!can_early_terminate(
        &Sort::with_fields(vec![SortField::with_reverse(
            Some("a"),
            SortFieldType::Long,
            false
        )?])?,
        Some(&Sort::with_fields(vec![SortField::get_field_doc()?])?)
    )?);

    assert!(!can_early_terminate(
        &Sort::with_fields(vec![
            SortField::with_reverse(Some("a"), SortFieldType::Long, false)?,
            SortField::get_field_doc()?
        ])?,
        Some(&Sort::with_fields(vec![SortField::get_field_doc()?])?)
    )?);

    Ok(())
}
#[test]
fn test_can_early_terminate_on_prefix() -> Result<()> {
    assert!(can_early_terminate(
        &Sort::with_fields(vec![SortField::with_reverse(
            Some("a"),
            SortFieldType::Long,
            false
        )?])?,
        Some(&Sort::with_fields(vec![SortField::with_reverse(
            Some("a"),
            SortFieldType::Long,
            false
        )?])?)
    )?);

    assert!(can_early_terminate(
        &Sort::with_fields(vec![
            SortField::with_reverse(Some("a"), SortFieldType::Long, false)?,
            SortField::with_reverse(Some("b"), SortFieldType::String, false)?,
        ])?,
        Some(&Sort::with_fields(vec![
            SortField::with_reverse(Some("a"), SortFieldType::Long, false)?,
            SortField::with_reverse(Some("b"), SortFieldType::String, false)?,
        ])?)
    )?);

    assert!(can_early_terminate(
        &Sort::with_fields(vec![SortField::with_reverse(
            Some("a"),
            SortFieldType::Long,
            false
        )?])?,
        Some(&Sort::with_fields(vec![
            SortField::with_reverse(Some("a"), SortFieldType::Long, false)?,
            SortField::with_reverse(Some("b"), SortFieldType::String, false)?,
        ])?)
    )?);

    assert!(!can_early_terminate(
        &Sort::with_fields(vec![SortField::with_reverse(
            Some("a"),
            SortFieldType::Long,
            true
        )?])?,
        None
    )?);

    assert!(!can_early_terminate(
        &Sort::with_fields(vec![SortField::with_reverse(
            Some("a"),
            SortFieldType::Long,
            true
        )?])?,
        Some(&Sort::with_fields(vec![SortField::with_reverse(
            Some("a"),
            SortFieldType::Long,
            false
        )?])?)
    )?);

    assert!(!can_early_terminate(
        &Sort::with_fields(vec![
            SortField::with_reverse(Some("a"), SortFieldType::Long, false)?,
            SortField::with_reverse(Some("b"), SortFieldType::String, false)?,
        ])?,
        Some(&Sort::with_fields(vec![SortField::with_reverse(
            Some("a"),
            SortFieldType::Long,
            false
        )?])?)
    )?);

    assert!(!can_early_terminate(
        &Sort::with_fields(vec![
            SortField::with_reverse(Some("a"), SortFieldType::Long, false)?,
            SortField::with_reverse(Some("b"), SortFieldType::String, false)?,
        ])?,
        Some(&Sort::with_fields(vec![
            SortField::with_reverse(Some("a"), SortFieldType::Long, false)?,
            SortField::with_reverse(Some("c"), SortFieldType::String, false)?,
        ])?)
    )?);

    assert!(!can_early_terminate(
        &Sort::with_fields(vec![
            SortField::with_reverse(Some("a"), SortFieldType::Long, false)?,
            SortField::with_reverse(Some("b"), SortFieldType::String, false)?,
        ])?,
        Some(&Sort::with_fields(vec![
            SortField::with_reverse(Some("c"), SortFieldType::Long, false)?,
            SortField::with_reverse(Some("b"), SortFieldType::String, false)?,
        ])?)
    )?);

    Ok(())
}
