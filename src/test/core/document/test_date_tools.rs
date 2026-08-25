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

use chrono::{DateTime, Datelike, NaiveDate, Timelike, Utc};

use crate::core::document::date_tools::{DateTools, Resolution};
use crate::core::util::error::lucene_error::{LuceneError, Result};

#[allow(dead_code)] // for quick search
struct TestDateTools;

#[test]
fn test_string_to_date() -> Result<()> {
  let mut d = DateTools::string_to_date("2004")?;
  assert_eq!("2004-01-01 00:00:00:000", iso_format(d));
  d = DateTools::string_to_date("20040705")?;
  assert_eq!("2004-07-05 00:00:00:000", iso_format(d));
  d = DateTools::string_to_date("200407050910")?;
  assert_eq!("2004-07-05 09:10:00:000", iso_format(d));
  d = DateTools::string_to_date("20040705091055990")?;
  assert_eq!("2004-07-05 09:10:55:990", iso_format(d));

  assert!(matches!(
    DateTools::string_to_date("97"),
    Err(LuceneError::IllegalArgument(_))
  ));
  assert!(matches!(
    DateTools::string_to_date("200401011235009999"),
    Err(LuceneError::IllegalArgument(_))
  ));
  assert!(matches!(
    DateTools::string_to_date("aaaa"),
    Err(LuceneError::IllegalArgument(_))
  ));
  Ok(())
}

#[test]
fn test_stringto_time() -> Result<()> {
  let mut time = DateTools::string_to_time("197001010000")?;
  let mut cal = utc_datetime(1970, 1, 1, 0, 0, 0, 0);
  assert_eq!(cal.timestamp_millis(), time);

  cal = utc_datetime(1980, 2, 2, 11, 5, 0, 0);
  time = DateTools::string_to_time("198002021105")?;
  assert_eq!(cal.timestamp_millis(), time);
  Ok(())
}

#[test]
fn test_date_and_timeto_string() -> Result<()> {
  let mut cal = utc_datetime(2004, 2, 3, 22, 8, 56, 333);

  let mut date_string = DateTools::date_to_string(cal, Resolution::YEAR)?;
  assert_eq!("2004", date_string);
  assert_eq!(
    "2004-01-01 00:00:00:000",
    iso_format(DateTools::string_to_date(&date_string)?)
  );

  date_string = DateTools::date_to_string(cal, Resolution::MONTH)?;
  assert_eq!("200402", date_string);
  assert_eq!(
    "2004-02-01 00:00:00:000",
    iso_format(DateTools::string_to_date(&date_string)?)
  );

  date_string = DateTools::date_to_string(cal, Resolution::DAY)?;
  assert_eq!("20040203", date_string);
  assert_eq!(
    "2004-02-03 00:00:00:000",
    iso_format(DateTools::string_to_date(&date_string)?)
  );

  date_string = DateTools::date_to_string(cal, Resolution::HOUR)?;
  assert_eq!("2004020322", date_string);
  assert_eq!(
    "2004-02-03 22:00:00:000",
    iso_format(DateTools::string_to_date(&date_string)?)
  );

  date_string = DateTools::date_to_string(cal, Resolution::MINUTE)?;
  assert_eq!("200402032208", date_string);
  assert_eq!(
    "2004-02-03 22:08:00:000",
    iso_format(DateTools::string_to_date(&date_string)?)
  );

  date_string = DateTools::date_to_string(cal, Resolution::SECOND)?;
  assert_eq!("20040203220856", date_string);
  assert_eq!(
    "2004-02-03 22:08:56:000",
    iso_format(DateTools::string_to_date(&date_string)?)
  );

  date_string = DateTools::date_to_string(cal, Resolution::MILLISECOND)?;
  assert_eq!("20040203220856333", date_string);
  assert_eq!(
    "2004-02-03 22:08:56:333",
    iso_format(DateTools::string_to_date(&date_string)?)
  );

  // Date before 1970.
  cal = utc_datetime(1961, 3, 5, 23, 9, 51, 444);
  date_string = DateTools::date_to_string(cal, Resolution::MILLISECOND)?;
  assert_eq!("19610305230951444", date_string);
  assert_eq!(
    "1961-03-05 23:09:51:444",
    iso_format(DateTools::string_to_date(&date_string)?)
  );

  date_string = DateTools::date_to_string(cal, Resolution::HOUR)?;
  assert_eq!("1961030523", date_string);
  assert_eq!(
    "1961-03-05 23:00:00:000",
    iso_format(DateTools::string_to_date(&date_string)?)
  );

  // timeToString.
  cal = utc_datetime(1970, 1, 1, 0, 0, 0, 0);
  date_string = DateTools::time_to_string(cal.timestamp_millis(), Resolution::MILLISECOND)?;
  assert_eq!("19700101000000000", date_string);

  cal = utc_datetime(1970, 1, 1, 1, 2, 3, 0);
  date_string = DateTools::time_to_string(cal.timestamp_millis(), Resolution::MILLISECOND)?;
  assert_eq!("19700101010203000", date_string);
  Ok(())
}

