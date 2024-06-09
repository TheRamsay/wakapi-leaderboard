use anyhow::{Ok, Result};
use chrono::Local;


pub fn get_current_datetime() -> Result<String> {
    if let Some(tz) = chrono::FixedOffset::east_opt(2 * 3600) {
        Ok(Local::now().with_timezone(&tz).format("%F %H:%M:%S").to_string())
    } else {
        Err(anyhow::anyhow!("Could not get timezone"))
    }

}