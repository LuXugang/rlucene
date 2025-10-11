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
use crate::core::search::sort_field::SortField;
use crate::core::search::sort_field_enum::SortFieldEnum;
use crate::core::util::error::lucene_error::Result;

pub trait TopFieldCollector {}
fn can_early_terminate(search_sort: &Sort, index_sort: Option<&Sort>) -> Result<bool> {
    Ok(can_early_terminate_on_doc_id(search_sort)?
        || can_early_terminate_on_prefix(search_sort, index_sort)?)
}

fn can_early_terminate_on_doc_id(search_sort: &Sort) -> Result<bool> {
    let fields = search_sort.get_sort();
    if let Some(SortFieldEnum::Sorter(field)) = fields.first() {
        let field_doc = SortField::get_field_doc()?;
        Ok(*field == field_doc)
    } else {
        Ok(false)
    }
}
fn can_early_terminate_on_prefix(search_sort: &Sort, index_sort: Option<&Sort>) -> Result<bool> {
    if let Some(index_sort) = index_sort {
        let fields1 = search_sort.get_sort();
        let fields2 = index_sort.get_sort();

        if fields1.len() > fields2.len() {
            return Ok(false);
        }

        Ok(fields1.iter().zip(fields2.iter()).all(|(a, b)| a == b))
    } else {
        Ok(false)
    }
}
