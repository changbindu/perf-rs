//! Kernel symbol resolver using /proc/kallsyms.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::error::{PerfError, Result};

use super::{SymbolInfo, SymbolResolver};

/// Kernel symbol resolver that reads symbols from /proc/kallsyms.
pub struct KernelResolver {
    /// Symbol table indexed by start address.
    symbols: HashMap<u64, String>,
    /// Sorted addresses for O(log n) binary search lookups.
    sorted_addrs: Vec<u64>,
    /// Whether symbols are loaded.
    loaded: bool,
}

impl KernelResolver {
    /// Create a new kernel resolver.
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
            sorted_addrs: Vec::new(),
            loaded: false,
        }
    }

    /// Load kernel symbols from /proc/kallsyms.
    ///
    /// Format: `address type name [module]`
    /// Example: `ffffffff81000000 T _text`
    fn load_kallsyms(&mut self) -> Result<()> {
        let path = Path::new("/proc/kallsyms");
        let file = File::open(path).map_err(|e| PerfError::KernelSymbols {
            source: Box::new(e),
        })?;

        let reader = BufReader::new(file);
        let mut zero_addr_count = 0;

        for line in reader.lines() {
            let line = line.map_err(|e| PerfError::KernelSymbols {
                source: Box::new(e),
            })?;

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 3 {
                continue;
            }

            // Parse address (hex)
            let addr = match u64::from_str_radix(parts[0], 16) {
                Ok(a) => a,
                Err(_) => continue,
            };

            // Track zero addresses (may indicate permission issues)
            if addr == 0 {
                zero_addr_count += 1;
                continue;
            }

            // Symbol type (T, t, D, d, etc.)
            // We only care about text (code) symbols: T, t
            let sym_type = parts[1].chars().next().unwrap_or(' ');
            if !matches!(sym_type, 'T' | 't' | 'W' | 'w') {
                continue;
            }

            // Symbol name
            let name = parts[2].to_string();

            self.symbols.insert(addr, name);
        }

        // Warn if all addresses are zero (permission issue)
        if zero_addr_count > 0 && self.symbols.is_empty() {
            return Err(PerfError::PermissionDenied {
                operation: "read kernel symbols - kptr_restrict may be enabled".to_string(),
            });
        }

        // Build sorted address vector for O(log n) lookups
        self.sorted_addrs = self.symbols.keys().copied().collect();
        self.sorted_addrs.sort_unstable();

        self.loaded = true;
        Ok(())
    }

    /// Find the symbol that contains the given address.
    ///
    /// Uses binary search on pre-sorted symbol addresses.
    fn find_symbol_containing(&self, addr: u64) -> Option<(u64, &str)> {
        if self.sorted_addrs.is_empty() {
            return None;
        }

        // Binary search for the largest address <= target
        let idx = self.sorted_addrs.partition_point(|&a| a <= addr);

        if idx == 0 {
            return None;
        }

        let sym_addr = self.sorted_addrs[idx - 1];
        let sym_name = self.symbols.get(&sym_addr)?;

        Some((sym_addr, sym_name))
    }
}

impl Default for KernelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbolResolver for KernelResolver {
    fn resolve(&self, addr: u64) -> Result<Option<SymbolInfo>> {
        let (sym_addr, sym_name) = match self.find_symbol_containing(addr) {
            Some(result) => result,
            None => return Ok(None),
        };

        let info = SymbolInfo::new(sym_name.to_string(), sym_addr, 0);

        Ok(Some(info))
    }

    fn load_symbols(&mut self, _path: &Path) -> Result<()> {
        self.load_kallsyms()
    }

    fn clear(&mut self) {
        self.symbols.clear();
        self.sorted_addrs.clear();
        self.loaded = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kernel_resolver_new() {
        let resolver = KernelResolver::new();
        assert!(resolver.symbols.is_empty());
    }

    #[test]
    fn test_kernel_resolver_clear() {
        let mut resolver = KernelResolver::new();
        resolver.symbols.insert(0x1000, "test_symbol".to_string());
        resolver.loaded = true;

        resolver.clear();

        assert!(resolver.symbols.is_empty());
    }

    #[test]
    fn test_find_symbol_containing() {
        let mut resolver = KernelResolver::new();
        resolver.symbols.insert(0x1000, "sym1".to_string());
        resolver.symbols.insert(0x2000, "sym2".to_string());
        resolver.symbols.insert(0x3000, "sym3".to_string());
        resolver.sorted_addrs = vec![0x1000, 0x2000, 0x3000];

        // Exact match
        let result = resolver.find_symbol_containing(0x1000);
        assert!(result.is_some());
        let (addr, name) = result.unwrap();
        assert_eq!(addr, 0x1000);
        assert_eq!(name, "sym1");

        // Between symbols (should find the one before)
        let result = resolver.find_symbol_containing(0x1500);
        assert!(result.is_some());
        let (addr, _) = result.unwrap();
        assert_eq!(addr, 0x1000);

        // Above all symbols
        let result = resolver.find_symbol_containing(0x4000);
        assert!(result.is_some());
        let (addr, name) = result.unwrap();
        assert_eq!(addr, 0x3000);
        assert_eq!(name, "sym3");

        // Below all symbols
        let result = resolver.find_symbol_containing(0x100);
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_unloaded() {
        let resolver = KernelResolver::new();
        let result = resolver.resolve(0x1000).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_kernel_symbol_load() {
        let mut resolver = KernelResolver::new();

        // This test may fail if not root (kptr_restrict)
        // Just check that the function doesn't panic
        let result = resolver.load_symbols(Path::new(""));

        // Should either succeed or fail with permission denied
        match result {
            Ok(()) => {
                assert!(!resolver.symbols.is_empty());
            }
            Err(PerfError::PermissionDenied { .. }) => {
                // Expected on systems with kptr_restrict
            }
            Err(e) => {
                panic!("Unexpected error: {:?}", e);
            }
        }
    }
}
