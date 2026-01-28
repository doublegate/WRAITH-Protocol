use crate::utils::entropy::get_random_bytes;
use alloc::vec::Vec;
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    XChaCha20Poly1305, XNonce,
};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct SensitiveData {
    encrypted: Vec<u8>,
    nonce: [u8; 24],
    key: [u8; 32],
}

impl SensitiveData {
    pub fn new(plaintext: &[u8]) -> Self {
        let mut key = [0u8; 32];
        get_random_bytes(&mut key);
        let mut nonce = [0u8; 24];
        get_random_bytes(&mut nonce);

        let cipher = XChaCha20Poly1305::new(&key.into());
        let xnonce = XNonce::from_slice(&nonce);

        // Encrypt returns Vec<u8> (ciphertext + tag)
        let encrypted = cipher
            .encrypt(xnonce, plaintext)
            .expect("Encryption failed");

        Self {
            encrypted,
            nonce,
            key,
        }
    }

    pub fn unlock(&self) -> Option<SensitiveGuard> {
        let cipher = XChaCha20Poly1305::new(&self.key.into());
        let xnonce = XNonce::from_slice(&self.nonce);

        let decrypted = cipher.decrypt(xnonce, self.encrypted.as_ref()).ok()?;

        Some(SensitiveGuard {
            data: Zeroizing::new(decrypted),
        })
    }
}

pub struct SensitiveGuard {
    data: Zeroizing<Vec<u8>>,
}

impl core::ops::Deref for SensitiveGuard {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

#[derive(Zeroize)]
#[zeroize(drop)]
pub struct SecureBuffer {
    data: Vec<u8>,
}

impl SecureBuffer {
    pub fn new(data: Vec<u8>) -> Self {
        let mut buf = Self { data };
        buf.lock_memory();
        buf
    }

    fn lock_memory(&mut self) {
        if self.data.is_empty() {
            return;
        }

        #[cfg(not(target_os = "windows"))]
        unsafe {
            sys_mlock(self.data.as_ptr(), self.data.len());
        }

        #[cfg(target_os = "windows")]
        unsafe {
            use crate::utils::api_resolver::{hash_str, resolve_function};
            let k32 = hash_str(b"kernel32.dll");
            let vl_hash = hash_str(b"VirtualLock");
            let fn_vl = resolve_function(k32, vl_hash);

            if !fn_vl.is_null() {
                type FnVirtualLock =
                    unsafe extern "system" fn(*const core::ffi::c_void, usize) -> i32;
                core::mem::transmute::<_, FnVirtualLock>(fn_vl)(
                    self.data.as_ptr() as *const _,
                    self.data.len(),
                );
            }
        }
    }
}

impl AsRef<[u8]> for SecureBuffer {
    fn as_ref(&self) -> &[u8] {
        &self.data
    }
}

impl AsMut<[u8]> for SecureBuffer {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }
}

impl From<Vec<u8>> for SecureBuffer {
    fn from(v: Vec<u8>) -> Self {
        Self::new(v)
    }
}

#[cfg(not(target_os = "windows"))]
unsafe fn sys_mlock(addr: *const u8, len: usize) -> isize {
    let ret: isize;
    core::arch::asm!(
        "syscall",
        in("rax") 149_usize, // mlock
        in("rdi") addr as usize,
        in("rsi") len,
        lateout("rax") ret,
        out("rcx") _,
        out("r11") _,
        options(nostack)
    );
    ret
}
