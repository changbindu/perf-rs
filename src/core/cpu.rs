//! CPU detection and list parsing utilities.
//!
//! This module provides functions to:
//! - Detect online CPUs on the system
//! - Parse CPU list strings in various formats
//! - Validate CPU IDs

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::error::{PerfError, Result};

/// Path to the online CPUs file in sysfs.
const CPU_ONLINE_PATH: &str = "/sys/devices/system/cpu/online";

/// Returns the number of online CPUs.
pub fn get_cpu_count() -> Result<usize> {
    let cpus = get_online_cpus()?;
    Ok(cpus.len())
}

/// Returns a list of online CPU IDs.
pub fn get_online_cpus() -> Result<Vec<u32>> {
    let path = Path::new(CPU_ONLINE_PATH);
    let content = fs::read_to_string(path).map_err(|e| PerfError::FileRead {
        path: path.to_path_buf(),
        source: Box::new(e),
    })?;

    parse_sysfs_cpu_list(content.trim(), path)
}

fn parse_sysfs_cpu_list(s: &str, path: &Path) -> Result<Vec<u32>> {
    let mut cpus = Vec::new();

    if s.is_empty() {
        return Ok(cpus);
    }

    for entry in s.split(',') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }

        if entry.contains('-') {
            let parts: Vec<&str> = entry.split('-').collect();
            if parts.len() != 2 {
                return Err(PerfError::FileRead {
                    path: path.to_path_buf(),
                    source: Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("Invalid CPU range: {}", entry),
                    )),
                });
            }

            let start: u32 = parts[0].parse().map_err(|e| PerfError::FileRead {
                path: path.to_path_buf(),
                source: Box::new(e),
            })?;
            let end: u32 = parts[1].parse().map_err(|e| PerfError::FileRead {
                path: path.to_path_buf(),
                source: Box::new(e),
            })?;

            for cpu in start..=end {
                cpus.push(cpu);
            }
        } else {
            let cpu: u32 = entry.parse().map_err(|e| PerfError::FileRead {
                path: path.to_path_buf(),
                source: Box::new(e),
            })?;
            cpus.push(cpu);
        }
    }

    cpus.sort();
    cpus.dedup();
    Ok(cpus)
}

pub fn parse_cpu_list(input: &str) -> Result<Vec<u32>> {
    let input = input.trim();

    if input.is_empty() {
        return Err(PerfError::InvalidCpuList {
            message: "empty CPU list".to_string(),
        });
    }

    let mut cpus = HashSet::new();

    for part in input.split(',') {
        let part = part.trim();

        if part.is_empty() {
            return Err(PerfError::InvalidCpuList {
                message: "empty CPU identifier in list".to_string(),
            });
        }

        if part.contains('-') {
            let range_parts: Vec<&str> = part.split('-').collect();
            if range_parts.len() != 2 {
                return Err(PerfError::InvalidCpuList {
                    message: format!("invalid range format: '{}'", part),
                });
            }

            let start: u32 =
                range_parts[0]
                    .trim()
                    .parse()
                    .map_err(|_| PerfError::InvalidCpuList {
                        message: format!("invalid CPU number in range: '{}'", range_parts[0]),
                    })?;

            let end: u32 =
                range_parts[1]
                    .trim()
                    .parse()
                    .map_err(|_| PerfError::InvalidCpuList {
                        message: format!("invalid CPU number in range: '{}'", range_parts[1]),
                    })?;

            if start > end {
                return Err(PerfError::InvalidCpuList {
                    message: format!("invalid range: start ({}) > end ({})", start, end),
                });
            }

            for cpu in start..=end {
                cpus.insert(cpu);
            }
        } else {
            let cpu: u32 = part.parse().map_err(|_| PerfError::InvalidCpuList {
                message: format!("invalid CPU number: '{}'", part),
            })?;
            cpus.insert(cpu);
        }
    }

    let mut result: Vec<u32> = cpus.into_iter().collect();
    result.sort_unstable();
    Ok(result)
}

