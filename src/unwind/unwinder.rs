//! DWARF-based stack unwinding support.
//!
//! This module provides functionality for calculating the Canonical Frame Address (CFA)
//! and restoring register values using DWARF Call Frame Information (CFI).

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;

use gimli::{
    read::EhFrame, CfaRule, EndianSlice, NativeEndian, Register, RegisterRule, UnwindContext,
    UnwindSection, UnwindTableRow,
};

use crate::error::{Result, UnwindError};

use super::BinaryUnwindInfo;

/// x86_64 register indices matching Linux kernel perf_regs.h
///
/// Reference: linux/arch/x86/include/uapi/asm/perf_regs.h
mod regs {
    pub const AX: u32 = 0;
    pub const BX: u32 = 1;
    pub const CX: u32 = 2;
    pub const DX: u32 = 3;
    pub const SI: u32 = 4;
    pub const DI: u32 = 5;
    pub const BP: u32 = 6;
    pub const SP: u32 = 7;
    pub const IP: u32 = 8;
    pub const FLAGS: u32 = 9;
    pub const CS: u32 = 10;
    pub const SS: u32 = 11;
    pub const DS: u32 = 12;
    pub const ES: u32 = 13;
    pub const FS: u32 = 14;
    pub const GS: u32 = 15;
    pub const R8: u32 = 16;
    pub const R9: u32 = 17;
    pub const R10: u32 = 18;
    pub const R11: u32 = 19;
    pub const R12: u32 = 20;
    pub const R13: u32 = 21;
    pub const R14: u32 = 22;
    pub const R15: u32 = 23;
}

/// Maximum stack depth to prevent infinite loops on corrupted stacks.
const MAX_STACK_DEPTH: usize = 128;

/// Thread-local storage for UnwindContext to enable reuse across samples.
///
/// UnwindContext is NOT thread-safe, so we use thread_local! to ensure
/// each thread has its own context. Reusing the context is critical for
/// performance as it avoids repeated allocations.
thread_local! {
    static UNWIND_CTX: RefCell<UnwindContext<usize>> = RefCell::new(UnwindContext::new());
}

/// Maximum number of x86_64 general-purpose registers.
const MAX_REGISTERS: usize = 24;

/// Container for user-space register values during stack unwinding.
///
/// This struct holds the current register state during DWARF-based stack unwinding.
/// It uses a fixed-size array for efficient storage and access.
#[derive(Clone, Debug, Default)]
pub struct UserRegisters {
    /// Register values indexed by x86_64 register number.
    /// None indicates the register value is not available.
    values: [Option<u64>; MAX_REGISTERS],
}

impl UserRegisters {
    /// Create a new empty register set.
    pub fn new() -> Self {
        Self {
            values: [None; MAX_REGISTERS],
        }
    }

    /// Create a register set from a HashMap of register values.
    ///
    /// This is useful for initializing registers from perf sample data.
    pub fn from_map(map: HashMap<u16, u64>) -> Self {
        let mut regs = Self::new();
        for (reg, value) in map {
            if (reg as usize) < MAX_REGISTERS {
                regs.values[reg as usize] = Some(value);
            }
        }
        regs
    }

    /// Get the value of a register by index.
    ///
    /// Returns `None` if the register value is not available.
    pub fn get(&self, reg: u16) -> Option<u64> {
        if (reg as usize) < MAX_REGISTERS {
            self.values[reg as usize]
        } else {
            None
        }
    }

    /// Get the value of a gimli Register.
    ///
    /// This is a convenience method for use with gimli's Register type.
    pub fn get_register(&self, reg: Register) -> Option<u64> {
        self.get(reg.0 as u16)
    }

    /// Set the value of a register by index.
    pub fn set(&mut self, reg: u16, value: u64) {
        if (reg as usize) < MAX_REGISTERS {
            self.values[reg as usize] = Some(value);
        }
    }

    /// Set the value of a gimli Register.
    pub fn set_register(&mut self, reg: Register, value: u64) {
        self.set(reg.0 as u16, value);
    }

