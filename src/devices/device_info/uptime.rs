use std::time::Instant;

/// Represents an uptime measurement at a moment in time. Enables querying at a later time.
#[derive(Debug, Clone)]
pub struct Uptime {
  /// The value of the uptime measurement in seconds
  measurement_value: u64,

  /// The instant this measurement was taken
  measurement_instant: Instant
}

impl Uptime {
  pub fn new(value: u64) -> Self {
    Self {
      measurement_value: value,
      measurement_instant: Instant::now()
    }
  }

  /// Gets the uptime measurement at the current instant
  pub fn seconds(&self) -> u64 {
    self.measurement_value + self.measurement_instant.elapsed().as_secs()
  }

  /// Pretty formats the uptime at the current instant
  pub fn pretty(&self) -> String {
    let value   = self.seconds();
    let seconds = value % 60;
    let minutes = (value / 60) % 60;
    let hours   = (value / (60 * 60)) % 24;
    let days    = (value / (60 * 60 * 24)) % 7;
    let weeks   = value / (60 * 60 * 24 * 7);

    let wk_substring  = if weeks != 0   { format!("{}w ",  weeks) }   else { "".into() };
    let day_substring = if days != 0    { format!("{}d ",  days) }    else { "".into() };
    let hr_substring  = if hours != 0   { format!("{}h ", hours) }   else { "".into() };
    let min_substring = if minutes != 0 { format!("{}m ", minutes) } else { "".into() };
    let sec_substring = if seconds != 0 { format!("{}s", seconds) } else { "".into() };

    format!(
      "{}{}{}{}{}", 
      wk_substring,
      day_substring,
      hr_substring,
      min_substring,
      sec_substring
    )
  }
}