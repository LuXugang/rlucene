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
use crate::core::geo::polygon::Polygon;
use crate::core::geo::rectangle::Rectangle;
use crate::core::util::SloppyMath;
use crate::core::util::error::lucene_error::{LuceneError, Result};

pub struct EarthDebugger {
  b: String,
  next_shape: i32,
  finished: bool,
}

impl Default for EarthDebugger {
  fn default() -> Self {
    Self::new()
  }
}

impl EarthDebugger {
  pub fn new() -> Self {
    let mut b = String::new();
    b.push_str("<!DOCTYPE HTML>\n");
    b.push_str("<html>\n");
    b.push_str("  <head>\n");
    b.push_str("    <script src=\"http://www.webglearth.com/v2/api.js\"></script>\n");
    b.push_str("    <script>\n");
    b.push_str("      function initialize() {\n");
    b.push_str("        var earth = new WE.map('earth_div');\n");
    Self {
      b,
      next_shape: 0,
      finished: false,
    }
  }

  #[allow(dead_code)]
  pub fn with_center(center_lat: f64, center_lon: f64, altitude_meters: f64) -> Self {
    let mut b = String::new();
    b.push_str("<!DOCTYPE HTML>\n");
    b.push_str("<html>\n");
    b.push_str("  <head>\n");
    b.push_str("    <script src=\"http://www.webglearth.com/v2/api.js\"></script>\n");
    b.push_str("    <script>\n");
    b.push_str("      function initialize() {\n");
    b.push_str(&format!(
      "        var earth = new WE.map('earth_div', {{center: [{}, {}], altitude: {}}});\n",
      center_lat, center_lon, altitude_meters
    ));
    Self {
      b,
      next_shape: 0,
      finished: false,
    }
  }

  #[allow(dead_code)]
  pub fn add_polygon(&mut self, poly: &Polygon) {
    self.add_polygon_with_color(poly, "#00ff00");
  }

  #[allow(dead_code)]
  pub fn add_polygon_with_color(&mut self, poly: &Polygon, color: &str) {
    let name = format!("poly{}", self.next_shape);
    self.next_shape += 1;

    self
      .b
      .push_str(&format!("        var {} = WE.polygon([\n", name));
    let poly_lats = poly.get_poly_lats();
    let poly_lons = poly.get_poly_lons();
    for i in 0..poly_lats.len() {
      self.b.push_str(&format!(
        "          [{}, {}],\n",
        poly_lats[i], poly_lons[i]
      ));
    }
    self.b.push_str(&format!(
      "        ], {{color: '{}', fillColor: \"#000000\", fillOpacity: 0.0001}});\n",
      color
    ));
    self
      .b
      .push_str(&format!("        {}.addTo(earth);\n", name));

    for hole in poly.get_holes() {
      self.add_polygon_with_color(hole, "#ffffff");
    }
  }

  const MAX_KM_PER_STEP: f64 = 100.0;

  fn get_step_count(&self, min_lat: f64, max_lat: f64, min_lon: f64, max_lon: f64) -> i32 {
    let distance_meters = SloppyMath::haversin_meters(min_lat, min_lon, max_lat, max_lon);
    i32::max(
      1,
      ((distance_meters / 1000.0) / Self::MAX_KM_PER_STEP).round() as i32,
    )
  }

  fn draw_segment(&mut self, min_lat: f64, max_lat: f64, min_lon: f64, max_lon: f64) {
    let steps = self.get_step_count(min_lat, max_lat, min_lon, max_lon);
    for i in 0..steps {
      self.b.push_str(&format!(
        "          [{}, {}],\n",
        min_lat + (max_lat - min_lat) * i as f64 / steps as f64,
        min_lon + (max_lon - min_lon) * i as f64 / steps as f64
      ));
    }
  }

  pub fn add_rect(&mut self, min_lat: f64, max_lat: f64, min_lon: f64, max_lon: f64) {
    self.add_rect_with_color(min_lat, max_lat, min_lon, max_lon, "#ff0000");
  }

