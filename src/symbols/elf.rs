//! ELF symbol resolver using object, gimli, and addr2line crates.

use std::fs::File;
use std::path::Path;

use addr2line::object::{Object, ObjectSymbol};
use addr2line::Context;

use crate::error::{PerfError, Result};

use super::{SymbolInfo, SymbolResolver};

/// Symbol entry for sorted storage.
#[derive(Debug, Clone)]
struct SymbolEntry {
    start_addr: u64,
    end_addr: u64,
    name: String,
    size: u64,
}

/// ELF symbol resolver that parses both ELF symbols and DWARF debug info.
pub struct ElfResolver {
    /// Symbols sorted by start address for O(log n) lookups.
    symbols: Vec<SymbolEntry>,
    /// DWARF context for address-to-line resolution.
    dwarf_context: Option<Context<gimli::EndianRcSlice<gimli::RunTimeEndian>>>,
    /// Path to the loaded ELF file.
    loaded_path: Option<std::path::PathBuf>,
}

impl ElfResolver {
    /// Create a new ELF resolver.
    pub fn new() -> Self {
        Self {
            symbols: Vec::new(),
            dwarf_context: None,
            loaded_path: None,
        }
    }

    /// Load symbols from an ELF file.
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

        self.load_elf_symbols(&object);
        self.load_dwarf_context(&object)?;

        self.loaded_path = Some(path.to_path_buf());
        Ok(())
    }

    /// Load symbols from the ELF symbol table.
    fn load_elf_symbols(&mut self, object: &addr2line::object::File<'_>) {
        let mut entries: Vec<SymbolEntry> = Vec::new();

        for symbol in object.symbols() {
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

            let end_addr = if size > 0 { addr + size } else { addr + 1 };

            entries.push(SymbolEntry {
                start_addr: addr,
                end_addr,
                name: name.to_string(),
                size,
            });
        }

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

            let end_addr = if size > 0 { addr + size } else { addr + 1 };

            // Only add if not already present
            if !entries.iter().any(|e| e.start_addr == addr) {
                entries.push(SymbolEntry {
                    start_addr: addr,
                    end_addr,
                    name: name.to_string(),
                    size,
                });
            }
        }

        // Sort by start address for binary search
        entries.sort_by_key(|e| e.start_addr);
        self.symbols = entries;
    }

    /// Load DWARF debug context.
    fn load_dwarf_context(&mut self, object: &addr2line::object::File<'_>) -> Result<()> {
        match Context::new(object) {
            Ok(ctx) => {
                self.dwarf_context = Some(ctx);
                Ok(())
            }
            Err(_) => {
                self.dwarf_context = None;
                Ok(())
            }
        }
    }

    /// Resolve source location using DWARF info.
    fn resolve_dwarf_location(&self, addr: u64) -> Option<(String, u32)> {
        let ctx = self.dwarf_context.as_ref()?;

        let loc = ctx.find_location(addr).ok()??;

        match (loc.file, loc.line) {
            (Some(file), Some(line)) => Some((file.to_string(), line)),
            _ => None,
        }
    }

    /// Find symbol containing the given address using binary search.
    fn find_symbol_containing(&self, addr: u64) -> Option<SymbolInfo> {
        // Binary search for the largest start_addr <= addr
        let idx = self.symbols.partition_point(|e| e.start_addr <= addr);

        if idx == 0 {
            return None;
        }

        let entry = &self.symbols[idx - 1];

        // Check if address falls within the symbol's range
        if addr >= entry.start_addr && addr < entry.end_addr {
            return Some(SymbolInfo::new(
                entry.name.clone(),
                entry.start_addr,
                entry.size,
            ));
        }

        None
    }
}

impl Default for ElfResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbolResolver for ElfResolver {
    fn resolve(&self, addr: u64) -> Result<Option<SymbolInfo>> {
        // Try exact match first via binary search
        let exact_idx = self.symbols.partition_point(|e| e.start_addr < addr);

        if let Some(entry) = self.symbols.get(exact_idx) {
            if entry.start_addr == addr {
                let mut info = SymbolInfo::new(entry.name.clone(), entry.start_addr, entry.size);
                if let Some((file, line)) = self.resolve_dwarf_location(addr) {
                    info = info.with_source(file, line);
                }
                return Ok(Some(info));
            }
        }

        // Fall back to finding symbol containing the address
        if let Some(mut info) = self.find_symbol_containing(addr) {
            if let Some((file, line)) = self.resolve_dwarf_location(addr) {
                info = info.with_source(file, line);
            }
            return Ok(Some(info));
        }

        Ok(None)
    }

    fn load_symbols(&mut self, path: &Path) -> Result<()> {
        self.clear();
        self.load_elf(path)
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
        assert!(resolver.symbols.is_empty());
    }

    #[test]
    fn test_elf_resolver_clear() {
        let mut resolver = ElfResolver::new();
        resolver.symbols.push(SymbolEntry {
            start_addr: 0x1000,
            end_addr: 0x1100,
            name: "test".to_string(),
            size: 0x100,
        });
        resolver.loaded_path = Some(PathBuf::from("/test"));

        resolver.clear();

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
        resolver.symbols.push(SymbolEntry {
            start_addr: 0x1000,
            end_addr: 0x1100,
            name: "func1".to_string(),
            size: 0x100,
        });
        resolver.symbols.push(SymbolEntry {
            start_addr: 0x2000,
            end_addr: 0x2200,
            name: "func2".to_string(),
            size: 0x200,
        });

        let sym = resolver.find_symbol_containing(0x1000);
        assert!(sym.is_some());
        assert_eq!(sym.unwrap().name, "func1");

        let sym = resolver.find_symbol_containing(0x1050);
        assert!(sym.is_some());
        assert_eq!(sym.unwrap().name, "func1");

        let sym = resolver.find_symbol_containing(0x5000);
        assert!(sym.is_none());
    }
}
