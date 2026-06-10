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
use crate::core::geo::geo_encoding_utils::GeoEncodingUtils;
use crate::core::geo::rectangle::Rectangle;
use crate::core::index::point_values::{IntersectVisitor, PointTree, PointValues, Relation};
use crate::core::util::bit_util::BitUtil;
use crate::core::util::bits::Bits;
use crate::core::util::clone::TryClone;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::priority_queue::Compare;
use crate::core::util::{SloppyMath, ToInt};
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::fmt::{Display, Formatter};

#[allow(dead_code)] // for quick search
pub(crate) struct NearestNeighbor;
/// # Parameters
///
/// - `distance_sort_key`: The closest distance from a point in this cell to the query point,
///   computed as a sort key through [`SloppyMath::haversin_sort_key`]. Note that this is an
///   approximation to the closest distance, and there could be a point in the cell that is
///   closer.
#[derive(Clone)]
pub(crate) struct Cell<PT>
where
  PT: PointTree,
{
  index: PT,
  reader_index: i32,
  min_packed: Vec<u8>,
  max_packed: Vec<u8>,
  distance_sort_key: f64,
}

impl<PT> Cell<PT>
where
  PT: PointTree,
{
  pub(crate) fn new(
    index: PT,
    reader_index: i32,
    min_packed: Vec<u8>,
    max_packed: Vec<u8>,
    distance_sort_key: f64,
  ) -> Self {
    Self {
      index,
      reader_index,
      min_packed,
      max_packed,
      distance_sort_key,
    }
  }
}

impl<PT> PartialEq for Cell<PT>
where
  PT: PointTree,
{
  fn eq(&self, other: &Self) -> bool {
    self.distance_sort_key.to_bits() == other.distance_sort_key.to_bits()
  }
}

impl<PT> Eq for Cell<PT> where PT: PointTree {}

impl<PT> PartialOrd for Cell<PT>
where
  PT: PointTree,
{
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    Some(self.cmp(other))
  }
}

impl<PT> Ord for Cell<PT>
where
  PT: PointTree,
{
  fn cmp(&self, other: &Self) -> std::cmp::Ordering {
    // BinaryHeap pops greatest first. We want closest cells first (smallest distance_sort_key).
    // Reverse comparison so smaller distance_sort_key is "greater" and pops first.
    // This matches Java PriorityQueue natural ordering (closest cells explored first).
    other.distance_sort_key.total_cmp(&self.distance_sort_key)
  }
}
/// Holds one hit from [`NearestNeighbor::nearest`]
#[derive(Default, Clone)]
pub struct NearestHit {
  pub doc_id: i32,

  /// The distance from the hit to the query point, computed as a sort key through
  /// [`SloppyMath::haversin_sort_key`].
  pub distance_sort_key: f64,
}

impl Display for NearestHit {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "NearestHit(docID={} distanceSortKey={})",
      self.doc_id, self.distance_sort_key
    )
  }
}
impl PartialEq for NearestHit {
  fn eq(&self, other: &Self) -> bool {
    self.doc_id == other.doc_id
      && self.distance_sort_key.to_bits() == other.distance_sort_key.to_bits()
  }
}

impl Eq for NearestHit {}

