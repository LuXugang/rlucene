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
use crate::store::directory::Directory;
use crate::util::bit_util::BitUtil;
use crate::util::bkd::bkd_config::BKDConfig;
use crate::util::bkd::heap_point_write::HeapPointWriter;
use crate::util::bkd::offline_point_write::OfflinePointWriter;
use crate::util::bkd::point_writer::PointWriterEnum;
use crate::util::error::lucene_error::LuceneError;
use std::cell::RefCell;
use std::rc::Rc;

/// Offline Radix selector for BKD tree.
pub struct BKDRadixSelector<D> {
    // histogram array
    histogram: Vec<i64>,
    // number of bytes to be sorted: config.bytesPerDim() + Integer.BYTES
    bytes_sorted: i32,
    // flag to when we are moving to sort on heap
    max_points_sort_in_heap: i32,
    // reusable buffer
    offline_buffer: Vec<u8>,
    // holder for partition points
    partition_bucket: Vec<i32>,
    // scratch array to hold temporary data
    scratch: Vec<u8>,
    // Directory to create new Offline writer
    temp_dir: Rc<RefCell<D>>,
    // prefix for temp files
    temp_file_name_prefix: String,
    // BKD tree configuration
    config: Rc<BKDConfig>,
}
impl<D> BKDRadixSelector<D>
where
    D: Directory,
{
    // size of the histogram
    const HISTOGRAM_SIZE: usize = 256;
    // size of the online buffer: 8 KB
    const MAX_SIZE_OFFLINE_BUFFER: usize = 1024 * 8;
    /// Sole constructor.
    pub fn new(
        config: Rc<BKDConfig>,
        max_points_sort_in_heap: i32,
        temp_dir: Rc<RefCell<D>>,
        temp_file_name_prefix: String,
    ) -> Self {
        // Selection and sorting is done in a given dimension. In case the value of the dimension are
        // equal
        // between two points we tie break first using the data-only dimensions and if those are still
        // equal
        // we tie-break on the docID. Here we account for all bytes used in the process.
        let bytes_sorted = config.get_bytes_per_dim()
            + (config.get_num_dims() - config.get_num_index_dims()) * config.get_bytes_per_dim()
            + BitUtil::INT_BYTES as i32;
        let number_of_points_offline =
            Self::MAX_SIZE_OFFLINE_BUFFER / config.bytes_per_doc() as usize;
        let offline_buffer = vec![0u8; number_of_points_offline * config.bytes_per_doc() as usize];
        let partition_bucket = vec![0; bytes_sorted as usize];
        let histogram = vec![0; Self::HISTOGRAM_SIZE];
        let scratch = vec![0u8; bytes_sorted as usize];
        BKDRadixSelector {
            config,
            max_points_sort_in_heap,
            temp_dir,
            temp_file_name_prefix,
            bytes_sorted,
            offline_buffer,
            partition_bucket,
            histogram,
            scratch,
        }
    }

    /// It uses the provided `points` from the given `from` to the given `to` to
    /// populate the `partitionSlices` array holder (length > 1) with two path slices so the
    /// path slice at position 0 contains `partition - from` points where the value of the `dim`
    /// is lower or equal to the `to - from` points on the slice at position 1.
    ///
    /// The `dimCommonPrefix` provides a hint for the length of the common prefix length for
    /// the `dim` where are partitioning the points.
    ///
    /// It return the value of the `dim` at the partition point.
    ///
    /// If the provided `points` is wrapping an `OfflinePointWriter`, the writer is
    /// destroyed in the process to save disk space.
    // pub fn select(
    //     &mut self,
    //     points: &mut PathSlice<D>,
    //     partition_slices: &mut [PathSlice<D>],
    //     from: i64,
    //     to: i64,
    //     partition_point: i64,
    //     dim: i32,
    //     dim_common_prefix: i32,
    // ) -> Result<Vec<u8>, LuceneError> {
    //     Self::check_args(from, to, partition_point)?;
    //     debug_assert!(
    //         partition_slices.len() > 1,
    //         "[partition slices] must be > 1, got {}",
    //         partition_slices.len()
    //     );
    //     match points.writer {
    //         PointWriterEnum::Heap(_) => {
    //             let partition = heap_radix_select(
    //                 points.writer.as_heap().unwrap(),
    //                 dim,
    //                 from as i32,
    //                 to as i32,
    //                 partition_point as i32,
    //                 dim_common_prefix,
    //             )?;
    //             partition_slices[0] = PathSlice::new(points.writer.clone(), from, partition_point - from);
    //             partition_slices[1] =
    //                 PathSlice::new(points.writer.clone(), partition_point, to - partition_point);
    //             return Ok(partition);
    //         }
    //         _=> {
    //         }
    //     };
    //     let offline_point_writer = points.writer.as_offline().unwrap();
    //     let left = get_point_writer(partition_point - from, format!("left{}", dim))?;
    //     let right = get_point_writer(to - partition_point, format!("right{}", dim))?;
    //     partition_slices[0] = PathSlice::new(left.clone(), 0, partition_point - from);
    //     partition_slices[1] = PathSlice::new(right.clone(), 0, to - partition_point);
    //     build_histogram_and_partition(
    //         offline_point_writer,
    //         left,
    //         right,
    //         from,
    //         to,
    //         partition_point,
    //         0,
    //         dim_common_prefix,
    //         dim,
    //     )
    // }
    //
    // fn check_args(from: i64, to: i64, partition_point: i64) -> Result<(), LuceneError> {
    //     if partition_point < from {
    //         return Err(LuceneError::IllegalArgument(
    //             "partitionPoint must be >= from".to_string(),
    //         ));
    //     }
    //     if partition_point >= to {
    //         return Err(LuceneError::IllegalArgument(
    //             "partitionPoint must be < to".to_string(),
    //         ));
    //     }
    //     Ok(())
    // }
    //
    // fn find_common_prefix_and_histogram(
    //     &mut self,
    //     points: &mut OfflinePointWriter,
    //     from: i64,
    //     to: i64,
    //     dim: i32,
    //     dim_common_prefix: i32,
    // ) -> Result<i32, LuceneError> {
    //     let mut common_prefix_position = self.bytes_sorted;
    //     let offset = dim * self.config.get_bytes_per_dim();
    //     let mut reader = points.get_reader(from, to - from, self.offline_buffer.clone())?;
    //     assert!(common_prefix_position > dim_common_prefix);
    //     reader.next()?;
    //     let mut point_value = reader.point_value();
    //     let mut packed_value_doc_id = point_value.packed_value_doc_id_bytes();
    //     // copy dimension
    //     packed_value_doc_id.bytes[packed_value_doc_id.offset + offset as usize..packed_value_doc_id.offset + offset as usize + self.config.get_bytes_per_dim() as usize]
    //         .copy_from_slice(&self.scratch[0..self.config.get_bytes_per_dim() as usize]);
    //     // copy data dimensions and docID
    //     packed_value_doc_id.bytes[packed_value_doc_id.offset + self.config.get_packed_index_bytes_length() as usize..packed_value_doc_id.offset + self.config.get_packed_index_bytes_length() as usize + ((self.config.get_num_dims() - self.config.get_num_index_dims()) * self.config.get_bytes_per_dim() + std::mem::size_of::<i32>()) as usize]
    //         .copy_from_slice(&self.scratch[self.config.get_bytes_per_dim() as usize..self.config.get_bytes_per_dim() as usize + ((self.config.get_num_dims() - self.config.get_num_index_dims()) * self.config.get_bytes_per_dim() + std::mem::size_of::<i32>()) as usize]);
    //     for i in (from + 1)..to {
    //         reader.next()?;
    //         point_value = reader.point_value();
    //         if common_prefix_position == dim_common_prefix {
    //             self.histogram[get_bucket(offset, common_prefix_position, &point_value)? as usize] += 1;
    //             for j in (i + 1)..to {
    //                 reader.next()?;
    //                 point_value = reader.point_value();
    //                 self.histogram[get_bucket(offset, common_prefix_position, &point_value)? as usize] += 1;
    //             }
    //             break;
    //         } else {
    //             let start_index = std::cmp::min(dim_common_prefix, self.config.get_bytes_per_dim());
    //             let end_index = std::cmp::min(common_prefix_position, self.config.get_bytes_per_dim());
    //             packed_value_doc_id = point_value.packed_value_doc_id_bytes();
    //             let j = arrays_mismatch(
    //                 &self.scratch,
    //                 start_index as usize,
    //                 end_index as usize,
    //                 &packed_value_doc_id.bytes,
    //                 packed_value_doc_id.offset + offset as usize + start_index as usize,
    //                 packed_value_doc_id.offset + offset as usize + end_index as usize,
    //             );
    //             if j == -1 {
    //                 if common_prefix_position > self.config.get_bytes_per_dim() {
    //                     let start_tie_break = self.config.get_packed_index_bytes_length();
    //                     let end_tie_break = start_tie_break + common_prefix_position - self.config.get_bytes_per_dim();
    //                     let k = arrays_mismatch(
    //                         &self.scratch,
    //                         self.config.get_bytes_per_dim() as usize,
    //                         common_prefix_position as usize,
    //                         &packed_value_doc_id.bytes,
    //                         packed_value_doc_id.offset + start_tie_break as usize,
    //                         packed_value_doc_id.offset + end_tie_break as usize,
    //                     );
    //                     if k != -1 {
    //                         common_prefix_position = self.config.get_bytes_per_dim() + k;
    //                         self.histogram.fill(0);
    //                         self.histogram[self.scratch[common_prefix_position as usize] as usize] = i - from;
    //                     }
    //                 }
    //             } else {
    //                 common_prefix_position = dim_common_prefix + j;
    //                 self.histogram.fill(0);
    //                 self.histogram[self.scratch[common_prefix_position as usize] as usize] = i - from;
    //             }
    //             if common_prefix_position != self.bytes_sorted {
    //                 self.histogram[get_bucket(offset, common_prefix_position, &point_value)? as usize] += 1;
    //             }
    //         }
    //     }
    //     for i in 0..common_prefix_position as usize {
    //         self.partition_bucket[i] = self.scratch[i] as i32 & 0xff;
    //     }
    //     Ok(common_prefix_position)
    // }

    fn get_delta_point_writer(
        &self,
        left: Rc<RefCell<PointWriterEnum<D>>>,
        right: Rc<RefCell<PointWriterEnum<D>>>,
        delta: i64,
        iteration: i32,
    ) -> Result<PointWriterEnum<D>, LuceneError> {
        if delta >= i32::MAX as i64 {
            return Err(LuceneError::integer_overflow(
                "Delta is too large".to_string(),
            ));
        }
        if delta <= self.get_max_points_sort_in_heap(left, right) as i64 {
            Ok(PointWriterEnum::Heap(HeapPointWriter::new(
                self.config.clone(),
                delta as i32,
            )))
        } else {
            Ok(PointWriterEnum::Offline(OfflinePointWriter::new(
                self.config.clone(),
                self.temp_dir.clone(),
                &self.temp_file_name_prefix,
                &format!("delta{}", iteration),
                delta,
            )?))
        }
    }

    fn get_max_points_sort_in_heap(
        &self,
        left: Rc<RefCell<PointWriterEnum<D>>>,
        right: Rc<RefCell<PointWriterEnum<D>>>,
    ) -> i32 {
        let mut points_used = 0;
        if let PointWriterEnum::Heap(ref heap_writer) = *left.borrow() {
            points_used += heap_writer.size;
        }
        if let PointWriterEnum::Heap(ref heap_writer) = *right.borrow() {
            points_used += heap_writer.size;
        }
        debug_assert!(self.max_points_sort_in_heap >= points_used);
        debug_assert!(self.max_points_sort_in_heap >= points_used);
        self.max_points_sort_in_heap - points_used
    }

    fn get_point_writer(&self, count: i64, desc: &str) -> Result<PointWriterEnum<D>, LuceneError> {
        // As we recurse, we hold two on-heap point writers at any point. Therefore the
        // max size for these objects is half of the total points we can have on-heap.
        if count <= self.max_points_sort_in_heap as i64 / 2 {
            let size = i32::try_from(count)
                .map_err(|_| LuceneError::integer_overflow("Count is too large".to_string()))?;
            Ok(PointWriterEnum::Heap(HeapPointWriter::new(
                self.config.clone(),
                size,
            )))
        } else {
            Ok(PointWriterEnum::Offline(OfflinePointWriter::new(
                self.config.clone(),
                self.temp_dir.clone(),
                &self.temp_file_name_prefix,
                desc,
                count,
            )?))
        }
    }
}

pub struct PathSlice<D>
where
    D: Directory,
{
    pub writer: Rc<RefCell<PointWriterEnum<D>>>,
    pub start: i64,
    pub count: i64,
}
impl<D> PathSlice<D>
where
    D: Directory,
{
    pub fn new(writer: Rc<RefCell<PointWriterEnum<D>>>, start: i64, count: i64) -> Self {
        PathSlice {
            writer,
            start,
            count,
        }
    }
}