    /// Get the instruction pointer (RIP).
    pub fn ip(&self) -> Option<u64> {
        self.get(regs::IP as u16)
    }

    /// Set the instruction pointer (RIP).
    pub fn set_ip(&mut self, value: u64) {
        self.set(regs::IP as u16, value);
    }

    /// Get the stack pointer (RSP).
    pub fn sp(&self) -> Option<u64> {
        self.get(regs::SP as u16)
    }

    /// Set the stack pointer (RSP).
    pub fn set_sp(&mut self, value: u64) {
        self.set(regs::SP as u16, value);
    }

    /// Get the base pointer (RBP).
    pub fn bp(&self) -> Option<u64> {
        self.get(regs::BP as u16)
    }

    /// Set the base pointer (RBP).
    pub fn set_bp(&mut self, value: u64) {
        self.set(regs::BP as u16, value);
    }

    /// Check if a register has a value.
    pub fn has(&self, reg: u16) -> bool {
        self.get(reg).is_some()
    }

    /// Clear a register value.
    pub fn clear(&mut self, reg: u16) {
        if (reg as usize) < MAX_REGISTERS {
            self.values[reg as usize] = None;
        }
    }
}

/// Read a u64 value from the stack with bounds checking.
///
/// # Arguments
///
/// * `stack` - The captured stack memory slice
/// * `addr` - The absolute address to read from
/// * `stack_base` - The base address of the stack (lowest address)
///
/// # Returns
///
/// The u64 value at the given address, or an error if out of bounds.
///
/// # Example
///
/// ```no_run
/// use perf_rs::unwind::read_stack_u64;
/// let stack = [0u8; 1024];
/// let base = 0x7fff0000;
/// // Read from address 0x7fff0010 (offset 16 from base)
/// let value = read_stack_u64(&stack, 0x7fff0010, base)?;
/// # Ok::<(), perf_rs::PerfError>(())
/// ```
pub fn read_stack_u64(
    stack: &[u8],
    addr: u64,
    stack_base: u64,
) -> std::result::Result<u64, UnwindError> {
    // Calculate offset from stack base
    let offset = addr
        .checked_sub(stack_base)
        .ok_or_else(|| UnwindError::StackReadFailed { address: addr })?;

    // Check that we can read 8 bytes starting at offset
    let offset_usize = offset as usize;
    if offset_usize + 8 > stack.len() {
        return Err(UnwindError::StackReadFailed { address: addr });
    }

    // Read the value (little-endian on x86_64)
    let bytes: [u8; 8] = stack[offset_usize..offset_usize + 8]
        .try_into()
        .expect("slice has correct length");
    Ok(u64::from_le_bytes(bytes))
}

/// Calculate the Canonical Frame Address (CFA) from an unwind table row.
///
/// The CFA is the base address of the current stack frame. It is typically
/// calculated as the value of a register (usually RSP or RBP) plus an offset.
///
/// # Arguments
///
/// * `row` - The unwind table row containing CFA rules
/// * `regs` - The current register values
/// * `stack` - The captured stack memory (for expression evaluation)
/// * `stack_base` - The base address of the stack
///
/// # Returns
///
/// The calculated CFA value, or an error if the CFA cannot be determined.
///
/// # Example
///
/// ```no_run
/// use gimli::{UnwindTableRow, EndianSlice, NativeEndian};
/// use perf_rs::unwind::UserRegisters;
///
/// let regs = UserRegisters::new();
/// let stack = vec![0u8; 1024];
/// // let cfa = calculate_cfa(&row, &regs, &stack, 0x7fff0000)?;
/// # Ok::<(), perf_rs::PerfError>(())
/// ```
pub fn calculate_cfa(
    row: &UnwindTableRow<usize>,
    regs: &UserRegisters,
    stack: &[u8],
    stack_base: u64,
) -> std::result::Result<u64, UnwindError> {
    match *row.cfa() {
        CfaRule::RegisterAndOffset { register, offset } => {
            // Get the register value
            let reg_value =
                regs.get_register(register)
                    .ok_or_else(|| UnwindError::RegisterNotFound {
                        register: register.0 as u16,
                    })?;

            // Calculate CFA = register + offset (offset can be negative)
            let cfa = (reg_value as i64 + offset) as u64;
            Ok(cfa)
        }
        CfaRule::Expression(_expr) => {
            // For v1, we don't implement full DWARF expression evaluation
            // This would require a full DWARF expression evaluator
            log::warn!("CFA expression evaluation not implemented for v1");
            Err(UnwindError::InvalidCfi {
                message: "CFA expression evaluation not implemented".to_string(),
            })
        }
    }
}

