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
use crate::core::util::bit_util::BitUtil;
use crate::core::util::error::lucene_error::{LuceneError, Result};

/// Basic parameters for indexing points on the BKD tree.
///
/// # Parameters
/// - `num_dims`: How many dimensions are stored at the leaf (data) node.
/// - `num_index_dims`: How many dimensions are indexed in the internal nodes.
/// - `bytes_per_dim`: How many bytes each value in each dimension takes.
/// - `max_points_in_leaf_node`: Maximum points allowed in a leaf block.
#[derive(Clone, Debug, Default)]
pub struct BKDConfig {
    pub num_dims: usize,
    pub num_index_dims: usize,
    pub bytes_per_dim: usize,
    pub max_points_in_leaf_node: usize,
}

impl BKDConfig {
    /// Default maximum number of points in each leaf block.
    pub const DEFAULT_MAX_POINTS_IN_LEAF_NODE: usize = 512;
    /// Maximum number of index dimensions (2 * max index dimensions).
    pub const MAX_DIMS: usize = 16;
    /// Maximum number of index dimensions.
    pub const MAX_INDEX_DIMS: usize = 8;
    /// Creates a new `BKDConfig` instance after validating the inputs.
    ///
    /// # Errors
    ///
    /// Returns an `Err(String)` if any of the validations fail.
    ///
    /// # Validations
    ///
    /// - `num_dims` must be between 1 and `MAX_DIMS` (inclusive).
    /// - `num_index_dims` must be between 1 and `MAX_INDEX_DIMS` (inclusive).
    /// - `num_index_dims` cannot exceed `num_dims`.
    /// - `bytes_per_dim` must be greater than 0.
    /// - `max_points_in_leaf_node` must be greater than 0 and less than or
    ///   equal to `MAX_ARRAY_LENGTH`.
    pub fn new(
        num_dims: usize,
        num_index_dims: usize,
        bytes_per_dim: usize,
        max_points_in_leaf_node: usize,
    ) -> Result<Self> {
        if !(1..=Self::MAX_DIMS).contains(&num_dims) {
            return Err(LuceneError::illegal_argument(format!(
                "num_dims must be 1 .. {} (got: {})",
                Self::MAX_DIMS,
                num_dims
            )));
        }
        if !(1..=Self::MAX_INDEX_DIMS).contains(&num_index_dims) {
            return Err(LuceneError::illegal_argument(format!(
                "num_index_dims must be 1 .. {} (got: {})",
                Self::MAX_INDEX_DIMS,
                num_index_dims
            )));
        }
        if bytes_per_dim == 0 {
            return Err(LuceneError::illegal_argument(format!(
                "bytes_per_dim must be > 0; got {bytes_per_dim}"
            )));
        }
        if num_index_dims > num_dims {
            return Err(LuceneError::illegal_argument(format!(
                "num_index_dims cannot exceed num_dims (got: {num_dims} vs {num_index_dims})"
            )));
        }
        //TODO: Implement ArrayUtil::MAX_ARRAY_LENGTH
        // if max_points_in_leaf_node > ArrayUtil::MAX_ARRAY_LENGTH {
        //     return Err(LuceneError::illegal_argument(format!(
        //         "max_points_in_leaf_node must be <= MAX_ARRAY_LENGTH (= {});
        // got {}",         ArrayUtil::MAX_ARRAY_LENGTH,
        //         max_points_in_leaf_node
        //     )));
        // }
        Ok(Self {
            num_dims,
            num_index_dims,
            bytes_per_dim,
            max_points_in_leaf_node,
        })
    }

    /// Returns `num_dims * bytes_per_dim`.
    pub fn packed_bytes_length(&self) -> usize {
        self.num_dims * self.bytes_per_dim
    }

    /// Returns `num_index_dims * bytes_per_dim`.
    pub fn packed_index_bytes_length(&self) -> usize {
        self.num_index_dims * self.bytes_per_dim
    }

    /// Returns `(num_dims * bytes_per_dim) + size_of::<i32>()`
    /// (packed_bytes_length plus document ID size).
    pub fn bytes_per_doc(&self) -> usize {
        self.packed_bytes_length() + BitUtil::INT_BYTES
    }
}
#[cfg(test)]
mod tests {
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
            // TODO:
            // let result = BKDConfig::new(1, 1, 8, ArrayUtil::MAX_ARRAY_LENGTH
            // + 1); assert!(result.is_err());
            // if let Err(err) = result {
            //     let err_msg = format!("{:?}", err);
            //     assert!(
            //         err_msg.contains("max_points_in_leaf_node must be <=
            // ArrayUtil::MAX_ARRAY_LENGTH")     );
            // }
        }
    }
}
