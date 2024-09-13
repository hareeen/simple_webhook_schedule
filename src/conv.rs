use chrono::{Datelike, Timelike};
use chrono_tz::{Asia::Seoul, Tz};
use thiserror::Error;

#[derive(Debug, Error)]
enum TomlDatetimeConversionError {
    #[error("Missing field: {0}")]
    MissingField(&'static str),

    #[error("Field has invalid value: {0}={1}")]
    InvalidValue(&'static str, String),
}

serde_with::serde_conv!(
    pub TomlDatetimeAsChronoDateTimeSeoul,
    chrono::DateTime<Tz>,
    |dt: &chrono::DateTime<Tz>| toml::value::Datetime {
        date: Some(toml::value::Date {
            year: dt.year() as u16,
            month: dt.month() as u8,
            day: dt.day() as u8,
        }),
        time: Some(toml::value::Time {
            hour: dt.hour() as u8,
            minute: dt.minute() as u8,
            second: dt.second() as u8,
            nanosecond: dt.nanosecond() as u32,
        }),
        offset: None,
    },
    |value: toml::value::Datetime| -> Result<chrono::DateTime<Tz>, TomlDatetimeConversionError> {
        let date = value
            .date
            .ok_or(TomlDatetimeConversionError::MissingField("date"))?;
        let time = value
            .time
            .ok_or(TomlDatetimeConversionError::MissingField("time"))?;

        Ok(
            chrono::NaiveDate::from_ymd_opt(date.year as i32, date.month as u32, date.day as u32)
                .ok_or(TomlDatetimeConversionError::InvalidValue("date", format!("{}-{}-{}", date.year, date.month, date.day).to_owned()))?
                .and_hms_milli_opt(
                    time.hour as u32,
                    time.minute as u32,
                    time.second as u32,
                    time.nanosecond as u32,
                )
                .ok_or(TomlDatetimeConversionError::InvalidValue("time", format!("{}:{}:{}.{}", time.hour, time.minute, time.second, time.nanosecond).to_owned()))?
                .and_local_timezone(Seoul)
                .unwrap(),
        )
    }
);
