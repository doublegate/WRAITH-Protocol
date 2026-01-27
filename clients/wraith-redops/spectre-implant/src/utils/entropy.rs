use core::arch::asm;

pub fn get_random_bytes(buf: &mut [u8]) {
    for i in 0..buf.len() {
        buf[i] = get_random_u8();
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn get_random_u8() -> u8 {
    let mut val: u64 = 0;
    let tsc: u64;
    
    unsafe {
        // RDRAND - Hardware random number generator
        // If supported, val will be random. If not, it might be 0 or unchanged.
        // We use a "byte" directive to emit raw opcode if needed, but "rdrand" mnemonic is usually supported.
        // We won't check CF for success in this minimal no_std impl, we just mix with RDTSC.
        asm!(
            "rdrand {}",
            out(reg) val,
            options(nomem, nostack)
        );
        
        // RDTSC - Time Stamp Counter
        asm!(
            "rdtsc",
            out("rax") tsc,
            out("rdx") _,
            options(nomem, nostack)
        );
    }
    
    // Mix RDRAND output with RDTSC and stack address for additional entropy
    let stack_addr = &val as *const u64 as u64;
    
    let mixed = val.wrapping_add(tsc).wrapping_add(stack_addr);
    
    // Simple mixing function (PCG-like step) to spread bits
    let mixed = mixed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    
    (mixed >> 56) as u8
}

#[cfg(target_arch = "aarch64")]
fn get_random_u8() -> u8 {
    let mut cntvct: u64;
    unsafe {
        asm!(
            "mrs {}, cntvct_el0",
            out(reg) cntvct,
            options(nomem, nostack)
        );
    }
    
    // Mix with stack address
    let stack_var = 0u8;
    let stack_addr = &stack_var as *const u8 as u64;
    
    let mixed = cntvct.wrapping_add(stack_addr);
    
    // Simple mixing function
    let mixed = mixed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    
    (mixed >> 56) as u8
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")))]
fn get_random_u8() -> u8 {
    // Fallback for others
    let stack_var = 0u8;
    let addr = &stack_var as *const u8 as u64;
    (addr & 0xFF) as u8
}