pub fn validate_cpu_ids(cpus: &[u32], max_cpu: u32) -> Result<()> {
    for &cpu_id in cpus {
        if cpu_id > max_cpu {
            return Err(PerfError::CpuOutOfRange { cpu_id, max_cpu });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_cpu() {
        assert_eq!(parse_cpu_list("0").unwrap(), vec![0]);
        assert_eq!(parse_cpu_list("5").unwrap(), vec![5]);
        assert_eq!(parse_cpu_list("127").unwrap(), vec![127]);
    }

    #[test]
    fn test_parse_cpu_list() {
        assert_eq!(parse_cpu_list("0,2,4").unwrap(), vec![0, 2, 4]);
        assert_eq!(parse_cpu_list("1,3,5,7").unwrap(), vec![1, 3, 5, 7]);
    }

    #[test]
    fn test_parse_cpu_range() {
        assert_eq!(parse_cpu_list("0-3").unwrap(), vec![0, 1, 2, 3]);
        assert_eq!(parse_cpu_list("5-7").unwrap(), vec![5, 6, 7]);
    }

    #[test]
    fn test_parse_mixed() {
        assert_eq!(
            parse_cpu_list("0-2,5,7-9").unwrap(),
            vec![0, 1, 2, 5, 7, 8, 9]
        );
        assert_eq!(parse_cpu_list("1,3-5,8").unwrap(), vec![1, 3, 4, 5, 8]);
    }

    #[test]
    fn test_parse_with_whitespace() {
        assert_eq!(parse_cpu_list(" 0 ").unwrap(), vec![0]);
        assert_eq!(parse_cpu_list("0, 2, 4").unwrap(), vec![0, 2, 4]);
        assert_eq!(parse_cpu_list(" 0 - 3 ").unwrap(), vec![0, 1, 2, 3]);
    }

    #[test]
    fn test_parse_deduplicates() {
        assert_eq!(parse_cpu_list("0,0,0").unwrap(), vec![0]);
        assert_eq!(parse_cpu_list("0-3,1,2").unwrap(), vec![0, 1, 2, 3]);
    }

    #[test]
    fn test_parse_sorts() {
        assert_eq!(parse_cpu_list("3,1,2").unwrap(), vec![1, 2, 3]);
        assert_eq!(parse_cpu_list("5-7,0-2").unwrap(), vec![0, 1, 2, 5, 6, 7]);
    }

    #[test]
    fn test_parse_empty_error() {
        assert!(matches!(
            parse_cpu_list(""),
            Err(PerfError::InvalidCpuList { .. })
        ));
        assert!(matches!(
            parse_cpu_list("   "),
            Err(PerfError::InvalidCpuList { .. })
        ));
    }

    #[test]
    fn test_parse_invalid_syntax_error() {
        assert!(matches!(
            parse_cpu_list("0--3"),
            Err(PerfError::InvalidCpuList { .. })
        ));
        assert!(matches!(
            parse_cpu_list("abc"),
            Err(PerfError::InvalidCpuList { .. })
        ));
        assert!(matches!(
            parse_cpu_list("0,,2"),
            Err(PerfError::InvalidCpuList { .. })
        ));
        assert!(matches!(
            parse_cpu_list("-1"),
            Err(PerfError::InvalidCpuList { .. })
        ));
    }

    #[test]
    fn test_parse_invalid_range_error() {
        assert!(matches!(
            parse_cpu_list("5-3"),
            Err(PerfError::InvalidCpuList { .. })
        ));
    }

    #[test]
    fn test_validate_valid_cpus() {
        assert!(validate_cpu_ids(&[0, 1, 2], 3).is_ok());
        assert!(validate_cpu_ids(&[0], 0).is_ok());
        assert!(validate_cpu_ids(&[], 0).is_ok());
    }

    #[test]
    fn test_validate_out_of_range() {
        assert!(matches!(
            validate_cpu_ids(&[0, 5], 3),
            Err(PerfError::CpuOutOfRange {
                cpu_id: 5,
                max_cpu: 3
            })
        ));
        assert!(matches!(
            validate_cpu_ids(&[10], 7),
            Err(PerfError::CpuOutOfRange {
                cpu_id: 10,
                max_cpu: 7
            })
        ));
    }

    #[test]
    fn test_get_cpu_count() {
        let result = get_cpu_count();
        assert!(result.is_ok());
        let count = result.unwrap();
        assert!(count > 0);
    }

    #[test]
    fn test_get_online_cpus() {
        let result = get_online_cpus();
        assert!(result.is_ok());
        let cpus = result.unwrap();
        assert!(!cpus.is_empty());
        for i in 1..cpus.len() {
            assert!(cpus[i] > cpus[i - 1]);
        }
    }
}