/// Restore register values using the rules from an unwind table row.
///
/// This function applies the register rules from the DWARF CFI to compute
/// the register values for the caller's frame.
///
/// # Arguments
///
/// * `row` - The unwind table row containing register rules
/// * `current_regs` - The current register values (callee's frame)
/// * `stack` - The captured stack memory
/// * `stack_base` - The base address of the stack
/// * `cfa` - The calculated Canonical Frame Address
///
/// # Returns
///
/// A new `UserRegisters` struct containing the restored register values
/// for the caller's frame.
///
/// # Example
///
/// ```no_run
/// use perf_rs::unwind::UserRegisters;
///
/// let current_regs = UserRegisters::new();
/// let stack = vec![0u8; 1024];
/// // let caller_regs = restore_registers(&row, &current_regs, &stack, 0x7fff0000, cfa)?;
/// # Ok::<(), perf_rs::PerfError>(())
/// ```
pub fn restore_registers(
    row: &UnwindTableRow<usize>,
    current_regs: &UserRegisters,
    stack: &[u8],
    stack_base: u64,
    cfa: u64,
) -> std::result::Result<UserRegisters, UnwindError> {
    let mut caller_regs = UserRegisters::new();

    // Process each register rule
    for (reg, rule) in row.registers() {
        let value =
            match *rule {
                // Undefined: the register has no recoverable value
                // For most registers this means the value is not preserved
                // For the return address (RIP), this signals end of unwinding
                RegisterRule::Undefined => {
                    // Skip this register - it has no value in the caller
                    continue;
                }

                // SameValue: the register has the same value in the caller
                RegisterRule::SameValue => current_regs.get_register(*reg).ok_or_else(|| {
                    UnwindError::RegisterNotFound {
                        register: reg.0 as u16,
                    }
                })?,

                // Offset(N): the value is at CFA + N on the stack
                RegisterRule::Offset(offset) => {
                    let addr = (cfa as i64 + offset) as u64;
                    read_stack_u64(stack, addr, stack_base)?
                }

                // ValOffset(N): the value is CFA + N (not dereferenced)
                RegisterRule::ValOffset(offset) => (cfa as i64 + offset) as u64,

                // Register(R): the value is in another register
                RegisterRule::Register(other_reg) => current_regs
                    .get_register(other_reg)
                    .ok_or_else(|| UnwindError::RegisterNotFound {
                        register: other_reg.0 as u16,
                    })?,

                // Expression: evaluate a DWARF expression to get the address
                RegisterRule::Expression(_expr) => {
                    log::warn!("Register expression evaluation not implemented for v1");
                    return Err(UnwindError::InvalidCfi {
                        message: "Register expression evaluation not implemented".to_string(),
                    });
                }

                // ValExpression: evaluate a DWARF expression to get the value directly
                RegisterRule::ValExpression(_expr) => {
                    log::warn!("Register value expression evaluation not implemented for v1");
                    return Err(UnwindError::InvalidCfi {
                        message: "Register value expression evaluation not implemented".to_string(),
                    });
                }

                // Architectural: architecture-specific rule
                RegisterRule::Architectural => {
                    return Err(UnwindError::InvalidCfi {
                        message: "Architectural register rule not supported".to_string(),
                    });
                }

                // Constant: the value is a constant
                RegisterRule::Constant(value) => value,

                // Handle any future variants
                _ => {
                    log::warn!("Unsupported register rule: {:?}", rule);
                    return Err(UnwindError::InvalidCfi {
                        message: format!("Unsupported register rule: {:?}", rule),
                    });
                }
            };

        caller_regs.set_register(*reg, value);
    }

    // The CFA becomes the new SP in the caller's frame
    caller_regs.set_sp(cfa);

    Ok(caller_regs)
}

