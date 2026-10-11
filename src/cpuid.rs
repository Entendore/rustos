use alloc::string::String;
use core::arch::x86_64::__cpuid;

pub fn vendor() -> String {
    let cpuid0 = unsafe { __cpuid(0) };
    let vendor_bytes: [u8; 12] = [
        (cpuid0.ebx & 0xFF) as u8, ((cpuid0.ebx >> 8) & 0xFF) as u8, ((cpuid0.ebx >> 16) & 0xFF) as u8, ((cpuid0.ebx >> 24) & 0xFF) as u8,
        (cpuid0.edx & 0xFF) as u8, ((cpuid0.edx >> 8) & 0xFF) as u8, ((cpuid0.edx >> 16) & 0xFF) as u8, ((cpuid0.edx >> 24) & 0xFF) as u8,
        (cpuid0.ecx & 0xFF) as u8, ((cpuid0.ecx >> 8) & 0xFF) as u8, ((cpuid0.ecx >> 16) & 0xFF) as u8, ((cpuid0.ecx >> 24) & 0xFF) as u8,
    ];
    core::str::from_utf8(&vendor_bytes).unwrap_or("Unknown").to_string()
}

pub fn brand() -> String {
    let mut brand = [0u8; 48];
    for i in 0..3 {
        let cpuid = unsafe { __cpuid(0x80000002 + i) };
        let offset = i * 16;
        brand[offset..offset+4].copy_from_slice(&cpuid.eax.to_le_bytes());
        brand[offset+4..offset+8].copy_from_slice(&cpuid.ebx.to_le_bytes());
        brand[offset+8..offset+12].copy_from_slice(&cpuid.ecx.to_le_bytes());
        brand[offset+12..offset+16].copy_from_slice(&cpuid.edx.to_le_bytes());
    }
    core::str::from_utf8(&brand).unwrap_or("Unknown").trim_matches('\0').to_string()
}
