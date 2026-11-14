// SPDX-FileCopyrightText: 2026 Yanik Fuchs <yanik.fuchs@me.com>
// SPDX-License-Identifier: GPL-3.0-only

//! Sizes as the lists show them: always one decimal, so that they line up

use arrayvec::ArrayString;
use std::fmt::Write;

const UNITS: [&str; 6] = ["KiB", "MiB", "GiB", "TiB", "PiB", "EiB"];

/// A size in binary units with one decimal, e.g. "8.0 MiB", "688.4 MiB", "1.4 GiB"; bytes
/// below 1 KiB as they are ("512 B")
pub fn size_str(bytes: u64) -> ArrayString<10> {
    let mut s = ArrayString::new();
    if bytes < 1024 {
        let _ = write!(s, "{bytes} B");
        return s;
    }

    let mut value = bytes as f64 / 1024.0;
    let mut unit = 0;
    // 1023.96 would round to "1024.0", so switch to the next unit before that
    while value >= 1023.95 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    let _ = write!(s, "{value:.1} {}", UNITS[unit]);
    s
}

#[cfg(test)]
mod tests {
    use super::size_str;

    #[test]
    fn sizes_have_one_decimal() {
        assert_eq!(size_str(0).as_str(), "0 B");
        assert_eq!(size_str(1023).as_str(), "1023 B");
        assert_eq!(size_str(1024).as_str(), "1.0 KiB");
        assert_eq!(size_str(8 * 1024 * 1024).as_str(), "8.0 MiB");
        assert_eq!(size_str(721_420_288).as_str(), "688.0 MiB");
        assert_eq!(size_str(1_460_288_880).as_str(), "1.4 GiB");
        assert_eq!(size_str(1024 * 1024 * 1024 - 1).as_str(), "1.0 GiB");
        assert_eq!(size_str(u64::MAX).as_str(), "16.0 EiB");
    }
}