/// DWARF-based stack unwinder using CFI information.
///
/// This struct manages binary unwind information and provides stack unwinding
/// capabilities using DWARF Call Frame Information (CFI).
///
/// # Thread Safety
///
/// The unwinder uses thread-local storage for `UnwindContext` to enable efficient
/// reuse across multiple unwinding operations. Each thread should have its own
/// `DwarfUnwinder` instance.
pub struct DwarfUnwinder {
    /// Cached binary unwind information, keyed by binary path.
    binaries: HashMap<PathBuf, BinaryUnwindInfo>,
}

impl DwarfUnwinder {
    /// Create a new empty unwinder.
    pub fn new() -> Self {
        Self {
            binaries: HashMap::new(),
        }
    }

    /// Load unwind information from a binary file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the ELF binary file
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the binary was loaded successfully, or an error
    /// if the file could not be read or parsed.
    pub fn load_binary(&mut self, path: &PathBuf) -> Result<()> {
        let info = BinaryUnwindInfo::load(path)?;
        self.binaries.insert(path.clone(), info);
        Ok(())
    }

    /// Find the binary containing the given address.
    fn find_binary_for_address(&self, _address: u64) -> Option<&BinaryUnwindInfo> {
        // TODO: Implement proper address-to-binary mapping using process memory maps
        self.binaries.values().find(|b| b.has_unwind_info())
    }

    /// Unwind the stack starting from the given instruction pointer.
    ///
    /// This method performs DWARF-based stack unwinding using CFI information
    /// from loaded binaries. It returns a vector of instruction pointers
    /// representing the call chain.
    ///
    /// # Arguments
    ///
    /// * `ip` - The starting instruction pointer
    /// * `regs` - The initial register values
    /// * `stack` - The stack memory contents (for reading saved registers)
    /// * `stack_base` - The base address of the stack (lowest address)
    ///
    /// # Returns
    ///
    /// A vector of instruction pointers representing the call chain,
    /// starting with the initial `ip`.
    pub fn unwind_stack(
        &self,
        ip: u64,
        regs: &UserRegisters,
        stack: &[u8],
        stack_base: u64,
    ) -> Result<Vec<u64>> {
        let mut callchain = Vec::with_capacity(16);
        callchain.push(ip);

        let mut current_ip = ip;
        let mut current_regs = regs.clone();

        for depth in 0..MAX_STACK_DEPTH {
            let binary =
                self.find_binary_for_address(current_ip)
                    .ok_or_else(|| UnwindError::NoEhFrame {
                        address: current_ip,
                    })?;

            let eh_frame = binary.eh_frame().ok_or_else(|| UnwindError::NoEhFrame {
                address: current_ip,
            })?;

            let result = UNWIND_CTX.with(|ctx_cell| {
                let mut ctx = ctx_cell.borrow_mut();
                self.unwind_frame(
                    &eh_frame,
                    binary.bases(),
                    &mut ctx,
                    current_ip,
                    &mut current_regs,
                    stack,
                    stack_base,
                )
            });

            match result {
                Ok(Some(next_ip)) => {
                    callchain.push(next_ip);
                    current_ip = next_ip;
                }
                Ok(None) => break,
                Err(e) => {
                    log::debug!("Unwinding stopped at depth {}: {}", depth, e);
                    break;
                }
            }
        }

        Ok(callchain)
    }

