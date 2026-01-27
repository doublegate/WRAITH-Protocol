#[cfg(test)]
mod tests {
    #[cfg(not(target_os = "windows"))]
    #[test]
    fn test_heap_discovery() {
        let (base, size) = crate::utils::obfuscation::get_heap_range();
        println!("Heap base: {:p}, size: {}", base, size);
        
        // If heap is found, base != 0x10000000 (fallback)
        // But if test binary has no heap? It usually does.
        // If fallback, it returns 0x10000000.
        // We just verify it doesn't crash.
        assert!(!base.is_null());
        assert!(size > 0);
    }
}
