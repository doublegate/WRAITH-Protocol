use core::arch::asm;

pub fn get_random_bytes(buf: &mut [u8]) {
    #[cfg(not(target_os = "windows"))]
    {
        // Try OS RNG (getrandom) first
        let res = unsafe { crate::utils::syscalls::sys_getrandom(buf.as_mut_ptr(), buf.len(), 0) };
        if res == buf.len() as isize {
            return;
        }
    }

    for i in 0..buf.len() {
        buf[i] = get_random_u8();
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn get_random_u8() -> u8 {
    let mut val: u64 = 0;
    let mut success: u8 = 0;
    let tsc: u64;

    unsafe {
        // RDRAND - Hardware random number generator
        // We check the Carry Flag (CF) which is set to 1 if RDRAND succeeded.
        asm!(
            "rdrand {}",
            "setc {}",
            out(reg) val,
            out(reg_byte) success,
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

    // If RDRAND failed (success=0), val might be 0 or stale.
    if success == 0 {
        // Mix constant to differentiate from valid 0
        val = val.wrapping_add(0xCAFEBABE);
    }

    let stack_addr = &val as *const u64 as u64;

    let mixed = val.wrapping_add(tsc).wrapping_add(stack_addr);

    // PCG-like mixing step
    let mixed = mixed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);

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

    // Mix with stack address (ASLR entropy)
    let stack_var = 0u8;
    let stack_addr = &stack_var as *const u8 as u64;

    let mixed = cntvct.wrapping_add(stack_addr);

    // Simple mixing function
    let mixed = mixed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);

    (mixed >> 56) as u8
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")))]
fn get_random_u8() -> u8 {
    // Fallback for others
    let stack_var = 0u8;
    let addr = &stack_var as *const u8 as u64;
    (addr & 0xFF) as u8
}
