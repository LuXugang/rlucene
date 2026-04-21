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
use crate::core::geo::circle::Circle;
use crate::core::geo::component_tree::{ComponentTree, component_tree_util};
use crate::core::geo::component2d::Component2DEnum2;
use crate::core::geo::component2d::{Component2D, WithinRelation};
use crate::core::geo::geometry::Geometry;
use crate::core::geo::line::Line;
use crate::core::geo::point::Point;
use crate::core::geo::polygon::Polygon;
use crate::core::geo::rectangle::Rectangle;
use crate::core::index::point_values::Relation;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::impl_from_for_enum;

/// Lat/Lon Geometry object.
pub trait LatLonGeometry: Geometry {}
pub fn create<T>(xy_geometries: &[T]) -> Result<LatLonGeometryType<T::Component2D>>
where
  T: LatLonGeometry,
{
  if xy_geometries.is_empty() {
    return Err(LuceneError::illegal_argument(
      "geometries must not be empty",
    ));
  }
  if xy_geometries.len() == 1 {
    return Ok(LatLonGeometryType::A(
      xy_geometries.iter().next().unwrap().to_component2d()?,
    ));
  }
  let mut components = Vec::with_capacity(xy_geometries.len());
  for geometry in xy_geometries {
    components.push(geometry.to_component2d()?);
  }
  Ok(LatLonGeometryType::B(component_tree_util::create(
    components,
  )?))
}
pub type LatLonGeometryType<T> = Component2DEnum2<T, ComponentTree<T>>;
#[macro_export]
macro_rules! either_lat_lon_geometry_named {
    (
        $vis:vis $name:ident,
        $component_name:ident {
            $( $Variant:ident : $T:ident ),+ $(,)?
        }
    ) => {
        $crate::either_component2d_named!(
            $vis $component_name {
                $( $Variant : $T ),+
            }
        );

        #[derive(PartialEq, Eq, Hash)]
        $vis enum $name<$( $T ),+> {
            $( $Variant($T), )+
        }

        impl<$( $T ),+> std::fmt::Display for $name<$( $T ),+>
        where
            $( $T: LatLonGeometry ),+
        {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $( Self::$Variant(inner) => std::fmt::Display::fmt(inner, f), )+
                }
            }
        }

        impl<$( $T ),+> Geometry for $name<$( $T ),+>
        where
            $( $T: LatLonGeometry ),+
        {
            type Component2D = $component_name<$( <$T as Geometry>::Component2D ),+>;

            fn to_component2d(&self) -> Result<Self::Component2D> {
                match self {
                    $( Self::$Variant(inner) => Ok(Self::Component2D::$Variant(inner.to_component2d()?)), )+
                }
            }
        }

        impl<$( $T ),+> LatLonGeometry for $name<$( $T ),+>
        where
            $( $T: LatLonGeometry ),+
        {}
    };
}

either_lat_lon_geometry_named!(
  pub LatLonGeometryEnum5,
  LatLonGeometryComponent2DEnum5 {
    Circle: A,
    Line: B,
    Point: C,
    Polygon: D,
    Rectangle: E,
  }
);
pub type LatLonGeometryEnum = LatLonGeometryEnum5<Circle, Line, Point, Polygon, Rectangle>;
impl_from_for_enum!(
LatLonGeometryEnum,
Circle => Circle,
Line => Line,
Point => Point,
Polygon => Polygon,
Rectangle => Rectangle,
);
impl Clone for LatLonGeometryEnum {
  fn clone(&self) -> Self {
    match self {
      LatLonGeometryEnum5::Circle(v) => LatLonGeometryEnum5::Circle(v.clone()),
      LatLonGeometryEnum5::Line(v) => LatLonGeometryEnum5::Line(v.clone()),
      LatLonGeometryEnum5::Point(v) => LatLonGeometryEnum5::Point(*v),
      LatLonGeometryEnum5::Polygon(v) => LatLonGeometryEnum5::Polygon(v.clone()),
      LatLonGeometryEnum5::Rectangle(v) => LatLonGeometryEnum5::Rectangle(v.clone()),
    }
  }
}