    /// Unwind a single frame.
    ///
    /// Returns `Ok(Some(next_ip))` if unwinding succeeded and there's another frame,
    /// `Ok(None)` if we've reached the end of the stack, or an error.
    fn unwind_frame(
        &self,
        eh_frame: &EhFrame<EndianSlice<'_, NativeEndian>>,
        bases: &gimli::BaseAddresses,
        ctx: &mut UnwindContext<usize>,
        ip: u64,
        regs: &mut UserRegisters,
        stack: &[u8],
        stack_base: u64,
    ) -> Result<Option<u64>> {
        let fde = eh_frame
            .fde_for_address(bases, ip, EhFrame::cie_from_offset)
            .map_err(|e| UnwindError::InvalidCfi {
                message: format!("Failed to find FDE for address 0x{:x}: {}", ip, e),
            })?;

        let row = fde
            .unwind_info_for_address(eh_frame, bases, ctx, ip)
            .map_err(|e| UnwindError::InvalidCfi {
                message: format!("Failed to get unwind info for address 0x{:x}: {}", ip, e),
            })?;

        let cfa = calculate_cfa(row, regs, stack, stack_base)?;

        let caller_regs = restore_registers(row, regs, stack, stack_base, cfa)?;

        let return_addr = match caller_regs.ip() {
            Some(0) | None => return Ok(None),
            Some(ra) => ra,
        };

        *regs = caller_regs;

        Ok(Some(return_addr.saturating_sub(1)))
    }

    /// Check if the unwinder has any binaries loaded.
    pub fn has_binaries(&self) -> bool {
        !self.binaries.is_empty()
    }

    /// Get the number of loaded binaries.
    pub fn binary_count(&self) -> usize {
        self.binaries.len()
    }
}

impl Default for DwarfUnwinder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_registers_new() {
        let regs = UserRegisters::new();

