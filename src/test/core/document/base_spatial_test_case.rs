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
use crate::core::document::document::Document;
use crate::core::document::field::Store;
use crate::core::document::fields::Fields;
use crate::core::document::numeric_doc_values_field::NumericDocValuesField;
use crate::core::document::shape_field;
use crate::core::document::shape_field::{DecodedTriangle, DecodedTriangleType, QueryRelation};
use crate::core::document::string_field::StringField;
use crate::core::geo::component2d::{Component2D, WithinRelation};
use crate::core::geo::geo_utils::GeoUtils;
use crate::core::index::directory_reader;
use crate::core::index::index_reader::{
  IndexReader, IndexReaderContextKind, IndexReaderContextType,
};
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::indexable_field::IndexableField;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::multi_bits::get_live_docs;
use crate::core::index::multi_doc_values::MultiDocValues;
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::index::serial_merge_scheduler::SerialMergeScheduler;
use crate::core::index::term::Term;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::query::Query;
use crate::core::store::directory::{DirEnum, Directory};
use crate::core::util::bits::Bits;
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::core::util::io_utils::IOUtils;
use crate::test_framework::core::search::fixed_bit_set_collector::FixedBitSetCollector;
use crate::test_framework::core::util::lucene_test_case::{
  at_least, create_temp_dir_with_prefix, new_directory_shared, new_fs_directory,
  new_index_writer_config, new_searcher,
};
use rand::{Rng, RngExt};
use std::collections::HashSet;
use std::fmt::Debug;
use std::sync::Arc;

/// Name of the `LatLonShape` indexed field.
pub(crate) const FIELD_NAME: &str = "shape";

const QUERY_RELATIONS: [QueryRelation; 4] = [
  QueryRelation::Intersects,
  QueryRelation::Within,
  QueryRelation::Disjoint,
  QueryRelation::Contains,
];

const POINT_LINE_RELATIONS: [QueryRelation; 3] = [
  QueryRelation::Intersects,
  QueryRelation::Disjoint,
  QueryRelation::Contains,
];

/// Base test support for spherical and cartesian geometry indexing and search functionality.
pub trait BaseSpatialTestCase {
  type Shape: Clone + Debug;
  type Line: Clone;
  type Polygon: Clone;
  type Rectangle: Clone;
  type Point: Clone;
  type Circle: Clone;
  type Component2D: Component2D;
  type Encoder: Encoder;
  type Validator: Validator<Shape = Self::Shape, Encoder = Self::Encoder>;

  // A particularly tricky adversary for BKD tree:
  fn test_same_shape_many_times<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let num_shapes = if cfg!(feature = "nightly") {
      at_least(random, 50)
    } else {
      at_least(random, 3)
    } as usize;

    // Every doc has 2 points:
    let the_shape = self.next_shape(random)?;
    let shapes = vec![Some(the_shape); num_shapes];

