mod elf;
mod kernel;

pub use elf::ElfResolver;
pub use kernel::KernelResolver;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::Result;

#[derive(Debug, Clone, PartialEq)]
pub struct SymbolInfo {
    pub name: String,
    pub start_addr: u64,
    pub size: u64,
    pub source_file: Option<String>,
    pub line: Option<u32>,
}

impl SymbolInfo {
    pub fn new(name: String, start_addr: u64, size: u64) -> Self {
        Self {
            name,
            start_addr,
            size,
            source_file: None,
            line: None,
        }
    }

    pub fn with_source(mut self, file: String, line: u32) -> Self {
        self.source_file = Some(file);
        self.line = Some(line);
        self
    }

    pub fn contains(&self, addr: u64) -> bool {
        if self.size == 0 {
            addr == self.start_addr
        } else {
            addr >= self.start_addr && addr < self.start_addr + self.size
        }
    }
}

pub trait SymbolResolver {
    fn resolve(&self, addr: u64) -> Result<Option<SymbolInfo>>;
    fn load_symbols(&mut self, path: &Path) -> Result<()>;
    fn clear(&mut self);
}

pub struct MultiResolver {
    elf_resolvers: HashMap<PathBuf, ElfResolver>,
    kernel_resolver: Option<KernelResolver>,
}

impl MultiResolver {
    pub fn new() -> Self {
        Self {
            elf_resolvers: HashMap::new(),
            kernel_resolver: None,
        }
    }

    pub fn set_kernel_resolver(&mut self, resolver: KernelResolver) {
        self.kernel_resolver = Some(resolver);
    }
}

impl Default for MultiResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbolResolver for MultiResolver {
    fn resolve(&self, addr: u64) -> Result<Option<SymbolInfo>> {
        if let Some(ref resolver) = self.kernel_resolver {
            if let Some(info) = resolver.resolve(addr)? {
                return Ok(Some(info));
            }
        }

        for resolver in self.elf_resolvers.values() {
            if let Some(info) = resolver.resolve(addr)? {
                return Ok(Some(info));
            }
        }

        Ok(None)
    }

    fn load_symbols(&mut self, path: &Path) -> Result<()> {
        let mut resolver = ElfResolver::new();
        resolver.load_symbols(path)?;
        self.elf_resolvers.insert(path.to_path_buf(), resolver);
        Ok(())
    }

    fn clear(&mut self) {
        self.elf_resolvers.clear();
        self.kernel_resolver = None;
    }
}
