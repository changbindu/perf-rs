//! ELF symbol resolver using object, gimli, and addr2line crates.

use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

use addr2line::object::{Object, ObjectSymbol};
use addr2line::Context;

use crate::error::{PerfError, Result};

use super::{SymbolInfo, SymbolResolver};

/// ELF symbol resolver that parses both ELF symbols and DWARF debug info.
pub struct ElfResolver {
    /// Symbol table indexed by start address.
    symbols: HashMap<u64, SymbolInfo>,
    /// DWARF context for address-to-line resolution.
    dwarf_context: Option<Context<gimli::EndianRcSlice<gimli::RunTimeEndian>>>,
    /// Path to the loaded ELF file.
    loaded_path: Option<std::path::PathBuf>,
}

impl ElfResolver {
    /// Create a new ELF resolver.
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
            dwarf_context: None,
            loaded_path: None,
        }
    }

    /// Load symbols from an ELF file.
    ///
    /// This loads both ELF symbol table and DWARF debug information.
    fn load_elf(&mut self, path: &Path) -> Result<()> {
        let file = File::open(path).map_err(|e| PerfError::FileRead {
            path: path.to_path_buf(),
            source: Box::new(e),
        })?;

        let mmap = unsafe { memmap2::Mmap::map(&file) }.map_err(|e| PerfError::ElfParse {
            path: path.to_path_buf(),
            source: Box::new(e),
        })?;

        let object =
            addr2line::object::File::parse(&mmap[..]).map_err(|e| PerfError::ElfParse {
                path: path.to_path_buf(),
                source: Box::new(e),
            })?;

        // Load ELF symbols
        self.load_elf_symbols(&object);

        // Load DWARF context for debug info
        self.load_dwarf_context(&object)?;

        self.loaded_path = Some(path.to_path_buf());
        Ok(())
    }

    /// Load symbols from the ELF symbol table.
    fn load_elf_symbols(&mut self, object: &addr2line::object::File) {
        for symbol in object.symbols() {
            let name = symbol.name().unwrap_or("");
            if name.is_empty() {
                continue;
            }

            // Only load function symbols
            if !symbol.is_definition() || symbol.kind() != addr2line::object::SymbolKind::Text {
                continue;
            }

            let addr = symbol.address();
            let size = symbol.size();

            // Skip symbols with invalid addresses
            if addr == 0 {
                continue;
            }

            let info = SymbolInfo::new(name.to_string(), addr, size);
            self.symbols.insert(addr, info);
        }

        // Also load dynamic symbols
        for symbol in object.dynamic_symbols() {
            let name = symbol.name().unwrap_or("");
            if name.is_empty() {
                continue;
            }

            if !symbol.is_definition() || symbol.kind() != addr2line::object::SymbolKind::Text {
                continue;
            }

            let addr = symbol.address();
            let size = symbol.size();

            if addr == 0 {
                continue;
            }

            // Don't overwrite existing symbols
            if !self.symbols.contains_key(&addr) {
                let info = SymbolInfo::new(name.to_string(), addr, size);
                self.symbols.insert(addr, info);
            }
        }
    }

    /// Load DWARF debug context.
    fn load_dwarf_context(&mut self, object: &addr2line::object::File) -> Result<()> {
        // Try to create a DWARF context
        match Context::new(object) {
            Ok(ctx) => {
                self.dwarf_context = Some(ctx);
                Ok(())
            }
            Err(_) => {
                // DWARF info is optional; continue without it
                self.dwarf_context = None;
                Ok(())
            }
        }
    }

    /// Resolve source location using DWARF info.
    fn resolve_dwarf_location(&self, addr: u64) -> Option<(String, u32)> {
        let ctx = self.dwarf_context.as_ref()?;

        // Find the location for this address
        let loc = ctx.find_location(addr).ok()??;

        match (loc.file, loc.line) {
            (Some(file), Some(line)) => Some((file.to_string(), line)),
            _ => None,
        }
    }

    /// Find symbol containing the given address.
    fn find_symbol_containing(&self, addr: u64) -> Option<&SymbolInfo> {
        // Binary search could be used here for better performance
        // For now, linear scan is acceptable for MVP
        self.symbols.values().find(|sym| sym.contains(addr))
    }
}

impl Default for ElfResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbolResolver for ElfResolver {
    fn resolve(&self, addr: u64) -> Result<Option<SymbolInfo>> {
        // First try exact match
        if let Some(mut info) = self.symbols.get(&addr).cloned() {
            // Try to enhance with DWARF info
            if let Some((file, line)) = self.resolve_dwarf_location(addr) {
                info = info.with_source(file, line);
            }
            return Ok(Some(info));
        }

        // Try to find a symbol containing this address
        if let Some(mut info) = self.find_symbol_containing(addr).cloned() {
            // Try to enhance with DWARF info
            if let Some((file, line)) = self.resolve_dwarf_location(addr) {
                info = info.with_source(file, line);
            }
            return Ok(Some(info));
        }

        // No symbol found
        Ok(None)
    }

    fn load_symbols(&mut self, path: &Path) -> Result<()> {
        self.clear();
        self.load_elf(path)
    }

    fn is_loaded(&self) -> bool {
        self.loaded_path.is_some()
    }

    fn clear(&mut self) {
        self.symbols.clear();
        self.dwarf_context = None;
        self.loaded_path = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_elf_resolver_new() {
        let resolver = ElfResolver::new();
        assert!(!resolver.is_loaded());
        assert!(resolver.symbols.is_empty());
    }

    #[test]
    fn test_elf_resolver_clear() {
        let mut resolver = ElfResolver::new();
        resolver
            .symbols
            .insert(0x1000, SymbolInfo::new("test".to_string(), 0x1000, 0x100));
        resolver.loaded_path = Some(PathBuf::from("/test"));

        resolver.clear();

        assert!(!resolver.is_loaded());
        assert!(resolver.symbols.is_empty());
    }

    #[test]
    fn test_resolve_unloaded() {
        let resolver = ElfResolver::new();
        let result = resolver.resolve(0x1000).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_find_symbol_containing() {
        let mut resolver = ElfResolver::new();
        resolver
            .symbols
            .insert(0x1000, SymbolInfo::new("func1".to_string(), 0x1000, 0x100));
        resolver
            .symbols
            .insert(0x2000, SymbolInfo::new("func2".to_string(), 0x2000, 0x200));

        // Exact match
        let sym = resolver.find_symbol_containing(0x1000);
        assert!(sym.is_some());
        assert_eq!(sym.unwrap().name, "func1");

        // Within range
        let sym = resolver.find_symbol_containing(0x1050);
        assert!(sym.is_some());
        assert_eq!(sym.unwrap().name, "func1");

        // Not in any symbol
        let sym = resolver.find_symbol_containing(0x5000);
        assert!(sym.is_none());
    }
}
