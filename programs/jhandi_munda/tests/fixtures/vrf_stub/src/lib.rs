#![no_std]

// Test stand-in for the MagicBlock VRF program. It accepts every instruction and
// does nothing, so a test can check the real request path (accounts, guards,
// stored request id) without the oracle. Never deployed.
#[no_mangle]
pub extern "C" fn entrypoint(_input: *mut u8) -> u64 {
    0
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
