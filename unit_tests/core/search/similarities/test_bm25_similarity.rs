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
use crate::core::search::similarities_impl::bm25_similarity::BM25Similarity;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test_framework::core::search::base_similarity_test_case::BaseSimilarityTestCase;
use crate::test_framework::core::util::lucene_test_case::random;
use rand::Rng;
use rand::RngExt;

struct TestBM25Similarity;
impl BaseSimilarityTestCase for TestBM25Similarity {
  type Similarity = BM25Similarity;

  fn get_similarity<R>(&self, random: &mut R) -> Result<Self::Similarity>
  where
    R: Rng + ?Sized,
  {
    let k1: f32 = match random.random_range(0..4) {
      0 => 0.0,
      1 => f32::MIN_POSITIVE,
      2 => i32::MAX as f32,
      _ => {
        let r: f32 = random.random();
        (i32::MAX as f32) * r
      },
    };

    let b: f32 = match random.random_range(0..4) {
      0 => 0.0,
      1 => f32::MIN_POSITIVE,
      2 => 1.0,
      _ => random.random(),
    };

    BM25Similarity::with_k1_b(k1, b)
  }
}
#[test]
fn test_random_scoring() -> Result<()> {
  let mut random = random();
  let case = TestBM25Similarity;
  case.test_random_scoring(&mut random)
}
#[test]
fn test_illegal_k1() -> Result<()> {
  {
    let err = BM25Similarity::with_k1_b(f32::INFINITY, 0.75);
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
    assert!(err.unwrap_err().to_string().contains("illegal k1 value"));
  }

  {
    let err = BM25Similarity::with_k1_b(-1.0, 0.75);
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
    assert!(err.unwrap_err().to_string().contains("illegal k1 value"));
  }

  {
    let err = BM25Similarity::with_k1_b(f32::NAN, 0.75);
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
    assert!(err.unwrap_err().to_string().contains("illegal k1 value"));
  }

  Ok(())
}
#[test]
fn test_illegal_b() -> Result<()> {
  {
    let err = BM25Similarity::with_k1_b(1.2, 2.0);
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
    assert!(err.unwrap_err().to_string().contains("illegal b value"));
  }

  {
    let err = BM25Similarity::with_k1_b(1.2, -1.0);
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
    assert!(err.unwrap_err().to_string().contains("illegal b value"));
  }

  {
    let err = BM25Similarity::with_k1_b(1.2, f32::INFINITY);
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
    assert!(err.unwrap_err().to_string().contains("illegal b value"));
  }

  {
    let err = BM25Similarity::with_k1_b(1.2, f32::NAN);
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
    assert!(err.unwrap_err().to_string().contains("illegal b value"));
  }

  Ok(())
}