  pub fn add_rect_with_color(
    &mut self,
    min_lat: f64,
    max_lat: f64,
    min_lon: f64,
    max_lon: f64,
    color: &str,
  ) {
    let name = format!("rect{}", self.next_shape);
    self.next_shape += 1;

    self.b.push_str(&format!(
      "        // lat: {} TO {}; lon: {} TO {}\n",
      min_lat, max_lat, min_lon, max_lon
    ));
    self
      .b
      .push_str(&format!("        var {} = WE.polygon([\n", name));

    self.b.push_str("          // min -> max lat, min lon\n");
    self.draw_segment(min_lat, max_lat, min_lon, min_lon);

    self.b.push_str("          // max lat, min -> max lon\n");
    self.draw_segment(max_lat, max_lat, min_lon, max_lon);

    self.b.push_str("          // max -> min lat, max lon\n");
    self.draw_segment(max_lat, min_lat, max_lon, max_lon);

    self.b.push_str("          // min lat, max -> min lon\n");
    self.draw_segment(min_lat, min_lat, max_lon, min_lon);

    self.b.push_str("          // min lat, min lon\n");
    self
      .b
      .push_str(&format!("          [{}, {}]\n", min_lat, min_lon));
    self.b.push_str(&format!(
      "        ], {{color: \"{}\", fillColor: \"{}\"}});\n",
      color, color
    ));
    self
      .b
      .push_str(&format!("        {}.addTo(earth);\n", name));
  }

  pub fn add_lat_line(&mut self, lat: f64, min_lon: f64, max_lon: f64) {
    let name = format!("latline{}", self.next_shape);
    self.next_shape += 1;

    self
      .b
      .push_str(&format!("        var {} = WE.polygon([\n", name));
    let steps = self.get_step_count(lat, lat, min_lon, max_lon);
    let mut lon = min_lon;
    while lon <= max_lon {
      self.b.push_str(&format!("          [{}, {}],\n", lat, lon));
      lon += (max_lon - min_lon) / steps as f64;
    }
    self
      .b
      .push_str(&format!("          [{}, {}],\n", lat, max_lon));
    lon -= (max_lon - min_lon) / steps as f64;
    while lon >= min_lon {
      self.b.push_str(&format!("          [{}, {}],\n", lat, lon));
      lon -= (max_lon - min_lon) / steps as f64;
    }
    self.b.push_str(
            "        ], {color: \"#ff0000\", fillColor: \"#ffffff\", opacity: 1, fillOpacity: 0.0001});\n",
        );
    self
      .b
      .push_str(&format!("        {}.addTo(earth);\n", name));
  }

  #[allow(dead_code)]
  pub fn add_lon_line(&mut self, min_lat: f64, max_lat: f64, lon: f64) {
    let name = format!("lonline{}", self.next_shape);
    self.next_shape += 1;

    self
      .b
      .push_str(&format!("        var {} = WE.polygon([\n", name));
    let steps = self.get_step_count(min_lat, max_lat, lon, lon);
    let mut lat = min_lat;
    while lat <= max_lat {
      self.b.push_str(&format!("          [{}, {}],\n", lat, lon));
      lat += (max_lat - min_lat) / steps as f64;
    }
    self
      .b
      .push_str(&format!("          [{}, {}],\n", max_lat, lon));
    lat -= (max_lat - min_lat) / 36.0;
    while lat >= min_lat {
      self.b.push_str(&format!("          [{}, {}],\n", lat, lon));
      lat -= (max_lat - min_lat) / steps as f64;
    }
    self.b.push_str(
            "        ], {color: \"#ff0000\", fillColor: \"#ffffff\", opacity: 1, fillOpacity: 0.0001});\n",
        );
    self
      .b
      .push_str(&format!("        {}.addTo(earth);\n", name));
  }

  pub fn add_point(&mut self, lat: f64, lon: f64) {
    self.b.push_str(&format!(
      "        WE.marker([{}, {}]).addTo(earth);\n",
      lat, lon
    ));
  }