#[test]
fn test_round() -> Result<()> {
  let date = utc_datetime(2004, 2, 3, 22, 8, 56, 333);
  assert_eq!("2004-02-03 22:08:56:333", iso_format(date));

  let date_year = DateTools::round_date(date, Resolution::YEAR)?;
  assert_eq!("2004-01-01 00:00:00:000", iso_format(date_year));

  let date_month = DateTools::round_date(date, Resolution::MONTH)?;
  assert_eq!("2004-02-01 00:00:00:000", iso_format(date_month));

  let date_day = DateTools::round_date(date, Resolution::DAY)?;
  assert_eq!("2004-02-03 00:00:00:000", iso_format(date_day));

  let date_hour = DateTools::round_date(date, Resolution::HOUR)?;
  assert_eq!("2004-02-03 22:00:00:000", iso_format(date_hour));

  let date_minute = DateTools::round_date(date, Resolution::MINUTE)?;
  assert_eq!("2004-02-03 22:08:00:000", iso_format(date_minute));

  let date_second = DateTools::round_date(date, Resolution::SECOND)?;
  assert_eq!("2004-02-03 22:08:56:000", iso_format(date_second));

  let date_millisecond = DateTools::round_date(date, Resolution::MILLISECOND)?;
  assert_eq!("2004-02-03 22:08:56:333", iso_format(date_millisecond));

  // `i64` parameter.
  let date_year_long = DateTools::round(date.timestamp_millis(), Resolution::YEAR)?;
  assert_eq!(
    "2004-01-01 00:00:00:000",
    iso_format(date_time_from_millis(date_year_long))
  );

  let date_millisecond_long = DateTools::round(date.timestamp_millis(), Resolution::MILLISECOND)?;
  assert_eq!(
    "2004-02-03 22:08:56:333",
    iso_format(date_time_from_millis(date_millisecond_long))
  );
  Ok(())
}

#[test]
fn test_date_tools_utc() -> Result<()> {
  // Sun, 30 Oct 2005 00:00:00 +0000 -- the last second of 2005's DST in Europe/London.
  let time = 1_130_630_400i64;
  let d1 = DateTools::date_to_string(date_time_from_millis(time * 1000), Resolution::MINUTE)?;
  let d2 = DateTools::date_to_string(
    date_time_from_millis((time + 3600) * 1000),
    Resolution::MINUTE,
  )?;
  assert_ne!(d1, d2, "different times");
  assert_eq!(time * 1000, DateTools::string_to_time(&d1)?);
  assert_eq!((time + 3600) * 1000, DateTools::string_to_time(&d2)?);
  Ok(())
}

fn iso_format(date: DateTime<Utc>) -> String {
  format!(
    "{:04}-{:02}-{:02} {:02}:{:02}:{:02}:{:03}",
    date.year(),
    date.month(),
    date.day(),
    date.hour(),
    date.minute(),
    date.second(),
    date.timestamp_subsec_millis()
  )
}

fn utc_datetime(
  year: i32,
  month: u32,
  day: u32,
  hour: u32,
  minute: u32,
  second: u32,
  millisecond: u32,
) -> DateTime<Utc> {
  NaiveDate::from_ymd_opt(year, month, day)
    .unwrap()
    .and_hms_milli_opt(hour, minute, second, millisecond)
    .unwrap()
    .and_utc()
}

fn date_time_from_millis(time: i64) -> DateTime<Utc> {
  DateTime::<Utc>::from_timestamp_millis(time).unwrap()
}
