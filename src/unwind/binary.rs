//! Binary file unwinding support using DWARF CFI.

use std::fs::File;
use std::path::Path;

use addr2line::object::{self, Object, ObjectSection};
use gimli::{
    read::{EhFrame, EhFrameHdr, FrameDescriptionEntry},
    BaseAddresses, EndianSlice, NativeEndian, UnwindSection,
};
use memmap2::Mmap;

use crate::error::{PerfError, Result, UnwindError};

/// DWARF section names for unwind information.
const EH_FRAME_NAME: &str = ".eh_frame";
const EH_FRAME_HDR_NAME: &str = ".eh_frame_hdr";
const TEXT_NAME: &str = ".text";
const GOT_NAME: &str = ".got";

/// Binary unwind information extracted from an ELF file.
///
/// This struct holds the memory-mapped binary file and provides access
/// to DWARF CFI information needed for stack unwinding.
pub struct BinaryUnwindInfo {
    mmap: Mmap,
    bases: BaseAddresses,
    has_eh_frame: bool,
}

impl BinaryUnwindInfo {
    /// Load unwind information from an ELF binary.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the ELF binary file
    ///
    /// # Returns
    ///
    /// A `BinaryUnwindInfo` struct containing the parsed unwind information,
    /// or an error if the file cannot be read or parsed.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::path::Path;
    /// use perf_rs::unwind::BinaryUnwindInfo;
    ///
    /// let path = Path::new("/usr/bin/ls");
    /// let unwind_info = BinaryUnwindInfo::load(path)?;
    /// # Ok::<(), perf_rs::PerfError>(())
    /// ```
    pub fn load(path: &Path) -> Result<Self> {
        let file = File::open(path).map_err(|e| PerfError::FileRead {
            path: path.to_path_buf(),
            source: Box::new(e),
        })?;

        let mmap = unsafe { Mmap::map(&file) }.map_err(|e| PerfError::FileRead {
            path: path.to_path_buf(),
            source: Box::new(e),
        })?;

        let obj_file = object::File::parse(&*mmap).map_err(|e| PerfError::ElfParse {
            path: path.to_path_buf(),
            source: Box::new(e),
        })?;

        let bases = Self::build_base_addresses(&obj_file);
        let has_eh_frame = obj_file.section_by_name(EH_FRAME_NAME).is_some();

        Ok(Self {
            mmap,
            bases,
            has_eh_frame,
        })
    }

    /// Build base addresses from ELF sections.
    fn build_base_addresses(file: &object::File<'_>) -> BaseAddresses {
        let mut bases = BaseAddresses::default();

        if let Some(section) = file.section_by_name(EH_FRAME_NAME) {
            bases = bases.set_eh_frame(section.address());
        }

        if let Some(section) = file.section_by_name(EH_FRAME_HDR_NAME) {
            bases = bases.set_eh_frame_hdr(section.address());
        }

        if let Some(section) = file.section_by_name(TEXT_NAME) {
            bases = bases.set_text(section.address());
        }

        if let Some(section) = file.section_by_name(GOT_NAME) {
            bases = bases.set_got(section.address());
        }

        bases
    }

