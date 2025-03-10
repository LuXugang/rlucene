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
use crate::util::bkd::point_reader::PointReader;
use crate::util::bkd::point_value::{PointValue, PointValueEnum};
use crate::util::bkd::point_writer::{PointWriter, PointWriterEnum};
use crate::util::error::lucene_error::LuceneError;
use crate::util::intro_sorter::IntroSorter;
use crate::util::radix_selector::{RadixSelector, RadixSelectorBase};
use crate::util::selector::Selector;
use crate::util::{
    CommonUtil, IntroSelector, IntroSelectorBase, IntroSelectorBaseDefault, MSBRadixSorterBase,
    Sorter, VecCopyOps,
};
use std::cell::RefCell;
use std::cmp::min;
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
    offline_buffer: Rc<RefCell<Vec<u8>>>,
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
        let offline_buffer = Rc::new(RefCell::new(vec![
            0u8;
            number_of_points_offline
                * config.bytes_per_doc() as usize
        ]));
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
    #[allow(clippy::too_many_arguments)]
    pub fn select(
        &mut self,
        points: &mut PathSlice<D>,
        partition_slices: &mut Vec<PathSlice<D>>,
        from: i64,
        to: i64,
        partition_point: i64,
        dim: i32,
        dim_common_prefix: i32,
    ) -> Result<Vec<u8>, LuceneError> {
        Self::check_args(from, to, partition_point)?;
        debug_assert!(
            partition_slices.len() <=1,
        );
        partition_slices.clear();
        let result;
        match &mut *points.writer.borrow_mut() {
            PointWriterEnum::Heap(_) => {
                let partition = self.heap_radix_select(
                    points.writer.clone(),
                    dim,
                    from as i32,
                    to as i32,
                    partition_point as i32,
                    dim_common_prefix,
                )?;
                partition_slices.push(
                    PathSlice::new(points.writer.clone(), from, partition_point - from));
                partition_slices.push(
                    PathSlice::new(points.writer.clone(), partition_point, to - partition_point));
                result = partition;
            }
            PointWriterEnum::Offline(offline_point_writer) => {
                let mut left =
                    self.get_point_writer(partition_point - from, &format!("left{}", dim))?;
                let mut right =
                    self.get_point_writer(to - partition_point, &format!("right{}", dim))?;
                let partition = self.build_histogram_and_partition(
                    offline_point_writer,
                    &mut left,
                    &mut right,
                    from,
                    to,
                    partition_point,
                    0,
                    dim_common_prefix,
                    dim,
                )?;
                left.close();
                right.close();
                partition_slices.push(
                    PathSlice::new(Rc::new(RefCell::new(left)), 0, partition_point - from));
                partition_slices.push(
                    PathSlice::new(Rc::new(RefCell::new(right)), 0, to - partition_point));
                result = partition;
            }
        };
        Ok(result)
    }

    fn check_args(from: i64, to: i64, partition_point: i64) -> Result<(), LuceneError> {
        if partition_point < from {
            return Err(LuceneError::illegal_argument(
                "partitionPoint must be >= from".to_string(),
            ));
        }
        if partition_point >= to {
            return Err(LuceneError::illegal_argument(
                "partitionPoint must be < to".to_string(),
            ));
        }
        Ok(())
    }

    fn find_common_prefix_and_histogram(
        &mut self,
        points: &mut OfflinePointWriter<D>,
        from: i64,
        to: i64,
        dim: i32,
        dim_common_prefix: i32,
    ) -> Result<i32, LuceneError> {
        let mut common_prefix_position = self.bytes_sorted;
        let offset = dim * self.config.get_bytes_per_dim();
        let mut reader =
            points.get_reader_with_buffer(from, to - from, self.offline_buffer.clone())?;
        debug_assert!(common_prefix_position > dim_common_prefix);
        reader.next()?;
        {
            let point_value_ref = reader.point_value();
            let point_value = point_value_ref.borrow();
            let value = point_value.get_value();
            let (packed_value_offset, length) = point_value.packed_value_doc_id_bytes();
            let mut start = (packed_value_offset + offset) as usize;
            let mut end = start + self.config.get_bytes_per_dim() as usize;
            self.scratch.copy_from(&value.borrow()[start..end], 0);
            start = (packed_value_offset + self.config.packed_index_bytes_length()) as usize;
            end = start
                + (self.config.get_num_dims()
                - self.config.get_num_index_dims() * self.config.get_bytes_per_dim())
                as usize
                + BitUtil::INT_BYTES;
            self.scratch.copy_from(
                &value.borrow()[start..end],
                self.config.bytes_per_dim as usize,
            );
        }
        let mut histogram_index;
        for i in (from + 1)..to {
            reader.next()?;
            if common_prefix_position == dim_common_prefix {
                let point_value_ref = reader.point_value();
                let point_value = point_value_ref.borrow();
                histogram_index =
                    self.get_bucket(offset, common_prefix_position, &point_value) as usize;
                self.histogram[histogram_index] += 1;
                for _ in (i + 1)..to {
                    reader.next()?;
                    let point_value_ref = reader.point_value();
                    let point_value = point_value_ref.borrow();
                    histogram_index =
                        self.get_bucket(offset, common_prefix_position, &point_value) as usize;
                    self.histogram[histogram_index] += 1;
                }
                break;
            } else {
                let point_value_ref = reader.point_value();
                let point_value = point_value_ref.borrow();
                // Check common prefix and adjust histogram
                let scratch_start_index =
                    min(dim_common_prefix, self.config.get_bytes_per_dim()) as usize;
                let scratch_end_index =
                    min(common_prefix_position, self.config.get_bytes_per_dim()) as usize;
                let (packed_value_offset, length) = point_value.packed_value_doc_id_bytes();
                let packed_value_start_index =
                    (packed_value_offset + offset) as usize + scratch_start_index;
                let packed_value_end_index =
                    (packed_value_offset + offset) as usize + scratch_end_index;
                let j = CommonUtil::miss_match(
                    &self.scratch[scratch_start_index..scratch_end_index],
                    &point_value.get_value().borrow()
                        [packed_value_start_index..packed_value_end_index],
                );
                if j == -1 {
                    if common_prefix_position > self.config.get_bytes_per_dim() {
                        let start_tie_break = self.config.packed_index_bytes_length();
                        let end_tie_break = start_tie_break + common_prefix_position
                            - self.config.get_bytes_per_dim();
                        let k = CommonUtil::miss_match(
                            &self.scratch[self.config.bytes_per_dim as usize
                                ..common_prefix_position as usize],
                            &point_value.get_value().borrow()[(packed_value_offset
                                + start_tie_break)
                                as usize
                                ..(packed_value_offset + end_tie_break) as usize],
                        );
                        if k != -1 {
                            common_prefix_position = self.config.get_bytes_per_dim() + k;
                            self.histogram.fill(0);
                            self.histogram
                                [self.scratch[common_prefix_position as usize] as usize] = i - from;
                        }
                    }
                } else {
                    common_prefix_position = dim_common_prefix + j;
                    self.histogram.fill(0);
                    self.histogram[self.scratch[common_prefix_position as usize] as usize] =
                        i - from;
                }
                if common_prefix_position != self.bytes_sorted {
                    histogram_index =
                        self.get_bucket(offset, common_prefix_position, &point_value) as usize;
                    self.histogram[histogram_index] += 1;
                }
            }
        }
        // Build partition buckets up to commonPrefix
        for i in 0..common_prefix_position as usize {
            self.partition_bucket[i] = self.scratch[i] as i32;
        }
        Ok(common_prefix_position)
    }

    fn get_bucket(
        &self,
        offset: i32,
        common_prefix_position: i32,
        point_value: &PointValueEnum,
    ) -> i32 {
        let packed_value = point_value.get_value();
        if common_prefix_position < self.config.bytes_per_dim {
            let (packed_value_offset, _length) = point_value.packed_value();
            let index = (packed_value_offset + offset + common_prefix_position) as usize;
            packed_value.borrow()[index] as i32
        } else {
            let (packed_value_offset, _length) = point_value.packed_value_doc_id_bytes();
            let index = (packed_value_offset
                + self.config.packed_index_bytes_length()
                + common_prefix_position
                - self.config.bytes_per_dim) as usize;
            packed_value.borrow()[index] as i32
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn build_histogram_and_partition(
        &mut self,
        points: &mut OfflinePointWriter<D>,
        left: &mut PointWriterEnum<D>,
        right: &mut PointWriterEnum<D>,
        from: i64,
        to: i64,
        partition_point: i64,
        iteration: i32,
        base_common_prefix: i32,
        dim: i32,
    ) -> Result<Vec<u8>, LuceneError> {
        // Find common prefix from baseCommonPrefix and build histogram
        let common_prefix =
            self.find_common_prefix_and_histogram(points, from, to, dim, base_common_prefix)?;
        // If all equals we just partition the points
        if common_prefix == self.bytes_sorted {
            self.offline_partition(
                points,
                left,
                right,
                None,
                from,
                to,
                dim,
                common_prefix - 1,
                partition_point,
            )?;
            return self.partition_point_from_common_prefix();
        }

        let mut left_count = 0i64;
        let mut right_count = 0i64;
        // Count left points and record the partition point
        for i in 0..Self::HISTOGRAM_SIZE {
            let size = self.histogram[i];
            if left_count + size > partition_point - from {
                self.partition_bucket[common_prefix as usize] = i as i32;
                break;
            }
            left_count += size;
        }
        // Count right points
        for i in (self.partition_bucket[common_prefix as usize] as usize + 1)..Self::HISTOGRAM_SIZE
        {
            right_count += self.histogram[i];
        }
        let delta = self.histogram[self.partition_bucket[common_prefix as usize] as usize];
        debug_assert_eq!(
            left_count + right_count + delta,
            to - from,
            "{} / {}",
            left_count + right_count + delta,
            to - from
        );
        // Special case when points are equal except last byte, we can just tie-break
        if common_prefix == self.bytes_sorted - 1 {
            let tie_break_count = partition_point - from - left_count;
            self.offline_partition(
                points,
                left,
                right,
                None,
                from,
                to,
                dim,
                common_prefix,
                tie_break_count,
            )?;
            return self.partition_point_from_common_prefix();
        }

        // Create the delta points writer
        let mut delta_points = self.get_delta_point_writer(left, right, delta, iteration)?;
        self.offline_partition(
            points,
            left,
            right,
            Some(&mut delta_points),
            from,
            to,
            dim,
            common_prefix,
            0,
        )?;
        delta_points.close();
        let new_partition_point = partition_point - from - left_count;

        // Depending on the concrete type of delta_points, call the appropriate partition method.
        let count = delta_points.count();
        match delta_points {
            PointWriterEnum::Heap(_) => self.heap_partition(
                delta_points,
                left,
                right,
                dim,
                0,
                count as i32,
                new_partition_point as i32,
                common_prefix + 1,
            ),
            PointWriterEnum::Offline(mut offline_writer) => self.build_histogram_and_partition(
                &mut offline_writer,
                left,
                right,
                0,
                count,
                new_partition_point,
                iteration + 1,
                common_prefix + 1,
                dim,
            ),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn offline_partition(
        &mut self,
        points: &mut OfflinePointWriter<D>,
        left: &mut PointWriterEnum<D>,
        right: &mut PointWriterEnum<D>,
        mut delta_points: Option<&mut PointWriterEnum<D>>,
        from: i64,
        to: i64,
        dim: i32,
        byte_position: i32,
        num_docs_tiebreak: i64,
    ) -> Result<(), LuceneError> {
        debug_assert!(byte_position == self.bytes_sorted - 1 || delta_points.is_some());
        let offset = dim * self.config.bytes_per_dim;
        let mut tiebreak_counter = 0i64;
        let mut reader =
            points.get_reader_with_buffer(from, to - from, self.offline_buffer.clone())?;
        while reader.next()? {
            let point_value_ref = reader.point_value();
            let point_value = point_value_ref.borrow();
            let bucket = self.get_bucket(offset, byte_position, &point_value);
            if bucket < self.partition_bucket[byte_position as usize] {
                left.append_point_value(&point_value)?;
            } else if bucket > self.partition_bucket[byte_position as usize] {
                right.append_point_value(&point_value)?;
            } else if byte_position == self.bytes_sorted - 1 {
                if tiebreak_counter < num_docs_tiebreak {
                    left.append_point_value(&point_value)?;
                    tiebreak_counter += 1;
                } else {
                    right.append_point_value(&point_value)?;
                }
            } else if let Some(dp) = delta_points.as_mut() {
                dp.append_point_value(&point_value)?;
            }
        }
        // Delete original file
        points.destroy()?;
        Ok(())
    }

    fn partition_point_from_common_prefix(&self) -> Result<Vec<u8>, LuceneError> {
        let mut partition = vec![0u8; self.config.bytes_per_dim as usize];
        for i in 0..self.config.bytes_per_dim as usize {
            partition[i] = self.partition_bucket[i] as u8;
        }
        Ok(partition)
    }

    #[allow(clippy::too_many_arguments)]
    fn heap_partition(
        &self,
        points: PointWriterEnum<D>,
        left: &mut PointWriterEnum<D>,
        right: &mut PointWriterEnum<D>,
        dim: i32,
        from: i32,
        to: i32,
        partition_point: i32,
        common_prefix: i32,
    ) -> Result<Vec<u8>, LuceneError> {
        let points = Rc::new(RefCell::new(points));
        let partition = self.heap_radix_select(
            points.clone(),
            dim,
            from,
            to,
            partition_point,
            common_prefix,
        )?;
        let mut points = points.borrow_mut();
        match &mut *points {
            PointWriterEnum::Heap(heap_writer) => {
                for i in from..to {
                    let value = heap_writer.get_packed_value_slice(i);
                    if i < partition_point {
                        left.append_point_value(&value.borrow())?;
                    } else {
                        right.append_point_value(&value.borrow())?;
                    }
                }
                Ok(partition)
            }
            _ => {
                debug_assert!(false, "Point writer is not a heap writer");
                Ok(vec![0u8; 0])
            }
        }
    }
    /// Sort the heap writer by the specified dim. It is used to sort the leaves of the tree/`.
    pub fn heap_radix_select(
        &self,
        points: Rc<RefCell<PointWriterEnum<D>>>,
        dim: i32,
        from: i32,
        to: i32,
        partition_point: i32,
        common_prefix_length: i32,
    ) -> Result<Vec<u8>, LuceneError> {
        let bytes_per_dim = self.config.bytes_per_dim;
        let dim_offset = dim * bytes_per_dim + common_prefix_length;
        let dim_cmp_bytes = bytes_per_dim - common_prefix_length;
        let data_offset = self.config.packed_index_bytes_length() - dim_cmp_bytes;
        let sub_selector = RadixSelectorImpl {
            points: points.clone(),
            common_prefix_length,
            dim_cmp_bytes,
            dim_offset,
            data_offset,
            dim,
            bytes_per_dim,
            bytes_sorted: self.bytes_sorted,
        };

        let mut radix_selector =
            RadixSelector::new(self.bytes_sorted - common_prefix_length, sub_selector);
        radix_selector.select(from, to, partition_point)?;

        let mut partition = vec![0u8; bytes_per_dim as usize];

        let mut points = points.borrow_mut();
        match &mut *points {
            PointWriterEnum::Heap(heap_writer) => {
                let point_value = heap_writer.get_packed_value_slice(partition_point);
                let (offset, _length) = point_value.borrow().packed_value();

                let start = (offset + (dim * bytes_per_dim)) as usize;
                let end = start + bytes_per_dim as usize;

                partition.copy_from(
                    &point_value.borrow().get_value().borrow().as_slice()[start..end],
                    0,
                );
                Ok(partition)
            }
            _ => Err(LuceneError::unreachable(
                "Point writer is not a heap writer".to_string(),
            )),
        }
    }

    /// Sort the heap writer by the specified dim. It is used to sort the leaves of the tree.
    pub fn heap_radix_sort(
        &self,
        points: Rc<RefCell<PointWriterEnum<D>>>,
        from: i32,
        to: i32,
        dim: i32,
        common_prefix_length: i32,
    ) -> Result<(), LuceneError> {
        let bytes_per_dim = self.config.bytes_per_dim;
        let dim_offset = dim * bytes_per_dim + common_prefix_length;
        let dim_cmp_bytes = bytes_per_dim - common_prefix_length;
        let data_offset = self.config.packed_index_bytes_length() - dim_cmp_bytes;
        let mut sorter = MSBRadixSorterImpl {
            points,
            dim_cmp_bytes,
            dim_offset,
            data_offset,
            common_prefix_length,
            dim,
            bytes_per_dim,
            bytes_sorted: self.bytes_sorted,
        };
        sorter.sort(from, to)
    }

    fn get_delta_point_writer(
        &self,
        left: &mut PointWriterEnum<D>,
        right: &mut PointWriterEnum<D>,
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
        left: &mut PointWriterEnum<D>,
        right: &mut PointWriterEnum<D>,
    ) -> i32 {
        let mut points_used = 0;
        if let PointWriterEnum::Heap(ref heap_writer) = left {
            points_used += heap_writer.size;
        }
        if let PointWriterEnum::Heap(ref heap_writer) = right {
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
/// Sliced reference to points in an PointWriter.
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

struct MSBRadixSorterImpl<D>
where
    D: Directory,
{
    points: Rc<RefCell<PointWriterEnum<D>>>,
    dim_cmp_bytes: i32,
    dim_offset: i32,
    data_offset: i32,
    common_prefix_length: i32,
    dim: i32,
    bytes_per_dim: i32,
    bytes_sorted: i32,
}

impl<D> Sorter for MSBRadixSorterImpl<D>
where
    D: Directory,
{
    fn swap(&mut self, i: i32, j: i32) -> Result<(), LuceneError> {
        let mut points = self.points.borrow_mut();
        match &mut *points {
            PointWriterEnum::Heap(heap_writer) => {
                heap_writer.swap(i, j);
                Ok(())
            }
            _ => {
                debug_assert!(false, "should not be here");
                Ok(())
            }
        }
    }
}

impl<D> MSBRadixSorterBase for MSBRadixSorterImpl<D>
where
    D: Directory,
{
    fn byte_at(&mut self, i: i32, k: i32) -> Result<i32, LuceneError> {
        debug_assert!(k >= 0, "negative prefix {}", k);
        let pos = if k < self.dim_cmp_bytes {
            self.dim_offset + k
        } else {
            self.data_offset + k
        };
        let mut points = self.points.borrow_mut();
        match &mut *points {
            PointWriterEnum::Heap(heap_writer) => Ok(heap_writer.byte_at(i, pos)),
            _ => {
                debug_assert!(false, "should not be here");
                Ok(0)
            }
        }
    }

    fn get_fallback_sorter(&mut self, k: i32, _length: i32) -> impl Sorter
    where
        Self: Sized,
    {
        let skyped_bytes = k + self.common_prefix_length;
        let dim_start = self.dim * self.bytes_per_dim;
        IntroSorterImpl {
            points: self.points.clone(),
            skyped_bytes,
            dim_start,
            scratch: vec![0u8; self.bytes_sorted as usize],
            bytes_per_dim: self.bytes_per_dim,
        }
    }
}

struct IntroSorterImpl<D>
where
    D: Directory,
{
    points: Rc<RefCell<PointWriterEnum<D>>>,
    skyped_bytes: i32,
    dim_start: i32,
    scratch: Vec<u8>,
    bytes_per_dim: i32,
}

impl<D> Sorter for IntroSorterImpl<D>
where
    D: Directory,
{
    fn compare(&mut self, i: i32, j: i32) -> Result<i32, LuceneError> {
        let points = self.points.borrow();
        match &*points {
            PointWriterEnum::Heap(heap_writer) => {
                if self.skyped_bytes < self.bytes_per_dim {
                    let cmp = heap_writer.compare_dim(i, j, self.dim_start);
                    if cmp != 0 {
                        return Ok(cmp);
                    }
                }
                Ok(heap_writer.compare_data_dims_and_doc(i, j))
            }
            _ => {
                debug_assert!(false, "should not be here");
                Ok(0)
            }
        }
    }

    fn swap(&mut self, i: i32, j: i32) -> Result<(), LuceneError> {
        let mut points = self.points.borrow_mut();
        match &mut *points {
            PointWriterEnum::Heap(heap_writer) => {
                heap_writer.swap(i, j);
                Ok(())
            }
            _ => {
                debug_assert!(false, "should not be here");
                Ok(())
            }
        }
    }

    fn set_pivot(&mut self, i: i32) -> Result<(), LuceneError> {
        let mut points = self.points.borrow_mut();
        match &mut *points {
            PointWriterEnum::Heap(heap_writer) => {
                if self.skyped_bytes < self.bytes_per_dim {
                    heap_writer.copy_dim(i, self.dim_start, &mut self.scratch, 0);
                }
                heap_writer.copy_data_dims_and_doc(
                    i,
                    &mut self.scratch,
                    self.bytes_per_dim as usize,
                );
                Ok(())
            }
            _ => {
                debug_assert!(false, "should not be here");
                Ok(())
            }
        }
    }

    //TODO: 回头这里将改成 if match
    fn compare_pivot(&mut self, j: i32) -> Result<i32, LuceneError> {
        let point = self.points.borrow();
        match &*point {
            PointWriterEnum::Heap(heap_writer) => {
                if self.skyped_bytes < self.bytes_per_dim {
                    let cmp =
                        heap_writer.compare_dim_with_scratch(j, &self.scratch, 0, self.dim_start);
                    if cmp != 0 {
                        return Ok(cmp);
                    }
                }
                Ok(heap_writer.compare_data_dims_and_doc_with(
                    j,
                    &self.scratch,
                    self.bytes_per_dim as usize,
                ))
            }
            _ => {
                debug_assert!(false, "should not be here");
                Ok(0)
            }
        }
    }
    fn sort(&mut self, from: i32, to: i32) -> Result<(), LuceneError> {
        IntroSorter::sort_range(self, from, to)?;
        Ok(())
    }
}

impl<D> IntroSorter for IntroSorterImpl<D> where D: Directory {}

struct RadixSelectorImpl<D>
where
    D: Directory,
{
    points: Rc<RefCell<PointWriterEnum<D>>>,
    common_prefix_length: i32,
    bytes_per_dim: i32,
    dim_cmp_bytes: i32,
    dim_offset: i32,
    data_offset: i32,
    dim: i32,
    bytes_sorted: i32,
}

impl<D> Selector for RadixSelectorImpl<D>
where
    D: Directory,
{
    fn swap(&mut self, i: i32, j: i32) -> Result<(), LuceneError> {
        let mut points = self.points.borrow_mut();
        match &mut *points {
            PointWriterEnum::Heap(heap_writer) => {
                heap_writer.swap(i, j);
                Ok(())
            }
            _ => {
                debug_assert!(false, "should not be here");
                Ok(())
            }
        }
    }
}

impl<D> RadixSelectorBase for RadixSelectorImpl<D>
where
    D: Directory,
{
    fn byte_at(&self, i: i32, k: i32) -> i32 {
        debug_assert!(k >= 0, "negative prefix {}", k);
        let pos = if k < self.dim_cmp_bytes {
            self.dim_offset + k
        } else {
            self.data_offset + k
        };
        let points = self.points.borrow();
        match &*points {
            PointWriterEnum::Heap(heap_writer) => heap_writer.byte_at(i, pos),
            _ => {
                debug_assert!(false, "should not be here");
                0
            }
        }
    }

    fn get_fallback_selector(&mut self, d: i32, _max_length: i32) -> impl Selector
    where
        Self: Sized,
    {
        let skyped_bytes = d + self.common_prefix_length;
        let dim_start = self.dim * self.bytes_per_dim;
        let sub_selector = IntroSelectorImpl {
            points: self.points.clone(),
            skyped_bytes,
            bytes_per_dim: self.bytes_per_dim,
            dim_start,
            scratch: vec![0u8; self.bytes_sorted as usize],
        };
        IntroSelector::new(sub_selector)
    }
}

struct IntroSelectorImpl<D>
where
    D: Directory,
{
    points: Rc<RefCell<PointWriterEnum<D>>>,
    skyped_bytes: i32,
    bytes_per_dim: i32,
    dim_start: i32,
    scratch: Vec<u8>,
}

impl<D> IntroSelectorBaseDefault for IntroSelectorImpl<D>
where
    D: Directory,
{
    fn set_pivot(&mut self, i: i32) {
        let mut points = self.points.borrow_mut();
        match &mut *points {
            PointWriterEnum::Heap(heap_writer) => {
                if self.skyped_bytes < self.bytes_per_dim {
                    heap_writer.copy_dim(i, self.dim_start, &mut self.scratch, 0);
                }
                heap_writer.copy_data_dims_and_doc(
                    i,
                    &mut self.scratch,
                    self.bytes_per_dim as usize,
                );
            }
            _ => {
                debug_assert!(false, "should not be here");
            }
        }
    }

    fn compare_pivot(&self, j: i32) -> i32 {
        let points = self.points.borrow();
        match &*points {
            PointWriterEnum::Heap(heap_writer) => {
                if self.skyped_bytes < self.bytes_per_dim {
                    let cmp =
                        heap_writer.compare_dim_with_scratch(j, &self.scratch, 0, self.dim_start);
                    if cmp != 0 {
                        return cmp;
                    }
                }
                heap_writer.compare_data_dims_and_doc_with(
                    j,
                    &self.scratch,
                    self.bytes_per_dim as usize,
                )
            }
            _ => {
                debug_assert!(false, "should not be here");
                0
            }
        }
    }
}

impl<D> Selector for IntroSelectorImpl<D>
where
    D: Directory,
{
    fn swap(&mut self, i: i32, j: i32) -> Result<(), LuceneError> {
        let mut points = self.points.borrow_mut();
        match &mut *points {
            PointWriterEnum::Heap(heap_writer) => {
                heap_writer.swap(i, j);
                Ok(())
            }
            _ => {
                debug_assert!(false, "should not be here");
                Ok(())
            }
        }
    }
}

impl<D> IntroSelectorBase for IntroSelectorImpl<D>
where
    D: Directory,
{
    fn compare(&mut self, i: i32, j: i32) -> i32 {
        let points = self.points.borrow();
        match &*points {
            PointWriterEnum::Heap(heap_writer) => {
                if self.skyped_bytes < self.bytes_per_dim {
                    let cmp = heap_writer.compare_dim(i, j, self.dim_start);
                    if cmp != 0 {
                        return cmp;
                    }
                }
                heap_writer.compare_data_dims_and_doc(i, j)
            }
            _ => {
                debug_assert!(false, "should not be here");
                0
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::store::directory::Directory;
    use crate::test::util::lucene_test_case::{new_directory, random};
    use crate::test::util::test_util::TestUtil;
    use crate::util::bit_util::BitUtil;
    use crate::util::bkd::bkd_config::BKDConfig;
    use crate::util::bkd::bkd_radix_selector::{BKDRadixSelector, PathSlice};
    use crate::util::bkd::heap_point_write::HeapPointWriter;
    use crate::util::bkd::offline_point_write::OfflinePointWriter;
    use crate::util::bkd::point_reader::PointReader;
    use crate::util::bkd::point_value::PointValue;
    use crate::util::bkd::point_writer::{PointWriter, PointWriterEnum};
    use crate::util::error::lucene_error::LuceneError;
    use crate::util::numeric_utils::NumericUtils;
    use crate::util::CommonUtil;
    use rand::rngs::StdRng;
    use rand::Rng;
    use std::cell::RefCell;
    use std::cmp::Ordering::{Greater, Less};
    use std::rc::Rc;
    

    #[test]
    fn test_basic() -> Result<(), LuceneError> {
        let mut random = random();
        let values = 4;
        let dir = Rc::new(RefCell::new(new_directory(&mut random)?));
        let middle = 2;
        let dimensions = 1;
        let bytes_per_dimensions = BitUtil::INT_BYTES;
        let config = Rc::new(BKDConfig::new(
            dimensions,
            dimensions,
            bytes_per_dimensions as i32,
            BKDConfig::DEFAULT_MAX_POINTS_IN_LEAF_NODE,
        )?);
        let mut points =
            get_random_point_writer(&mut random, config.clone(), dir.clone(), values as i64)?;
        let mut value = vec![0u8; config.packed_bytes_length() as usize];

        NumericUtils::int_to_sortable_bytes(1, &mut value, 0);
        points.append_bytes(&value, 0)?;

        NumericUtils::int_to_sortable_bytes(2, &mut value, 0);
        points.append_bytes(&value, 1)?;

        NumericUtils::int_to_sortable_bytes(3, &mut value, 0);
        points.append_bytes(&value, 2)?;

        NumericUtils::int_to_sortable_bytes(4, &mut value, 0);
        points.append_bytes(&value, 3)?;
        points.close();
        let mut copy = copy_points(&mut random, config.clone(), dir.clone(), &points)?;
        verify(
            &mut random,
            config,
            dir.clone(),
            &mut copy,
            0,
            values as i64,
            middle as i64,
            0,
        )?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn verify<D: Directory>(
        random: &mut StdRng,
        config: Rc<BKDConfig>,
        dir: Rc<RefCell<D>>,
        points: &mut PointWriterEnum<D>,
        start: i64,
        end: i64,
        middle: i64,
        sorted_on_heap: i32,
    ) -> Result<(), LuceneError> {
        let mut radix_selector = BKDRadixSelector::new(
            config.clone(),
            sorted_on_heap,
            dir.clone(),
            "test".to_string(),
        );
        let data_only_dims = config.num_dims - config.num_index_dims;

        for split_dim in 0..config.num_index_dims {
            let copy = copy_points(random, config.clone(), dir.clone(), points)?;
            let mut input_slice = PathSlice::new(Rc::new(RefCell::new(copy)), 0, points.count());

            let common_prefix_length_input =
                get_random_common_prefix(config.clone(), &input_slice, split_dim, random)?;

            let mut slices: Vec<PathSlice<D>> = Vec::with_capacity(2);
            let partition_point = radix_selector.select(
                &mut input_slice,
                &mut slices,
                start,
                end,
                middle,
                split_dim,
                common_prefix_length_input,
            )?;

            assert_eq!(
                slices[0].count,
                middle - start,
                "Left slice count does not match"
            );
            assert_eq!(
                slices[1].count,
                end - middle,
                "Right slice count does not match"
            );

            let max = get_max(config.clone(), &slices[0], split_dim)?;
            let min = get_min(config.clone(), &slices[1], split_dim)?;
            let cmp = compare_unsigned(
                &max,
                config.bytes_per_dim as usize,
                &min,
                config.bytes_per_dim as usize,
            );
            assert!(
                cmp <= 0,
                "Expected left slice max to be <= right slice min; got {}",
                cmp
            );

            if cmp == 0 {
                let max_data_dim =
                    get_max_data_dimension(config.clone(), &slices[0], &max, split_dim)?;
                let min_data_dim =
                    get_min_data_dimension(config.clone(), &slices[1], &min, split_dim)?;
                let cmp2 = compare_unsigned(
                    &max_data_dim,
                    (data_only_dims * config.bytes_per_dim) as usize,
                    &min_data_dim,
                    (data_only_dims * config.bytes_per_dim) as usize,
                );
                assert!(
                    cmp2 <= 0,
                    "Expected left slice data dims max <= right slice data dims min; got {}",
                    cmp2
                );
                if cmp2 == 0 {
                    let max_doc_id = get_max_doc_id(
                        config.clone(),
                        &slices[0],
                        split_dim,
                        &partition_point,
                        &max_data_dim,
                    )?;
                    let min_doc_id = get_min_doc_id(
                        config.clone(),
                        &slices[1],
                        split_dim,
                        &partition_point,
                        &min_data_dim,
                    )?;
                    assert!(
                        min_doc_id >= max_doc_id,
                        "Expected min docID {} to be >= max docID {}",
                        min_doc_id,
                        max_doc_id
                    );
                }
            }
            assert_eq!(
                partition_point, min,
                "Partition point does not equal the minimum of the right slice"
            );
            slices[0].writer.borrow_mut().destroy()?;
            slices[1].writer.borrow_mut().destroy()?;
        }
        points.destroy()?;
        Ok(())
    }

    fn compare_unsigned(a: &[u8], len_a: usize, b: &[u8], len_b: usize) -> i32 {
        use std::cmp::Ordering;
        match a[..len_a].cmp(&b[..len_b]) {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        }
    }

    fn copy_points<D: Directory>(
        random: &mut StdRng,
        config: Rc<BKDConfig>,
        dir: Rc<RefCell<D>>,
        points: &PointWriterEnum<D>,
    ) -> Result<PointWriterEnum<D>, LuceneError> {
        let mut copy = get_random_point_writer(random, config, dir, points.count())?;
        let count = points.count();
        let mut reader = points.get_reader(0, count)?;
        while reader.next()? {
            let point_value_ref = reader.point_value();
            let point_value = point_value_ref.borrow();
            copy.append_point_value(&point_value)?
        }
        copy.close();
        Ok(copy)
    }

    /// returns a common prefix length equal or lower than the current one.
    fn get_random_common_prefix<D: Directory>(
        config: Rc<BKDConfig>,
        input_slice: &PathSlice<D>,
        split_dim: i32,
        random: &mut StdRng,
    ) -> Result<i32, LuceneError> {
        let points_max = get_max(config.clone(), input_slice, split_dim)?;
        let points_min = get_min(config.clone(), input_slice, split_dim)?;
        let mut common_prefix_length = CommonUtil::miss_match(
            &points_max[0..config.bytes_per_dim as usize],
            &points_min[0..config.bytes_per_dim as usize],
        );
        if common_prefix_length == -1 {
            common_prefix_length = config.bytes_per_dim;
        }

        if random.random_bool(0.5) {
            Ok(common_prefix_length)
        } else if common_prefix_length == 0 {
            Ok(0)
        } else {
            Ok(random.random_range(0..common_prefix_length))
        }
    }

    fn get_random_point_writer<D: Directory>(
        random: &mut StdRng,
        config: Rc<BKDConfig>,
        dir: Rc<RefCell<D>>,
        num_points: i64,
    ) -> Result<PointWriterEnum<D>, LuceneError> {
        assert!(num_points <= i32::MAX as i64);
        if num_points < 4096 && random.random_bool(0.5) {
            Ok(PointWriterEnum::Heap(HeapPointWriter::new(
                config,
                num_points as i32,
            )))
        } else {
            Ok(PointWriterEnum::Offline(OfflinePointWriter::new(
                config, dir, "test", "test", num_points,
            )?))
        }
    }

    fn get_min<D: Directory>(
        config: Rc<BKDConfig>,
        path_slice: &PathSlice<D>,
        dimension: i32,
    ) -> Result<Vec<u8>, LuceneError> {
        let size = config.bytes_per_dim as usize;
        let mut min = vec![0xffu8; size];
        let mut reader = path_slice
            .writer
            .borrow_mut()
            .get_reader(path_slice.start, path_slice.count)?;
        let mut value = vec![0u8; size];
        while reader.next()? {
            let point_value_ref = reader.point_value();
            let point_value = point_value_ref.borrow_mut();
            let (packed_value_offset, _) = point_value.packed_value();
            let value_ref = point_value.get_value();
            let start_idx = (packed_value_offset + dimension * config.bytes_per_dim) as usize;
            let end_idx = start_idx + size;
            value.copy_from_slice(&value_ref.borrow()[start_idx..end_idx]);
            if min.cmp(&value) == Greater {
                min.copy_from_slice(&value);
            }
        }
        Ok(min)
    }

    fn get_min_doc_id<D: Directory>(
        config: Rc<BKDConfig>,
        p: &PathSlice<D>,
        dimension: i32,
        partition_point: &[u8],
        data_dim: &[u8],
    ) -> Result<i32, LuceneError> {
        let mut doc_id = i32::MAX;
        let mut reader = p.writer.borrow_mut().get_reader(p.start, p.count)?;
        while reader.next()? {
            let point_value_ref = reader.point_value();
            let point_value = point_value_ref.borrow_mut();
            let value = point_value.get_value();
            let (packed_value_offset, _) = point_value.packed_value();
            let offset = dimension * config.bytes_per_dim;
            let data_offset = config.packed_index_bytes_length();
            let data_length = (config.num_dims - config.num_index_dims) * config.bytes_per_dim;

            let value_ref = value.borrow();
            let dim_slice = &value_ref[(packed_value_offset + offset) as usize
                ..(packed_value_offset + offset + config.bytes_per_dim) as usize];
            let partition_slice = &partition_point[0..config.bytes_per_dim as usize];
            let data_slice = &value_ref[(packed_value_offset + data_offset) as usize
                ..(packed_value_offset + data_offset + data_length) as usize];
            let data_dim_slice = &data_dim[0..data_length as usize];

            if dim_slice == partition_slice && data_slice == data_dim_slice {
                let new_doc_id = point_value.doc_id(&value_ref);
                if new_doc_id < doc_id {
                    doc_id = new_doc_id;
                }
            }
        }
        Ok(doc_id)
    }

    fn get_min_data_dimension<D: Directory>(
        config: Rc<BKDConfig>,
        p: &PathSlice<D>,
        min_dim: &[u8],
        split_dim: i32,
    ) -> Result<Vec<u8>, LuceneError> {
        let num_data_dims = config.num_dims - config.num_index_dims;
        let size = (num_data_dims * config.bytes_per_dim) as usize;
        let mut min = vec![0xffu8; size];
        let offset = split_dim * config.bytes_per_dim;
        let mut reader = p.writer.borrow_mut().get_reader(p.start, p.count)?;
        let mut value = vec![0u8; size];
        while reader.next()? {
            let point_value_ref = reader.point_value();
            let point_value = point_value_ref.borrow_mut();
            let (packed_value_offset, _) = point_value.packed_value();
            let value_vec = point_value.get_value();
            let start_idx = (packed_value_offset + offset) as usize;
            let end_idx = (packed_value_offset + offset + config.bytes_per_dim) as usize;
            let dim_slice = &value_vec.borrow()[start_idx..end_idx];
            let min_dim_slice = &min_dim[0..config.bytes_per_dim as usize];
            if min_dim_slice.cmp(dim_slice) == Less {
                let copy_start =
                    (packed_value_offset + config.num_index_dims * config.bytes_per_dim) as usize;
                let copy_end = copy_start + size;
                value.copy_from_slice(&value_vec.borrow()[copy_start..copy_end]);
                if min_dim_slice.cmp(&value) == Greater {
                    min.copy_from_slice(&value);
                }
            }
        }
        Ok(min)
    }

    fn get_max<D: Directory>(
        config: Rc<BKDConfig>,
        p: &PathSlice<D>,
        dimension: i32,
    ) -> Result<Vec<u8>, LuceneError> {
        let size = config.bytes_per_dim as usize;
        let mut max = vec![0u8; size];
        let mut reader = p.writer.borrow_mut().get_reader(p.start, p.count)?;
        let mut value = vec![0u8; size];
        while reader.next()? {
            let point_value_ref = reader.point_value();
            let point_value = point_value_ref.borrow_mut();
            let (packed_value_offset, _) = point_value.packed_value();
            let bytes_ref = point_value.get_value();
            let start_idx = (packed_value_offset + dimension * config.bytes_per_dim) as usize;
            let end_idx = start_idx + size;
            value.copy_from_slice(&bytes_ref.borrow()[start_idx..end_idx]);
            if max.cmp(&value) == std::cmp::Ordering::Less {
                max.copy_from_slice(&value);
            }
        }
        Ok(max)
    }

    fn get_max_data_dimension<D: Directory>(
        config: Rc<BKDConfig>,
        p: &PathSlice<D>,
        max_dim: &[u8],
        split_dim: i32,
    ) -> Result<Vec<u8>, LuceneError> {
        let num_data_dims = config.num_dims - config.num_index_dims;
        let size = (num_data_dims * config.bytes_per_dim) as usize;
        let mut max = vec![0u8; size];
        let offset = split_dim * config.bytes_per_dim;
        let mut reader = p.writer.borrow_mut().get_reader(p.start, p.count)?;
        let mut value = vec![0u8; size];
        while reader.next()? {
            let point_value_ref = reader.point_value();
            let point_value = point_value_ref.borrow_mut();
            let (packed_value_offset, _) = point_value.packed_value();
            let value_vec = point_value.get_value();
            let start_idx = (packed_value_offset + offset) as usize;
            let end_idx = (packed_value_offset + offset + config.bytes_per_dim) as usize;
            let dim_slice = &value_vec.borrow()[start_idx..end_idx];
            let max_dim_slice = &max_dim[0..config.bytes_per_dim as usize];
            if dim_slice.cmp(max_dim_slice) == std::cmp::Ordering::Less {
                let copy_start =
                    (packed_value_offset + config.packed_index_bytes_length()) as usize;
                let copy_end = copy_start + size;
                value.copy_from_slice(&value_vec.borrow()[copy_start..copy_end]);
                if max.cmp(&value) == std::cmp::Ordering::Less {
                    max.copy_from_slice(&value);
                }
            }
        }
        Ok(max)
    }

    fn get_max_doc_id<D: Directory>(
        config: Rc<BKDConfig>,
        p: &PathSlice<D>,
        dimension: i32,
        partition_point: &[u8],
        data_dim: &[u8],
    ) -> Result<i32, LuceneError> {
        let mut doc_id = i32::MIN;
        let mut reader = p.writer.borrow_mut().get_reader(p.start, p.count)?;
        while reader.next()? {
            let point_value_ref = reader.point_value();
            let point_value = point_value_ref.borrow_mut();
            let value = point_value.get_value();
            let (packed_value_offset, _) = point_value.packed_value();
            let offset = dimension * config.bytes_per_dim;
            let data_offset = config.packed_index_bytes_length();
            let data_length = (config.num_dims - config.num_index_dims) * config.bytes_per_dim;

            let dim_slice = &value.borrow()[(packed_value_offset + offset) as usize
                ..(packed_value_offset + offset + config.bytes_per_dim) as usize];
            let partition_slice = &partition_point[0..config.bytes_per_dim as usize];

            let data_slice = &value.borrow()[(packed_value_offset + data_offset) as usize
                ..(packed_value_offset + data_offset + data_length) as usize];
            let data_dim_slice = &data_dim[0..data_length as usize];
            if dim_slice == partition_slice && data_slice == data_dim_slice {
                let new_doc_id = point_value.doc_id(&value.borrow());
                if new_doc_id > doc_id {
                    doc_id = new_doc_id;
                }
            }
        }
        Ok(doc_id)
    }

    fn get_random_config(random: &mut StdRng) -> Result<BKDConfig, LuceneError> {
        let num_index_dims = TestUtil::next_int(random, 1, BKDConfig::MAX_INDEX_DIMS);
        let num_dims = TestUtil::next_int(random, num_index_dims, BKDConfig::MAX_DIMS);
        let bytes_per_dim = TestUtil::next_int(random, 2, 30);
        let max_points_in_leaf_node = TestUtil::next_int(random, 50, 2000);
        BKDConfig::new(
            num_dims,
            num_index_dims,
            bytes_per_dim,
            max_points_in_leaf_node,
        )
    }
}