        // All registers should be None
        for i in 0..MAX_REGISTERS {
            assert!(regs.get(i as u16).is_none());
        }
    }

    #[test]
    fn test_user_registers_set_get() {
        let mut regs = UserRegisters::new();

        regs.set(regs::IP as u16, 0x12345678);
        regs.set(regs::SP as u16, 0x7fff0000);
        regs.set(regs::BP as u16, 0x7fff0010);

        assert_eq!(regs.ip(), Some(0x12345678));
        assert_eq!(regs.sp(), Some(0x7fff0000));
        assert_eq!(regs.bp(), Some(0x7fff0010));
    }

    #[test]
    fn test_user_registers_from_map() {
        let mut map = HashMap::new();
        map.insert(regs::IP as u16, 0x1000);
        map.insert(regs::SP as u16, 0x2000);

        let regs = UserRegisters::from_map(map);

        assert_eq!(regs.ip(), Some(0x1000));
        assert_eq!(regs.sp(), Some(0x2000));
        assert_eq!(regs.bp(), None);
    }

    #[test]
    fn test_user_registers_clear() {
        let mut regs = UserRegisters::new();
        regs.set(regs::IP as u16, 0x1234);

        assert!(regs.has(regs::IP as u16));

        regs.clear(regs::IP as u16);

        assert!(!regs.has(regs::IP as u16));
    }

    #[test]
    fn test_stack_read_bounds_check() {
        let stack = [0u8; 16];
        let base = 0x1000u64;

        // Valid read at the start
        let result = read_stack_u64(&stack, base, base);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0u64);

        // Valid read at the end
        let result = read_stack_u64(&stack, base + 8, base);
        assert!(result.is_ok());

        // Invalid: address before stack base
        let result = read_stack_u64(&stack, base - 1, base);
        assert!(result.is_err());

        // Invalid: read would go past end of stack
        let result = read_stack_u64(&stack, base + 9, base);
        assert!(result.is_err());

        // Invalid: address completely outside stack
        let result = read_stack_u64(&stack, base + 100, base);
        assert!(result.is_err());
    }

    #[test]
    fn test_stack_read_values() {
        // Create a stack with known values
        let mut stack = [0u8; 32];
        let base = 0x7fff0000u64;

        // Write some values
        stack[0..8].copy_from_slice(&0x1111111111111111u64.to_le_bytes());
        stack[8..16].copy_from_slice(&0x2222222222222222u64.to_le_bytes());
        stack[16..24].copy_from_slice(&0x3333333333333333u64.to_le_bytes());

        // Read them back
        assert_eq!(
            read_stack_u64(&stack, base, base).unwrap(),
            0x1111111111111111u64
        );
        assert_eq!(
            read_stack_u64(&stack, base + 8, base).unwrap(),
            0x2222222222222222u64
        );
        assert_eq!(
            read_stack_u64(&stack, base + 16, base).unwrap(),
            0x3333333333333333u64
        );
    }

    #[test]
    fn test_cfa_calculation_rsp_offset() {
        // This test verifies the CFA calculation logic
        // We can't easily create an UnwindTableRow, so we test the underlying logic

        // Simulate: CFA = RSP + 8 (typical after a call instruction)
        let mut regs = UserRegisters::new();
        regs.set_sp(0x7fff0000);

        // The CFA should be 0x7fff0008
        let rsp = regs.sp().unwrap();
        let offset: i64 = 8;
        let cfa = (rsp as i64 + offset) as u64;

        assert_eq!(cfa, 0x7fff0008);
    }

    #[test]
    fn test_cfa_calculation_rbp_offset() {
        // Simulate: CFA = RBP + 16 (typical for frame pointer based unwinding)
        let mut regs = UserRegisters::new();
        regs.set_bp(0x7fff0010);

        let rbp = regs.bp().unwrap();
        let offset: i64 = 16;
        let cfa = (rbp as i64 + offset) as u64;

        assert_eq!(cfa, 0x7fff0020);
    }

    #[test]
    fn test_cfa_calculation_negative_offset() {
        // Test with negative offset (shouldn't happen for CFA, but test the logic)
        let mut regs = UserRegisters::new();
        regs.set_sp(0x7fff0100);

        let rsp = regs.sp().unwrap();
        let offset: i64 = -16;
        let cfa = (rsp as i64 + offset) as u64;

        assert_eq!(cfa, 0x7fff00f0);
    }

    #[test]
    fn test_register_rule_offset() {
        // Test the Offset rule: value is at CFA + offset on stack
        let mut stack = [0u8; 64];
        let base = 0x7fff0000u64;
        let cfa = 0x7fff0010u64;

        // Write a return address at CFA - 8 (typical location)
        let return_addr = 0x400500u64;
        stack[8..16].copy_from_slice(&return_addr.to_le_bytes());

        // Simulate reading the return address
        let offset: i64 = -8;
        let addr = (cfa as i64 + offset) as u64;
        let value = read_stack_u64(&stack, addr, base).unwrap();

        assert_eq!(value, return_addr);
    }

    #[test]
    fn test_register_rule_val_offset() {
        // Test the ValOffset rule: value is CFA + offset (not dereferenced)
        let cfa = 0x7fff0010u64;
        let offset: i64 = 16;
        let value = (cfa as i64 + offset) as u64;

        assert_eq!(value, 0x7fff0020u64);
    }

    #[test]
    fn test_register_rule_same_value() {
        // Test the SameValue rule: caller's register = callee's register
        let mut current_regs = UserRegisters::new();
        current_regs.set_bp(0x7fff0020);

        // For callee-saved registers like RBP, SameValue means
        // the caller's RBP = callee's RBP
        let caller_bp = current_regs.bp().unwrap();
        assert_eq!(caller_bp, 0x7fff0020);
    }

    #[test]
    fn test_dwarf_unwinder_new() {
        let unwinder = DwarfUnwinder::new();
        assert!(!unwinder.has_binaries());
        assert_eq!(unwinder.binary_count(), 0);
    }

    #[test]
    fn test_dwarf_unwinder_default() {
        let unwinder = DwarfUnwinder::default();
        assert!(!unwinder.has_binaries());
    }

    #[test]
    fn test_unwind_context_reuse() {
        UNWIND_CTX.with(|ctx_cell| {
            let ctx = ctx_cell.borrow();
            let _ = &*ctx;
        });

        UNWIND_CTX.with(|ctx_cell| {
            let ctx = ctx_cell.borrow();
            let _ = &*ctx;
        });

        UNWIND_CTX.with(|ctx_cell| {
            let mut ctx = ctx_cell.borrow_mut();
            *ctx = UnwindContext::new();
        });

        UNWIND_CTX.with(|ctx_cell| {
            let ctx = ctx_cell.borrow();
            let _ = &*ctx;
        });
    }

    #[test]
    fn test_max_stack_depth_constant() {
        assert_eq!(MAX_STACK_DEPTH, 128);
    }
}