    self.verify(random, &shapes)
  }

  // Force low cardinality leaves
  fn test_low_cardinality_shape_many_times<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let num_shapes = at_least(random, 20) as usize;
    let cardinality = random.random_range(2..=20);

    let mut diff_shapes = Vec::with_capacity(cardinality);
    for _ in 0..cardinality {
      diff_shapes.push(self.next_shape(random)?);
    }

    let mut shapes = Vec::with_capacity(num_shapes);
    for _ in 0..num_shapes {
      shapes.push(Some(
        diff_shapes[random.random_range(0..cardinality)].clone(),
      ));
    }

    self.verify(random, &shapes)
  }

  fn test_random_tiny<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    // Make sure single-leaf-node case is OK:
    self.do_test_random(random, 10)
  }

  fn test_random_medium<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let count = at_least(random, 20);
    self.do_test_random(random, count)
  }

  fn test_random_big<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    self.do_test_random(random, 20_000)
  }

  fn do_test_random<R>(&self, random: &mut R, count: i32) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let num_shapes = at_least(random, count) as usize;

    let mut shapes = Vec::with_capacity(num_shapes);
    for _id in 0..num_shapes {
      let x = random.random_range(0..=20);
      if x == 17 {
        shapes.push(None);
      } else {
        // Create a new shape.
        shapes.push(Some(self.next_shape(random)?));
      }
    }
    self.verify(random, &shapes)
  }

  fn get_shape_type(&self) -> &'static str;

  fn next_shape<R>(&self, random: &mut R) -> Result<Self::Shape>
  where
    R: Rng + ?Sized;

  fn get_encoder(&self) -> Self::Encoder;

  /// Creates the array of `LatLonShape::Triangle` values that are used to index the shape.
  fn create_indexable_fields(&self, field: &str, shape: &Self::Shape) -> Result<Vec<Fields>>;

  /// Adds a shape to a provided document.
  fn add_shape_to_doc(&self, field: &str, doc: &mut Document, shape: &Self::Shape) -> Result<()> {
    for field in self.create_indexable_fields(field, shape)? {
      doc.add(field);
    }
    Ok(())
  }

  /// Returns a semi-random line used for queries.
  fn next_line<R>(&self, random: &mut R) -> Result<Self::Line>
  where
    R: Rng + ?Sized;

  fn next_polygon<R>(&self, random: &mut R) -> Result<Self::Polygon>
  where
    R: Rng + ?Sized;

  fn random_query_box<R>(&self, random: &mut R) -> Result<Self::Rectangle>
  where
    R: Rng + ?Sized;

  fn next_points<R>(&self, random: &mut R) -> Result<Vec<Self::Point>>
  where
    R: Rng + ?Sized;

  fn next_circle<R>(&self, random: &mut R) -> Result<Self::Circle>
  where
    R: Rng + ?Sized;

  fn rect_min_x(&self, rect: &Self::Rectangle) -> f64;

  fn rect_max_x(&self, rect: &Self::Rectangle) -> f64;

  fn rect_min_y(&self, rect: &Self::Rectangle) -> f64;

  fn rect_max_y(&self, rect: &Self::Rectangle) -> f64;

  fn rect_crosses_dateline(&self, rect: &Self::Rectangle) -> bool;

  fn get_supported_query_relations(&self) -> &[QueryRelation] {
    &QUERY_RELATIONS
  }

  /// Returns a semi-random line used for queries.
  ///
  /// The `shapes` parameter may be used to ensure some queries intersect indexed shapes.
  fn random_query_line<R>(
    &self,
    random: &mut R,
    _shapes: &[Option<Self::Shape>],
  ) -> Result<Self::Line>
  where
    R: Rng + ?Sized,
  {
    self.next_line(random)
  }

  fn random_query_polygon<R>(&self, random: &mut R) -> Result<Self::Polygon>
  where
    R: Rng + ?Sized,
  {
    self.next_polygon(random)
  }

  fn random_query_circle<R>(&self, random: &mut R) -> Result<Self::Circle>
  where
    R: Rng + ?Sized,
  {
    self.next_circle(random)
  }

  /// Factory method to create a new bounding box query.
  fn new_rect_query(
    &self,
    field: &str,
    query_relation: QueryRelation,
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
  ) -> Result<Query>;

  /// Factory method to create a new line query.
  fn new_line_query(
    &self,
    field: &str,
    query_relation: QueryRelation,
    lines: Vec<Self::Line>,
  ) -> Result<Query>;

  /// Factory method to create a new polygon query.
  fn new_polygon_query(
    &self,
    field: &str,
    query_relation: QueryRelation,
    polygons: Vec<Self::Polygon>,
  ) -> Result<Query>;

  /// Factory method to create a new point query.
  fn new_points_query(
    &self,
    field: &str,
    query_relation: QueryRelation,
    points: Vec<Self::Point>,
  ) -> Result<Query>;

  /// Factory method to create a new distance query.
  fn new_distance_query(
    &self,
    field: &str,
    query_relation: QueryRelation,
    circle: Self::Circle,
  ) -> Result<Query>;

  fn to_line_2d(&self, lines: Vec<Self::Line>) -> Result<Self::Component2D>;

  fn to_polygon_2d(&self, polygons: Vec<Self::Polygon>) -> Result<Self::Component2D>;

  fn to_point_2d(&self, points: Vec<Self::Point>) -> Result<Self::Component2D>;

  fn to_circle_2d(&self, circle: Self::Circle) -> Result<Self::Component2D>;

  fn to_rectangle_2d(
    &self,
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
  ) -> Result<Self::Component2D>;

  fn verify<R>(&self, random: &mut R, shapes: &[Option<Self::Shape>]) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut iwc = new_index_writer_config(random)?;
    iwc.set_merge_scheduler(SerialMergeScheduler::new());
    let mbd = iwc.get_max_buffered_docs();
    if mbd != -1 && mbd < (shapes.len() / 100) as i32 {
      iwc.set_max_buffered_docs((shapes.len() / 100) as i32);
    }
    let dir = if shapes.len() > 1000 {
      new_fs_directory(
        random,
        create_temp_dir_with_prefix(std::any::type_name::<Self>())?,
      )?
    } else {
      new_directory_shared(random)?
    };
    let writer = IndexWriter::new(dir.clone(), iwc)?;

    // Index random polygons.
    self.index_random_shapes(random, &writer, shapes)?;

    // Query testing.
    let reader = Arc::new(directory_reader::open_from_writer(&writer)?);
    self.verify_random_queries(random, reader.clone(), shapes)?;

    let close_result = writer.close();
    let close_result = IOUtils::use_or_suppress_result(close_result, reader.close());
    IOUtils::use_or_suppress_result(close_result, dir.close())
  }

  fn index_random_shapes<R>(
    &self,
    random: &mut R,
    writer: &IndexWriter<DirEnum>,
    shapes: &[Option<Self::Shape>],
  ) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut deleted = HashSet::new();
    for (id, shape) in shapes.iter().enumerate() {
      let mut doc = Document::new();
      doc.add(StringField::from_string("id", id.to_string(), Store::No)?);
      doc.add(NumericDocValuesField::new("id", id as i64));
      if let Some(shape) = shape {
        self.add_shape_to_doc(FIELD_NAME, &mut doc, shape)?;
      }
      writer.add_document(doc)?;
      if id > 0 && random.random_range(0..100) == 42 {
        let id_to_delete = random.random_range(0..id);
        writer
          .delete_documents_with_terms(vec![Term::from_text("id", id_to_delete.to_string())])?;
        deleted.insert(id_to_delete);
      }
    }

    if random.random_bool(0.5) {
      writer.force_merge(1)?;
    }
    Ok(())
  }

  fn verify_random_queries<R, IR>(
    &self,
    random: &mut R,
    reader: Arc<IR>,
    shapes: &[Option<Self::Shape>],
  ) -> Result<()>
  where
    R: Rng + ?Sized,
    IR: IndexReader + Send + Sync + 'static,
    IR::ContextKind: IndexReaderContextKind<Arc<IR>>,
    IndexReaderContextType<Arc<IR>>: Sync,
  {
    let mut validator = self.get_validator()?;

    // Test random bbox queries.
    let searcher = new_searcher(random, reader.clone())?;
    self.verify_random_bbox_queries(random, &searcher, shapes, &mut validator)?;

    // Test random line queries.
    let searcher = new_searcher(random, reader.clone())?;
    self.verify_random_line_queries(random, &searcher, shapes, &mut validator)?;

    // Test random polygon queries.
    let searcher = new_searcher(random, reader.clone())?;
    self.verify_random_polygon_queries(random, &searcher, shapes, &mut validator)?;

    // Test random point queries.
    let searcher = new_searcher(random, reader.clone())?;
    self.verify_random_point_queries(random, &searcher, shapes, &mut validator)?;

    // Test random distance queries.
    let searcher = new_searcher(random, reader)?;
    self.verify_random_distance_queries(random, &searcher, shapes, &mut validator)
  }

  /// Tests random generated bounding boxes.
  fn verify_random_bbox_queries<R, IRC>(
    &self,
    random: &mut R,
    searcher: &IndexSearcher<IRC>,
    shapes: &[Option<Self::Shape>],
    validator: &mut Self::Validator,
  ) -> Result<()>
  where
    R: Rng + ?Sized,
    IRC: IndexReaderContext + Sync,
    IRC::IndexReader: Clone,
  {
    let iters = self.scaled_iteration_count(random, shapes.len());
    let live_docs = get_live_docs(searcher.get_index_reader().clone())?;
    let max_doc = searcher.get_index_reader().max_doc()?;

    for _iter in 0..iters {
      // BBox
      let rect = self.random_query_box(random)?;
      let relations = self.get_supported_query_relations();
      let query_relation = relations[random.random_range(0..relations.len())];
      let min_x = self.rect_min_x(&rect);
      let max_x = self.rect_max_x(&rect);
      let min_y = self.rect_min_y(&rect);
      let max_y = self.rect_max_y(&rect);
      let query = self.new_rect_query(FIELD_NAME, query_relation, min_x, max_x, min_y, max_y)?;

      let hits = self.search_index(searcher, query.clone(), max_doc)?;
      let mut doc_id_to_id =
        MultiDocValues::get_numeric_values(searcher.get_index_reader().clone(), "id")?
          .ok_or_else(|| LuceneError::illegal_state("id numeric doc values must exist"))?;
      for doc_id in 0..max_doc {
        assert_eq!(doc_id, doc_id_to_id.next_doc()?);
        let id = doc_id_to_id.long_value()? as usize;
        let is_live = match &live_docs {
          Some(live_docs) => live_docs.get(doc_id as usize)?,
          None => true,
        };
        let expected = if !is_live {
          false
        } else if let Some(shape) = &shapes[id] {
          if query_relation == QueryRelation::Contains && self.rect_crosses_dateline(&rect) {
            // For contains we need to call the validator for each section.
            // It is only expected if both sides are contained.
            let left = self.to_rectangle_2d(min_x, GeoUtils::MAX_LON_INCL, min_y, max_y)?;
            let right = self.to_rectangle_2d(GeoUtils::MIN_LON_INCL, max_x, min_y, max_y)?;
            validator.set_relation(query_relation);
            let left_matches = validator.test_component_query_with_shape(&left, shape)?;
            validator.set_relation(query_relation);
            left_matches && validator.test_component_query_with_shape(&right, shape)?
          } else {
            let component = self.to_rectangle_2d(min_x, max_x, min_y, max_y)?;
            validator.set_relation(query_relation);
            validator.test_component_query_with_shape(&component, shape)?
          }
        } else {
          false
        };

        assert_eq!(
          hits.get(doc_id as usize)?,
          expected,
          "wrong hit: id={id} relation={query_relation:?} query={query:?} docID={doc_id} shape={:?} deleted={}",
          shapes[id],
          !is_live
        );
      }
    }
    Ok(())
  }

  /// Tests random generated lines.
  fn verify_random_line_queries<R, IRC>(
    &self,
    random: &mut R,
    searcher: &IndexSearcher<IRC>,
    shapes: &[Option<Self::Shape>],
    validator: &mut Self::Validator,
  ) -> Result<()>
  where
    R: Rng + ?Sized,
    IRC: IndexReaderContext + Sync,
    IRC::IndexReader: Clone,
  {
    let iters = self.scaled_iteration_count(random, shapes.len());
    let live_docs = get_live_docs(searcher.get_index_reader().clone())?;
    let max_doc = searcher.get_index_reader().max_doc()?;

    for _iter in 0..iters {
      let query_line = self.random_query_line(random, shapes)?;
      let query_line_2d = self.to_line_2d(vec![query_line.clone()])?;
      let query_relation = POINT_LINE_RELATIONS[random.random_range(0..POINT_LINE_RELATIONS.len())];
      let query = self.new_line_query(FIELD_NAME, query_relation, vec![query_line.clone()])?;
      let hits = self.search_index(searcher, query.clone(), max_doc)?;

      let mut doc_id_to_id =
        MultiDocValues::get_numeric_values(searcher.get_index_reader().clone(), "id")?
          .ok_or_else(|| LuceneError::illegal_state("id numeric doc values must exist"))?;
      for doc_id in 0..max_doc {
        assert_eq!(doc_id, doc_id_to_id.next_doc()?);
        let id = doc_id_to_id.long_value()? as usize;
        let is_live = match &live_docs {
          Some(live_docs) => live_docs.get(doc_id as usize)?,
          None => true,
        };
        let expected = if !is_live {
          // Document is deleted.
          false
        } else if let Some(shape) = &shapes[id] {
          validator.set_relation(query_relation);
          validator.test_component_query_with_shape(&query_line_2d, shape)?
        } else {
          false
        };

        assert_eq!(
          hits.get(doc_id as usize)?,
          expected,
          "wrong hit: id={id} relation={query_relation:?} query={query:?} docID={doc_id} shape={:?} deleted={}",
          shapes[id],
          !is_live
        );
      }
    }
    Ok(())
  }

  /// Tests random generated polygons.
  fn verify_random_polygon_queries<R, IRC>(
    &self,
    random: &mut R,
    searcher: &IndexSearcher<IRC>,
    shapes: &[Option<Self::Shape>],
    validator: &mut Self::Validator,
  ) -> Result<()>
  where
    R: Rng + ?Sized,
    IRC: IndexReaderContext + Sync,
    IRC::IndexReader: Clone,
  {
    let iters = self.scaled_iteration_count(random, shapes.len());
    let live_docs = get_live_docs(searcher.get_index_reader().clone())?;
    let max_doc = searcher.get_index_reader().max_doc()?;

    for _iter in 0..iters {
      let query_polygon = self.random_query_polygon(random)?;
      let query_polygon_2d = self.to_polygon_2d(vec![query_polygon.clone()])?;
      let query_relation = QUERY_RELATIONS[random.random_range(0..QUERY_RELATIONS.len())];
      let query =
        self.new_polygon_query(FIELD_NAME, query_relation, vec![query_polygon.clone()])?;
      let hits = self.search_index(searcher, query.clone(), max_doc)?;

      let mut doc_id_to_id =
        MultiDocValues::get_numeric_values(searcher.get_index_reader().clone(), "id")?
          .ok_or_else(|| LuceneError::illegal_state("id numeric doc values must exist"))?;
      for doc_id in 0..max_doc {
        assert_eq!(doc_id, doc_id_to_id.next_doc()?);
        let id = doc_id_to_id.long_value()? as usize;
        let is_live = match &live_docs {
          Some(live_docs) => live_docs.get(doc_id as usize)?,
          None => true,
        };
        let expected = if !is_live {
          // Document is deleted.
          false
        } else if let Some(shape) = &shapes[id] {
          validator.set_relation(query_relation);
          validator.test_component_query_with_shape(&query_polygon_2d, shape)?
        } else {
          false
        };

        assert_eq!(
          hits.get(doc_id as usize)?,
          expected,
          "wrong hit: id={id} relation={query_relation:?} query={query:?} docID={doc_id} shape={:?} deleted={}",
          shapes[id],
          !is_live
        );
      }
    }
    Ok(())
  }

  /// Tests random generated point queries.
  fn verify_random_point_queries<R, IRC>(
    &self,
    random: &mut R,
    searcher: &IndexSearcher<IRC>,
    shapes: &[Option<Self::Shape>],
    validator: &mut Self::Validator,
  ) -> Result<()>
  where
    R: Rng + ?Sized,
    IRC: IndexReaderContext + Sync,
    IRC::IndexReader: Clone,
  {
    let iters = self.scaled_iteration_count(random, shapes.len());
    let live_docs = get_live_docs(searcher.get_index_reader().clone())?;
    let max_doc = searcher.get_index_reader().max_doc()?;

    for _iter in 0..iters {
      let query_points = self.next_points(random)?;
      let query_relation = QUERY_RELATIONS[random.random_range(0..QUERY_RELATIONS.len())];
      let points = if query_relation == QueryRelation::Contains {
        vec![
          query_points
            .first()
            .ok_or_else(|| LuceneError::illegal_state("next_points returned no points"))?
            .clone(),
        ]
      } else {
        query_points
      };
      let query_points_2d = self.to_point_2d(points.clone())?;
      let query = self.new_points_query(FIELD_NAME, query_relation, points)?;
      let hits = self.search_index(searcher, query.clone(), max_doc)?;

      let mut doc_id_to_id =
        MultiDocValues::get_numeric_values(searcher.get_index_reader().clone(), "id")?
          .ok_or_else(|| LuceneError::illegal_state("id numeric doc values must exist"))?;
      for doc_id in 0..max_doc {
        assert_eq!(doc_id, doc_id_to_id.next_doc()?);
        let id = doc_id_to_id.long_value()? as usize;
        let is_live = match &live_docs {
          Some(live_docs) => live_docs.get(doc_id as usize)?,
          None => true,
        };
        let expected = if !is_live {
          // Document is deleted.
          false
        } else if let Some(shape) = &shapes[id] {
          validator.set_relation(query_relation);
          validator.test_component_query_with_shape(&query_points_2d, shape)?
        } else {
          false
        };

        assert_eq!(
          hits.get(doc_id as usize)?,
          expected,
          "wrong hit: id={id} relation={query_relation:?} query={query:?} docID={doc_id} shape={:?} deleted={}",
          shapes[id],
          !is_live
        );
      }
    }
    Ok(())
  }

  /// Tests random generated circles.
  fn verify_random_distance_queries<R, IRC>(
    &self,
    random: &mut R,
    searcher: &IndexSearcher<IRC>,
    shapes: &[Option<Self::Shape>],
    validator: &mut Self::Validator,
  ) -> Result<()>
  where
    R: Rng + ?Sized,
    IRC: IndexReaderContext + Sync,
    IRC::IndexReader: Clone,
  {
    let iters = self.scaled_iteration_count(random, shapes.len());
    let live_docs = get_live_docs(searcher.get_index_reader().clone())?;
    let max_doc = searcher.get_index_reader().max_doc()?;

    for _iter in 0..iters {
      let query_circle = self.random_query_circle(random)?;
      let query_circle_2d = self.to_circle_2d(query_circle.clone())?;
      let query_relation = QUERY_RELATIONS[random.random_range(0..QUERY_RELATIONS.len())];
      let query = self.new_distance_query(FIELD_NAME, query_relation, query_circle)?;
      let hits = self.search_index(searcher, query.clone(), max_doc)?;

      let mut doc_id_to_id =
        MultiDocValues::get_numeric_values(searcher.get_index_reader().clone(), "id")?
          .ok_or_else(|| LuceneError::illegal_state("id numeric doc values must exist"))?;
      for doc_id in 0..max_doc {
        assert_eq!(doc_id, doc_id_to_id.next_doc()?);
        let id = doc_id_to_id.long_value()? as usize;
        let is_live = match &live_docs {
          Some(live_docs) => live_docs.get(doc_id as usize)?,
          None => true,
        };
        let expected = if !is_live {
          // Document is deleted.
          false
        } else if let Some(shape) = &shapes[id] {
          validator.set_relation(query_relation);
          validator.test_component_query_with_shape(&query_circle_2d, shape)?
        } else {
          false
        };

        assert_eq!(
          hits.get(doc_id as usize)?,
          expected,
          "wrong hit: id={id} relation={query_relation:?} query={query:?} docID={doc_id} shape={:?} deleted={}",
          shapes[id],
          !is_live
        );
      }
    }
    Ok(())
  }

  fn search_index<IRC>(
    &self,
    searcher: &IndexSearcher<IRC>,
    query: Query,
    max_doc: i32,
  ) -> Result<FixedBitSet>
  where
    IRC: IndexReaderContext + Sync,
  {
    searcher.search_with_collector_manager(query, &FixedBitSetCollector::create_manager(max_doc))
  }

  fn get_validator(&self) -> Result<Self::Validator>;

  fn scaled_iteration_count<R>(&self, random: &mut R, shapes: usize) -> i32
  where
    R: Rng + ?Sized,
  {
    if shapes < 500 {
      at_least(random, 50)
    } else if shapes < 5000 {
      at_least(random, 25)
    } else if shapes < 25_000 {
      at_least(random, 5)
    } else {
      at_least(random, 2)
    }
  }
}