impl PartialOrd for NearestHit {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for NearestHit {
  fn cmp(&self, other: &Self) -> Ordering {
    // BinaryHeap pops the greatest item first.
    // We want the heap top to be the current worst hit:
    // larger distance_sort_key is worse; if tied, larger doc_id is worse.
    match self.distance_sort_key.total_cmp(&other.distance_sort_key) {
      Ordering::Equal => self.doc_id.cmp(&other.doc_id),
      cmp => cmp,
    }
  }
}
struct NearestVisitor<'a, B>
where
  B: Bits + ?Sized,
{
  cur_doc_base: i32,
  cur_live_docs: Option<&'a B>,
  top_n: usize,
  hit_queue: &'a mut BinaryHeap<NearestHit>,
  point_lat: f64,
  point_lon: f64,
  set_bottom_counter: i32,

  min_lon: f64,
  max_lon: f64,
  min_lat: f64,
  max_lat: f64,

  // second set of longitude ranges to check (for cross-dateline case)
  min_lon2: f64,
}

impl<'a, B> NearestVisitor<'a, B>
where
  B: Bits + ?Sized,
{
  fn new(
    hit_queue: &'a mut BinaryHeap<NearestHit>,
    top_n: usize,
    point_lat: f64,
    point_lon: f64,
  ) -> Self {
    Self {
      cur_doc_base: 0,
      cur_live_docs: None,
      top_n,
      hit_queue,
      point_lat,
      point_lon,
      set_bottom_counter: 0,
      min_lon: f64::NEG_INFINITY,
      max_lon: f64::INFINITY,
      min_lat: f64::NEG_INFINITY,
      max_lat: f64::INFINITY,
      min_lon2: f64::INFINITY,
    }
  }

  fn maybe_update_bbox(&mut self) -> Result<()> {
    if self.set_bottom_counter < 1024 || (self.set_bottom_counter & 0x3F) == 0x3F {
      let hit = self
        .hit_queue
        .peek()
        .ok_or_else(|| LuceneError::unsupported_operation("hitQueue is empty"))?;
      let box_ = Rectangle::from_point_distance(
        self.point_lat,
        self.point_lon,
        SloppyMath::haversin_meters_from_sort_key(hit.distance_sort_key),
      )?;

      self.min_lat = box_.min_lat;
      self.max_lat = box_.max_lat;
      if box_.crosses_dateline() {
        // box1
        self.min_lon = f64::NEG_INFINITY;
        self.max_lon = box_.max_lon;
        // box2
        self.min_lon2 = box_.min_lon;
      } else {
        self.min_lon = box_.min_lon;
        self.max_lon = box_.max_lon;
        // disable box2
        self.min_lon2 = f64::INFINITY;
      }
    }

    self.set_bottom_counter += 1;
    Ok(())
  }
}

impl<B> IntersectVisitor for NearestVisitor<'_, B>
where
  B: Bits + ?Sized,
{
  fn visit(&mut self, _doc_id: i32) -> Result<()> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn visit_with_packed_value(&mut self, doc_id: i32, packed_value: &[u8]) -> Result<()> {
    if let Some(live_docs) = self.cur_live_docs
      && !live_docs.get(doc_id as usize)?
    {
      return Ok(());
    }

    let doc_latitude = GeoEncodingUtils::decode_latitude_from_bytes(packed_value, 0);
    let doc_longitude =
      GeoEncodingUtils::decode_longitude_from_bytes(packed_value, BitUtil::INT_BYTES);

    // test bounding box
    if doc_latitude < self.min_lat || doc_latitude > self.max_lat {
      return Ok(());
    }
    if (doc_longitude < self.min_lon || doc_longitude > self.max_lon)
      && doc_longitude < self.min_lon2
    {
      return Ok(());
    }

    // Use the haversin sort key when comparing hits, as it is faster to compute than the true
    // distance.
    let distance_sort_key =
      SloppyMath::haversin_sort_key(self.point_lat, self.point_lon, doc_latitude, doc_longitude);

    let full_doc_id = self.cur_doc_base + doc_id;

    if self.hit_queue.len() == self.top_n {
      // queue already full
      let hit = self
        .hit_queue
        .peek()
        .ok_or_else(|| LuceneError::unsupported_operation("hitQueue is empty"))?;

      // we don't collect docs in order here, so we must also test the tie-break case ourselves:
      if distance_sort_key.total_cmp(&hit.distance_sort_key).to_int() < 0
        || (distance_sort_key.total_cmp(&hit.distance_sort_key).to_int() == 0
          && full_doc_id < hit.doc_id)
      {
        let mut hit = self
          .hit_queue
          .pop()
          .ok_or_else(|| LuceneError::unsupported_operation("hitQueue is empty"))?;
        hit.doc_id = full_doc_id;
        hit.distance_sort_key = distance_sort_key;
        self.hit_queue.push(hit);
        self.maybe_update_bbox()?;
      }
    } else {
      let hit = NearestHit {
        doc_id: full_doc_id,
        distance_sort_key,
      };
      self.hit_queue.push(hit);
    }

    Ok(())
  }

  fn compare(&self, min_packed_value: &[u8], max_packed_value: &[u8]) -> Result<Relation> {
    let cell_min_lat = GeoEncodingUtils::decode_latitude_from_bytes(min_packed_value, 0);
    let cell_min_lon =
      GeoEncodingUtils::decode_longitude_from_bytes(min_packed_value, BitUtil::INT_BYTES);
    let cell_max_lat = GeoEncodingUtils::decode_latitude_from_bytes(max_packed_value, 0);
    let cell_max_lon =
      GeoEncodingUtils::decode_longitude_from_bytes(max_packed_value, BitUtil::INT_BYTES);

    if cell_max_lat < self.min_lat
      || self.max_lat < cell_min_lat
      || ((cell_max_lon < self.min_lon || self.max_lon < cell_min_lon)
        && cell_max_lon < self.min_lon2)
    {
      // this cell is outside our search bbox; don't bother exploring any more
      return Ok(Relation::CellOutsideQuery);
    }

    Ok(Relation::CellCrossesQuery)
  }
}
struct NearestHitCmp;
impl Compare<NearestHit> for NearestHitCmp {
  fn less_than(&self, a: &NearestHit, b: &NearestHit) -> Result<bool> {
    // sort by opposite distance_sort_key natural order
    match a.distance_sort_key.total_cmp(&b.distance_sort_key) {
      std::cmp::Ordering::Less => Ok(false),
      std::cmp::Ordering::Greater => Ok(true),
      std::cmp::Ordering::Equal => {
        // tie-break by higher doc_id
        Ok(a.doc_id < b.doc_id)
      },
    }
  }
}

pub fn nearest<PV, B>(
  point_lat: f64,
  point_lon: f64,
  readers: &[PV],
  live_docs: &[Option<B>],
  doc_bases: &[i32],
  n: usize,
) -> Result<Vec<NearestHit>>
where
  PV: PointValues,
  B: Bits,
{
  // Holds all cells, sorted by closest to the point:
  let mut cell_queue = BinaryHeap::new();

  let mut hit_queue = BinaryHeap::new();
  let mut visitor = NearestVisitor::new(&mut hit_queue, n, point_lat, point_lon);

  // Add root cell for each reader into the queue:
  for (i, reader) in readers.iter().enumerate() {
    let min_packed_value = reader
      .get_min_packed_value()?
      .ok_or_else(|| LuceneError::unsupported_operation("no points?"))?
      .into_owned();
    let max_packed_value = reader
      .get_max_packed_value()?
      .ok_or_else(|| LuceneError::unsupported_operation("no points?"))?
      .into_owned();
    let index_tree = reader.get_point_tree()?;
    let distance_sort_key = approx_best_distance_from_packed(
      min_packed_value.as_ref(),
      max_packed_value.as_ref(),
      point_lat,
      point_lon,
    );
    cell_queue.push(Cell::new(
      index_tree,
      i as i32,
      min_packed_value,
      max_packed_value,
      distance_sort_key,
    ));
  }

  while let Some(mut cell) = cell_queue.pop() {
    if visitor.compare(&cell.min_packed, &cell.max_packed)? == Relation::CellOutsideQuery {
      continue;
    }

    // TODO: if we replace approxBestDistance with actualBestDistance, we can put an opto here to
    // break once this "best" cell is fully outside of the hitQueue bottom's radius:

    if !cell.index.move_to_child()? {
      // Leaf block: visit all points and possibly collect them:
      visitor.cur_doc_base = doc_bases[cell.reader_index as usize];
      visitor.cur_live_docs = live_docs[cell.reader_index as usize].as_ref();
      cell.index.visit_doc_values(&mut visitor)?;
    } else {
      // Non-leaf block: split into two cells and put them back into the queue:

      // we must clone the index so that we can recurse left and right "concurrently":
      let new_index = cell.index.try_clone()?;

      let min_pv = new_index.get_min_packed_value()?.into_owned();
      let max_pv = new_index.get_max_packed_value()?.into_owned();
      let distance_sort_key =
        approx_best_distance_from_packed(min_pv.as_ref(), max_pv.as_ref(), point_lat, point_lon);
      cell_queue.push(Cell::new(
        new_index,
        cell.reader_index,
        min_pv,
        max_pv,
        distance_sort_key,
      ));

      // TODO: we are assuming a binary tree
      let move_to_sibling = cell.index.move_to_sibling()?;
      if move_to_sibling {
        let min_pv = cell.index.get_min_packed_value()?.into_owned();
        let max_pv = cell.index.get_max_packed_value()?.into_owned();
        let distance_sort_key =
          approx_best_distance_from_packed(min_pv.as_ref(), max_pv.as_ref(), point_lat, point_lon);
        cell_queue.push(Cell::new(
          cell.index,
          cell.reader_index,
          min_pv,
          max_pv,
          distance_sort_key,
        ));
      }
    }
  }

  let len = hit_queue.len();
  let mut hits = vec![NearestHit::default(); len];

  for i in (0..len).rev() {
    hits[i] = hit_queue
      .pop()
      .ok_or_else(|| LuceneError::unsupported_operation("hitQueue is empty"))?;
  }
  Ok(hits)
}

// NOTE: incoming args never cross the dateline, since they are a BKD cell
fn approx_best_distance_from_packed(
  min_packed_value: &[u8],
  max_packed_value: &[u8],
  point_lat: f64,
  point_lon: f64,
) -> f64 {
  let min_lat = GeoEncodingUtils::decode_latitude_from_bytes(min_packed_value, 0);
  let min_lon = GeoEncodingUtils::decode_longitude_from_bytes(min_packed_value, BitUtil::INT_BYTES);
  let max_lat = GeoEncodingUtils::decode_latitude_from_bytes(max_packed_value, 0);
  let max_lon = GeoEncodingUtils::decode_longitude_from_bytes(max_packed_value, BitUtil::INT_BYTES);
  approx_best_distance(min_lat, max_lat, min_lon, max_lon, point_lat, point_lon)
}
// NOTE: incoming args never cross the dateline, since they are a BKD cell
fn approx_best_distance(
  min_lat: f64,
  max_lat: f64,
  min_lon: f64,
  max_lon: f64,
  point_lat: f64,
  point_lon: f64,
) -> f64 {
  if point_lat >= min_lat && point_lat <= max_lat && point_lon >= min_lon && point_lon <= max_lon {
    // point is inside the cell!
    return 0.0;
  }

  let d1 = SloppyMath::haversin_sort_key(point_lat, point_lon, min_lat, min_lon);
  let d2 = SloppyMath::haversin_sort_key(point_lat, point_lon, min_lat, max_lon);
  let d3 = SloppyMath::haversin_sort_key(point_lat, point_lon, max_lat, max_lon);
  let d4 = SloppyMath::haversin_sort_key(point_lat, point_lon, max_lat, min_lon);
  d1.min(d2).min(d3.min(d4))
}
