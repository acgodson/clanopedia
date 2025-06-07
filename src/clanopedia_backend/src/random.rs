// src/clanopedia_backend/src/random.rs - Simple getrandom implementation for IC

use getrandom::{register_custom_getrandom, Error};

pub fn custom_random(buf: &mut [u8]) -> Result<(), Error> {
    // Use IC's time as a simple entropy source
    let time = ic_cdk::api::time();
    let mut seed = time;

    // Fill buffer with pseudo-random bytes using linear congruential generator
    for byte in buf.iter_mut() {
        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        *byte = (seed >> 16) as u8;
    }

    Ok(())
}

register_custom_getrandom!(custom_random);
