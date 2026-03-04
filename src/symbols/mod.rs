//! Symbol resolution module.
//!
//! This module provides symbol resolution capabilities for ELF binaries, DWARF debug info,
//! and kernel symbols from /proc/kallsyms.

mod elf;
mod kernel;

pub use elf::ElfResolver;
pub use kernel::KernelResolver;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::Result;

/// Symbol information returned by symbol resolvers.
#[derive(Debug, Clone, PartialEq)]
pub struct SymbolInfo {
    /// Symbol name (e.g., function name).
    pub name: String,
    /// Start address of the symbol.
    pub start_addr: u64,
    /// Size of the symbol in bytes (0 if unknown).
    pub size: u64,
    /// Source file path (if available from DWARF).
    pub source_file: Option<String>,
    /// Line number in source file (if available).
    pub line: Option<u32>,
}

impl SymbolInfo {
    /// Create a new symbol info with minimal information.
    pub fn new(name: String, start_addr: u64, size: u64) -> Self {
        Self {
            name,
            start_addr,
            size,
            source_file: None,
            line: None,
        }
    }

    /// Create a symbol info with source location.
    pub fn with_source(mut self, file: String, line: u32) -> Self {
        self.source_file = Some(file);
        self.line = Some(line);
        self
    }

    /// Check if an address falls within this symbol's range.
    pub fn contains(&self, addr: u64) -> bool {
        if self.size == 0 {
            // If size is unknown, exact match only
            addr == self.start_addr
        } else {
            addr >= self.start_addr && addr < self.start_addr + self.size
        }
    }
}

/// Trait for symbol resolvers.
///
/// Implementations provide address-to-symbol resolution for different
/// symbol sources (ELF files, kernel symbols, etc.).
pub trait SymbolResolver {
    /// Resolve an address to a symbol.
    ///
    /// Returns `Ok(Some(info))` if a symbol is found at or containing the address.
    /// Returns `Ok(None)` if no symbol found.
    /// Returns `Err` on failure to access symbol data.
    fn resolve(&self, addr: u64) -> Result<Option<SymbolInfo>>;

    /// Load symbols from a file.
    ///
    /// For ELF files, this loads both ELF symbols and DWARF debug info.
    /// For kernel symbols, use `load_kernel_symbols()` instead.
    fn load_symbols(&mut self, path: &Path) -> Result<()>;

    /// Check if symbols are loaded.
    fn is_loaded(&self) -> bool;

    /// Clear all loaded symbols.
    fn clear(&mut self);
}

/// Multi-source symbol resolver that combines multiple resolvers.
///
/// This resolver tries multiple symbol sources in order until finding a match.
pub struct MultiResolver {
    /// ELF resolvers keyed by path.
    elf_resolvers: HashMap<PathBuf, ElfResolver>,
    /// Kernel symbol resolver.
    kernel_resolver: Option<KernelResolver>,
    /// Whether kernel symbols are loaded.
    kernel_loaded: bool,
}

impl MultiResolver {
    /// Create a new multi-resolver.
    pub fn new() -> Self {
        Self {
            elf_resolvers: HashMap::new(),
            kernel_resolver: None,
            kernel_loaded: false,
        }
    }

    /// Load kernel symbols from /proc/kallsyms.
    pub fn load_kernel_symbols(&mut self) -> Result<()> {
        let mut resolver = KernelResolver::new();
        // KernelResolver ignores the path argument
        resolver.load_symbols(Path::new(""))?;
        self.kernel_resolver = Some(resolver);
        self.kernel_loaded = true;
        Ok(())
    }

    /// Get or load an ELF resolver for a path.
    pub fn get_or_load_elf(&mut self, path: &Path) -> Result<&ElfResolver> {
        if !self.elf_resolvers.contains_key(path) {
            let mut resolver = ElfResolver::new();
            resolver.load_symbols(path)?;
            self.elf_resolvers.insert(path.to_path_buf(), resolver);
        }
        Ok(self.elf_resolvers.get(path).unwrap())
    }

    /// Resolve address in kernel space.
    pub fn resolve_kernel(&self, addr: u64) -> Result<Option<SymbolInfo>> {
        if let Some(ref resolver) = self.kernel_resolver {
            resolver.resolve(addr)
        } else {
            Ok(None)
        }
    }

    /// Resolve address in a specific ELF file.
    pub fn resolve_elf(&self, path: &Path, addr: u64) -> Result<Option<SymbolInfo>> {
        if let Some(resolver) = self.elf_resolvers.get(path) {
            resolver.resolve(addr)
        } else {
            Ok(None)
        }
    }
}

impl Default for MultiResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbolResolver for MultiResolver {
    fn resolve(&self, addr: u64) -> Result<Option<SymbolInfo>> {
        // Try kernel symbols first (typically high addresses)
        if let Some(ref resolver) = self.kernel_resolver {
            if let Some(info) = resolver.resolve(addr)? {
                return Ok(Some(info));
            }
        }

        // Try ELF resolvers in order (this is simplified; in practice,
        // you'd need to know which ELF to query based on memory maps)
        for resolver in self.elf_resolvers.values() {
            if let Some(info) = resolver.resolve(addr)? {
                return Ok(Some(info));
            }
        }

        Ok(None)
    }

    fn load_symbols(&mut self, path: &Path) -> Result<()> {
        // For MultiResolver, load_symbols loads ELF symbols
        self.get_or_load_elf(path)?;
        Ok(())
    }

    fn is_loaded(&self) -> bool {
        !self.elf_resolvers.is_empty() || self.kernel_loaded
    }

    fn clear(&mut self) {
        self.elf_resolvers.clear();
        self.kernel_resolver = None;
        self.kernel_loaded = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_info_contains() {
        let sym = SymbolInfo::new("test_func".to_string(), 0x1000, 0x100);

        assert!(sym.contains(0x1000));
        assert!(sym.contains(0x1050));
        assert!(sym.contains(0x10FF));
        assert!(!sym.contains(0x1100));
        assert!(!sym.contains(0x0FFF));
    }

    #[test]
    fn test_symbol_info_contains_zero_size() {
        let sym = SymbolInfo::new("test_func".to_string(), 0x1000, 0);

        // With zero size, only exact match works
        assert!(sym.contains(0x1000));
        assert!(!sym.contains(0x1001));
    }

    #[test]
    fn test_symbol_info_with_source() {
        let sym = SymbolInfo::new("test_func".to_string(), 0x1000, 0x100)
            .with_source("test.c".to_string(), 42);

        assert_eq!(sym.source_file, Some("test.c".to_string()));
        assert_eq!(sym.line, Some(42));
    }

    #[test]
    fn test_multi_resolver_new() {
        let resolver = MultiResolver::new();
        assert!(!resolver.is_loaded());
    }

    #[test]
    fn test_multi_resolver_clear() {
        let mut resolver = MultiResolver::new();
        resolver.kernel_loaded = true;
        resolver.clear();
        assert!(!resolver.is_loaded());
    }
}
