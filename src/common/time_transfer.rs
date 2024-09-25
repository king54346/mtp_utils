use std::time::{SystemTime, UNIX_EPOCH};

pub fn system_time_to_string(time: SystemTime) -> String {
    match time.duration_since(UNIX_EPOCH) {
        Ok(duration) => format!("{}", duration.as_secs()), // 返回自 Unix 纪元以来的秒数
        Err(_) => "Invalid SystemTime".to_string(),
    }
}

pub fn string_to_system_time(s: &str) -> Result<SystemTime, String> {
    match s.parse::<u64>() {
        Ok(seconds) => Ok(UNIX_EPOCH + std::time::Duration::new(seconds, 0)),
        Err(_) => Err("Invalid string format".to_string()),
    }
}