  pub fn add_circle(
    &mut self,
    center_lat: f64,
    center_lon: f64,
    radius_meters: f64,
    also_add_bbox: bool,
  ) -> Result<()> {
    self.add_point(center_lat, center_lon);
    let name = format!("circle{}", self.next_shape);
    self.next_shape += 1;
    self
      .b
      .push_str(&format!("        var {} = WE.polygon([\n", name));
    Self::inverse_haversin(&mut self.b, center_lat, center_lon, radius_meters);
    self
      .b
      .push_str("        ], {color: '#00ff00', fillColor: \"#000000\", fillOpacity: 0.0001 });\n");
    self
      .b
      .push_str(&format!("        {}.addTo(earth);\n", name));

    if also_add_bbox {
      let box_ = Rectangle::from_point_distance(center_lat, center_lon, radius_meters)?;
      self.add_rect(box_.min_lat, box_.max_lat, box_.min_lon, box_.max_lon);
      self.add_lat_line(
        Rectangle::axis_lat(center_lat, radius_meters),
        box_.min_lon,
        box_.max_lon,
      );
    }

    Ok(())
  }

  pub fn finish(&mut self) -> Result<String> {
    if self.finished {
      return Err(LuceneError::illegal_state("already finished"));
    }
    self.finished = true;
    self
      .b
      .push_str("        WE.tileLayer('http://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png',{\n");
    self
      .b
      .push_str("          attribution: '© OpenStreetMap contributors'\n");
    self.b.push_str("        }).addTo(earth);\n");
    self.b.push_str("      }\n");
    self.b.push_str("    </script>\n");
    self.b.push_str("    <style>\n");
    self
      .b
      .push_str("      html, body{padding: 0; margin: 0;}\n");
    self.b.push_str(
      "      #earth_div{top: 0; right: 0; bottom: 0; left: 0; position: absolute !important;}\n",
    );
    self.b.push_str("    </style>\n");
    self
      .b
      .push_str("    <title>WebGL Earth API: Hello World</title>\n");
    self.b.push_str("  </head>\n");
    self.b.push_str("  <body onload=\"initialize()\">\n");
    self.b.push_str("    <div id=\"earth_div\"></div>\n");
    self.b.push_str("  </body>\n");
    self.b.push_str("</html>\n");

    Ok(self.b.clone())
  }

  fn inverse_haversin(b: &mut String, center_lat: f64, center_lon: f64, radius_meters: f64) {
    let mut angle: f64 = 0.0;
    let steps = 100;

    'new_angle: while angle < 360.0 {
      let x = angle.to_radians().cos();
      let y = angle.to_radians().sin();
      let mut factor = 2.0;
      let mut step = 1.0;
      let mut last = 0;
      let mut last_distance_meters = 0.0;

      loop {
        let lat = Self::wrap_lat(center_lat + y * factor);
        let lon = Self::wrap_lon(center_lon + x * factor);
        let distance_meters = SloppyMath::haversin_meters(center_lat, center_lon, lat, lon);

        if last == 1 && distance_meters < last_distance_meters {
          angle += 360.0 / steps as f64;
          continue 'new_angle;
        }
        if last == -1 && distance_meters > last_distance_meters {
          angle += 360.0 / steps as f64;
          continue 'new_angle;
        }
        last_distance_meters = distance_meters;

        if (distance_meters - radius_meters).abs() < 0.1 {
          b.push_str(&format!("          [{}, {}],\n", lat, lon));
          break;
        }
        if distance_meters > radius_meters {
          factor -= step;
          if last == 1 {
            step /= 2.0;
          }
          last = -1;
        } else if distance_meters < radius_meters {
          factor += step;
          if last == -1 {
            step /= 2.0;
          }
          last = 1;
        }
      }

      angle += 360.0 / steps as f64;
    }
  }

  fn wrap_lat(lat: f64) -> f64 {
    if lat > 90.0 {
      180.0 - lat
    } else if lat < -90.0 {
      -180.0 - lat
    } else {
      lat
    }
  }

  fn wrap_lon(lon: f64) -> f64 {
    if lon > 180.0 {
      lon - 360.0
    } else if lon < -180.0 {
      lon + 360.0
    } else {
      lon
    }
  }
}