pub trait Encoder {
  fn decode_x(&self, encoded: i32) -> f64;

  fn decode_y(&self, encoded: i32) -> f64;

  fn quantize_x(&self, raw: f64) -> f64;

  fn quantize_x_ceil(&self, raw: f64) -> f64;

  fn quantize_y(&self, raw: f64) -> f64;

  fn quantize_y_ceil(&self, raw: f64) -> f64;
}

/// Validator used to test query results against ground truth.
pub trait Validator {
  type Shape;
  type Encoder: Encoder;

  fn encoder(&self) -> &Self::Encoder;

  fn query_relation(&self) -> QueryRelation {
    QueryRelation::Intersects
  }

  fn set_relation(&mut self, relation: QueryRelation);

  fn test_component_query_with_shape(
    &self,
    query: &impl Component2D,
    shape: &Self::Shape,
  ) -> Result<bool>;

  fn test_component_query(&self, query: &impl Component2D, fields: &[Fields]) -> Result<bool> {
    let mut decoded_triangle = DecodedTriangle::default();

    for field in fields {
      let (intersects, contains) = match field.binary_value()? {
        Some(binary_value) => {
          shape_field::decode_triangle(&binary_value.as_ref().bytes, &mut decoded_triangle)?;

          match decoded_triangle.type_ {
            DecodedTriangleType::Point => {
              let y = self.encoder().decode_y(decoded_triangle.a_y);
              let x = self.encoder().decode_x(decoded_triangle.a_x);
              let intersects = query.contains(x, y);
              let contains = intersects;
              (intersects, contains)
            },
            DecodedTriangleType::Line => {
              let a_y = self.encoder().decode_y(decoded_triangle.a_y);
              let a_x = self.encoder().decode_x(decoded_triangle.a_x);
              let b_y = self.encoder().decode_y(decoded_triangle.b_y);
              let b_x = self.encoder().decode_x(decoded_triangle.b_x);
              let intersects = query.intersects_line_values(a_x, a_y, b_x, b_y);
              let contains = query.contains_line_values(a_x, a_y, b_x, b_y);
              (intersects, contains)
            },
            DecodedTriangleType::Triangle => {
              let a_y = self.encoder().decode_y(decoded_triangle.a_y);
              let a_x = self.encoder().decode_x(decoded_triangle.a_x);
              let b_y = self.encoder().decode_y(decoded_triangle.b_y);
              let b_x = self.encoder().decode_x(decoded_triangle.b_x);
              let c_y = self.encoder().decode_y(decoded_triangle.c_y);
              let c_x = self.encoder().decode_x(decoded_triangle.c_x);
              let intersects = query.intersects_triangle_values(a_x, a_y, b_x, b_y, c_x, c_y);
              let contains = query.contains_triangle_values(a_x, a_y, b_x, b_y, c_x, c_y);
              (intersects, contains)
            },
          }
        },
        None => {
          return Err(LuceneError::illegal_argument(
            "field.binary_value() is None",
          ));
        },
      };

      assert!((contains == intersects) || (!contains && intersects));

      match self.query_relation() {
        QueryRelation::Disjoint if intersects => return Ok(false),
        QueryRelation::Within if !contains => return Ok(false),
        QueryRelation::Intersects if intersects => return Ok(true),
        _ => {},
      }
    }

    Ok(!matches!(self.query_relation(), QueryRelation::Intersects))
  }

