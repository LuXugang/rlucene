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
use crate::core::util::array_util::ArrayUtil;
use crate::core::util::bkd::bkd_config::BKDConfig;

#[allow(dead_code)] // for quick search
struct TestBKDConfig;
#[test]
fn test_invalid_num_dims() {
  let result = BKDConfig::new(0, 0, 8, BKDConfig::DEFAULT_MAX_POINTS_IN_LEAF_NODE);
  assert!(result.is_err());
  if let Err(err) = result {
    let err_msg = format!("{:?}", err);
    assert!(
      err_msg.contains("num_dims must be 1 .. ")
        && err_msg.contains(&BKDConfig::MAX_DIMS.to_string())
    );
  }
}
#[test]
fn test_invalid_num_indexed_dims() {
  {
    let result = BKDConfig::new(1, 0, 8, BKDConfig::DEFAULT_MAX_POINTS_IN_LEAF_NODE);
    assert!(result.is_err());
    if let Err(err) = result {
      let err_msg = format!("{:?}", err);
      assert!(
        err_msg.contains("num_index_dims must be 1 .. ")
          && err_msg.contains(&BKDConfig::MAX_INDEX_DIMS.to_string())
      );
    }
  }
  {
    let result = BKDConfig::new(1, 2, 8, BKDConfig::DEFAULT_MAX_POINTS_IN_LEAF_NODE);
    assert!(result.is_err());
    if let Err(err) = result {
      let err_msg = format!("{:?}", err);
      assert!(err_msg.contains("num_index_dims cannot exceed num_dims"));
    }
  }
}
#[test]
fn test_invalid_bytes_per_dim() {
  let result = BKDConfig::new(1, 1, 0, BKDConfig::DEFAULT_MAX_POINTS_IN_LEAF_NODE);
  assert!(result.is_err());
  if let Err(err) = result {
    let err_msg = format!("{:?}", err);
    assert!(err_msg.contains("bytes_per_dim must be > 0"));
  }
}

#[test]
fn test_invalid_max_points_per_leaf_node() {
  {
    let result = BKDConfig::new(1, 1, 8, 0);
    assert!(result.is_err());
    if let Err(err) = result {
      let err_msg = format!("{:?}", err);
      assert!(err_msg.contains("max_points_in_leaf_node must be > 0"));
    }
  }
  {
    let result = BKDConfig::new(1, 1, 8, ArrayUtil::MAX_ARRAY_LENGTH + 1);
    assert!(result.is_err());
    if let Err(err) = result {
      let err_msg = format!("{:?}", err);
      assert!(err_msg.contains("max_points_in_leaf_node must be <= ArrayUtil::MAX_ARRAY_LENGTH"));
    }
  }
}