    /// Parse and return the ELF file.
    ///
    /// This parses the memory-mapped data on each call. For performance-critical
    /// code, consider caching the result if needed multiple times.
    pub fn file(&self) -> object::File<'_> {
        object::File::parse(&*self.mmap).expect("Failed to parse ELF file from cached mmap")
    }

    /// Get the .eh_frame section data if available.
    pub fn eh_frame(&self) -> Option<EhFrame<EndianSlice<'_, NativeEndian>>> {
        let file = self.file();
        file.section_by_name(EH_FRAME_NAME)
            .and_then(|section| section.data().ok())
            .map(|data| EhFrame::new(data, NativeEndian))
    }

    /// Get the .eh_frame_hdr section data if available.
    pub fn eh_frame_hdr(&self) -> Option<EhFrameHdr<EndianSlice<'_, NativeEndian>>> {
        let file = self.file();
        file.section_by_name(EH_FRAME_HDR_NAME)
            .and_then(|section| section.data().ok())
            .map(|data| EhFrameHdr::new(data, NativeEndian))
    }

    /// Get the base addresses for DWARF sections.
    pub fn bases(&self) -> &BaseAddresses {
        &self.bases
    }

    /// Check if the binary has unwind information.
    pub fn has_unwind_info(&self) -> bool {
        self.has_eh_frame
    }

    /// Find the Frame Description Entry (FDE) for a given address.
    ///
    /// This method uses a two-tier lookup strategy:
    /// 1. **Fast path**: If `.eh_frame_hdr` is available with a binary search table,
    ///    uses O(log n) lookup via `ParsedEhFrameHdr::table()`.
    /// 2. **Slow path**: Falls back to linear search via `EhFrame::fde_for_address()`.
    ///
    /// # Arguments
    ///
    /// * `address` - The instruction pointer address to find FDE for
    ///
    /// # Returns
    ///
    /// The `FrameDescriptionEntry` containing CFI for the function containing the address,
    /// or an error if no FDE is found.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::path::Path;
    /// use perf_rs::unwind::BinaryUnwindInfo;
    ///
    /// let path = Path::new("/usr/bin/ls");
    /// let unwind_info = BinaryUnwindInfo::load(path)?;
    /// let fde = unwind_info.find_fde(0x401000)?;
    /// # Ok::<(), perf_rs::PerfError>(())
    /// ```
    pub fn find_fde(
        &self,
        address: u64,
    ) -> std::result::Result<FrameDescriptionEntry<EndianSlice<'_, NativeEndian>, usize>, UnwindError>
    {
        let file = self.file();
        let address_size = file
            .architecture()
            .address_size()
            .map(|s| s.bytes())
            .unwrap_or(std::mem::size_of::<usize>() as u8);

        // Try fast path: use eh_frame_hdr binary search table if available
        if let Some(eh_frame_hdr) = self.eh_frame_hdr() {
            if let Ok(parsed_hdr) = eh_frame_hdr.parse(&self.bases, address_size) {
                if let Some(table) = parsed_hdr.table() {
                    // Fast path: O(log n) binary search via eh_frame_hdr table
                    if let Some(eh_frame) = self.eh_frame() {
                        if let Ok(fde) = table.fde_for_address(
                            &eh_frame,
                            &self.bases,
                            address,
                            EhFrame::cie_from_offset,
                        ) {
                            return Ok(fde);
                        }
                    }
                }
            }
        }

        // Slow path: linear search through eh_frame
        if let Some(eh_frame) = self.eh_frame() {
            eh_frame
                .fde_for_address(&self.bases, address, EhFrame::cie_from_offset)
                .map_err(|e| match e {
                    gimli::Error::NoUnwindInfoForAddress => UnwindError::NoEhFrame { address },
                    other => UnwindError::InvalidCfi {
                        message: other.to_string(),
                    },
                })
        } else {
            Err(UnwindError::NoEhFrame { address })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eh_frame_name_constant() {
        assert_eq!(EH_FRAME_NAME, ".eh_frame");
        assert_eq!(EH_FRAME_HDR_NAME, ".eh_frame_hdr");
        assert_eq!(TEXT_NAME, ".text");
        assert_eq!(GOT_NAME, ".got");
    }

    #[test]
    fn test_find_fde_with_real_binary() {
        let path = Path::new("/usr/bin/ls");
        if !path.exists() {
            return;
        }

        let unwind_info = BinaryUnwindInfo::load(path).expect("Failed to load binary");

        if !unwind_info.has_unwind_info() {
            return;
        }

        let file = unwind_info.file();
        let text_section = match file.section_by_name(TEXT_NAME) {
            Some(s) => s,
            None => return,
        };

        let text_start = text_section.address();
        let text_end = text_start + text_section.size();

        if text_start >= text_end {
            return;
        }

        let mid_address = text_start + (text_section.size() / 2);
        let result = unwind_info.find_fde(mid_address);

        match result {
            Ok(fde) => {
                assert!(fde.initial_address() <= mid_address);
                assert!(fde.initial_address() + fde.len() > mid_address);
            }
            Err(UnwindError::NoEhFrame { address }) => {
                assert_eq!(address, mid_address);
            }
            Err(UnwindError::InvalidCfi { message: _ }) => {}
            _ => panic!("Unexpected error type"),
        }
    }

    #[test]
    fn test_find_fde_invalid_address() {
        let path = Path::new("/usr/bin/ls");
        if !path.exists() {
            return;
        }

        let unwind_info = BinaryUnwindInfo::load(path).expect("Failed to load binary");

        if !unwind_info.has_unwind_info() {
            return;
        }

        let invalid_address = 0xDEADBEEF;
        let result = unwind_info.find_fde(invalid_address);

        assert!(matches!(
            result,
            Err(UnwindError::NoEhFrame { address }) if address == invalid_address
        ));
    }

    #[test]
    fn test_eh_frame_hdr_section_available() {
        let path = Path::new("/usr/bin/ls");
        if !path.exists() {
            return;
        }

        let unwind_info = BinaryUnwindInfo::load(path).expect("Failed to load binary");

        if unwind_info.has_unwind_info() {
            let eh_frame = unwind_info.eh_frame();
            assert!(eh_frame.is_some());

            let eh_frame_hdr = unwind_info.eh_frame_hdr();
            if eh_frame_hdr.is_some() {
                let file = unwind_info.file();
                let address_size = file
                    .architecture()
                    .address_size()
                    .map(|s| s.bytes())
                    .unwrap_or(std::mem::size_of::<usize>() as u8);
                let parsed = eh_frame_hdr
                    .unwrap()
                    .parse(unwind_info.bases(), address_size);
                if let Ok(parsed_hdr) = parsed {
                    let table = parsed_hdr.table();
                    assert!(table.is_some());
                }
            }
        }
    }
}