  fn test_within_query(
    &self,
    query: &impl Component2D,
    fields: &[Fields],
  ) -> Result<WithinRelation> {
    let mut answer = WithinRelation::Disjoint;
    let mut decoded_triangle = DecodedTriangle::default();

    for field in fields {
      let relation = match field.binary_value()? {
        Some(binary_value) => {
          shape_field::decode_triangle(&binary_value.as_ref().bytes, &mut decoded_triangle)?;

          match decoded_triangle.type_ {
            DecodedTriangleType::Point => {
              let y = self.encoder().decode_y(decoded_triangle.a_y);
              let x = self.encoder().decode_x(decoded_triangle.a_x);
              query.within_point(x, y)?
            },
            DecodedTriangleType::Line => {
              let a_y = self.encoder().decode_y(decoded_triangle.a_y);
              let a_x = self.encoder().decode_x(decoded_triangle.a_x);
              let b_y = self.encoder().decode_y(decoded_triangle.b_y);
              let b_x = self.encoder().decode_x(decoded_triangle.b_x);
              query.within_line_values(a_x, a_y, decoded_triangle.ab, b_x, b_y)?
            },
            DecodedTriangleType::Triangle => {
              let a_y = self.encoder().decode_y(decoded_triangle.a_y);
              let a_x = self.encoder().decode_x(decoded_triangle.a_x);
              let b_y = self.encoder().decode_y(decoded_triangle.b_y);
              let b_x = self.encoder().decode_x(decoded_triangle.b_x);
              let c_y = self.encoder().decode_y(decoded_triangle.c_y);
              let c_x = self.encoder().decode_x(decoded_triangle.c_x);
              query.within_triangle_values(
                a_x,
                a_y,
                decoded_triangle.ab,
                b_x,
                b_y,
                decoded_triangle.bc,
                c_x,
                c_y,
                decoded_triangle.ca,
              )?
            },
          }
        },
        None => {
          return Err(LuceneError::illegal_argument(
            "field.binary_value() is None",
          ));
        },
      };

      if relation == WithinRelation::NotWithin {
        return Ok(relation);
      } else if relation == WithinRelation::Candidate {
        answer = WithinRelation::Candidate;
      }
    }

    Ok(answer)
  }
}
