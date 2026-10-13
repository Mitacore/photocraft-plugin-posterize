//! Posterize: a PhotoCraft filter plug-in (plug-in ABI v1) built from the `invert-rs` example.
//!
//! `no_std`, no allocator, no imports. Reduces every colour channel to `levels` evenly spaced
//! values and leaves alpha alone.
#![cfg_attr(not(test), no_std)]

#[cfg(all(not(test), target_arch = "wasm32"))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}

const MANIFEST: &str = r#"{
  "id": "io.github.mitacore.posterize",
  "name": "Posterize (Plug-in)",
  "version": "1.0.0",
  "kind": "filter",
  "author": "Mitacore",
  "description": "Reduces each colour channel to a fixed number of levels.",
  "params": {
    "levels": {"type": "int", "min": 2, "max": 32, "default": 4}
  }
}"#;

/// Bit 8 of the `format` argument: the last channel is alpha.
const HAS_ALPHA: u32 = 1 << 8;
const DEFAULT_LEVELS: u32 = 4;

#[unsafe(no_mangle)]
pub extern "C" fn pc_abi_version() -> u32 {
    1
}

/// `len << 32 | ptr` of the UTF-8 manifest JSON.
#[unsafe(no_mangle)]
pub extern "C" fn pc_manifest() -> u64 {
    ((MANIFEST.len() as u64) << 32) | MANIFEST.as_ptr() as u64
}

/// A block of `size` bytes from fresh pages (each run gets a fresh instance, so nothing is freed).
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn pc_alloc(size: u32) -> u32 {
    let pages = (size as usize).div_ceil(65536);
    let old = core::arch::wasm32::memory_grow(0, pages);
    if old == usize::MAX { 0 } else { (old * 65536) as u32 }
}

/// Reads the integer after `"levels":` in the host's parameter JSON.
fn read_levels(json: &[u8]) -> u32 {
    let key = b"\"levels\"";
    let mut i = 0;
    while i + key.len() <= json.len() {
        if &json[i..i + key.len()] == key {
            let mut j = i + key.len();
            while j < json.len() && (json[j] == b':' || json[j] == b' ') {
                j += 1;
            }
            let mut n: u32 = 0;
            let mut any = false;
            while j < json.len() && json[j].is_ascii_digit() {
                n = n.saturating_mul(10).saturating_add((json[j] - b'0') as u32);
                any = true;
                j += 1;
            }
            return if any { n.clamp(2, 64) } else { DEFAULT_LEVELS };
        }
        i += 1;
    }
    DEFAULT_LEVELS
}

/// Snaps every colour sample (alpha excluded) to one of `levels` evenly spaced values in 0..=1.
fn posterize(px: &mut [f32], channels: usize, alpha: bool, levels: u32) {
    let steps = (levels.max(2) - 1) as f32;
    let colour = if alpha { channels - 1 } else { channels };
    for p in px.chunks_exact_mut(channels) {
        for v in &mut p[..colour] {
            let x = if *v < 0.0 { 0.0 } else if *v > 1.0 { 1.0 } else { *v };
            // x is in 0..=1, so the cast truncates a non-negative value (= floor).
            *v = ((x * steps + 0.5) as u32) as f32 / steps;
        }
    }
}

/// Posterizes `width x height` pixels of `channels` interleaved `f32` samples at `buf`.
///
/// # Safety
/// The host guarantees `buf..buf + len` came from `pc_alloc` and holds `len / 4` samples, and
/// `params..params + params_len` holds the parameter JSON.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pc_filter(buf: *mut f32, len: u32, _width: u32, _height: u32, channels: u32, format: u32, params: *const u8, params_len: u32) -> i32 {
    let n = channels as usize;
    if n == 0 {
        return 1;
    }
    // SAFETY: see the function's contract.
    let px = unsafe { core::slice::from_raw_parts_mut(buf, len as usize / 4) };
    let json: &[u8] = if params.is_null() || params_len == 0 {
        &[]
    } else {
        // SAFETY: see the function's contract.
        unsafe { core::slice::from_raw_parts(params, params_len as usize) }
    };
    posterize(px, n, format & HAS_ALPHA != 0, read_levels(json));
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(px: &mut [f32], channels: u32, format: u32, params: &str) -> i32 {
        // SAFETY: the pointers come from live slices and the lengths match them.
        unsafe { pc_filter(px.as_mut_ptr(), (px.len() * 4) as u32, 1, (px.len() as u32) / channels, channels, format, params.as_ptr(), params.len() as u32) }
    }

    #[test]
    fn two_levels_snap_to_black_or_white_and_keep_alpha() {
        let mut px = [0.2, 0.7, 0.5, 0.3, 0.9, 0.1, 0.49, 0.8];
        assert_eq!(run(&mut px, 4, HAS_ALPHA | 8 | (3 << 16), r#"{"levels":2,"_image":{}}"#), 0);
        assert_eq!(px, [0.0, 1.0, 1.0, 0.3, 1.0, 0.0, 0.0, 0.8]);
    }

    #[test]
    fn four_levels_use_thirds_and_leave_alpha_untouched() {
        let mut px = [0.0, 0.34, 0.6, 0.123, 1.0, 0.5, 0.9, 0.0];
        assert_eq!(run(&mut px, 4, HAS_ALPHA, r#"{"levels":4}"#), 0);
        let third = 1.0 / 3.0;
        assert_eq!(px[3], 0.123);
        assert_eq!(px[7], 0.0);
        assert!((px[1] - third).abs() < 1e-6 && (px[2] - 2.0 * third).abs() < 1e-6);
        assert_eq!((px[0], px[4]), (0.0, 1.0));
    }

    #[test]
    fn no_alpha_flag_means_every_channel_is_colour() {
        let mut px = [0.2, 0.8];
        assert_eq!(run(&mut px, 1, 8 | (1 << 16), r#"{"levels":2}"#), 0);
        assert_eq!(px, [0.0, 1.0]);
    }

    #[test]
    fn out_of_range_samples_are_clamped_and_missing_levels_default_to_four() {
        let mut px = [-0.5, 2.0];
        assert_eq!(run(&mut px, 1, 32, "{}"), 0);
        assert_eq!(px, [0.0, 1.0]);
        assert_eq!(read_levels(br#"{"levels": 9}"#), 9);
        assert_eq!(read_levels(br#"{"levels":999}"#), 64);
    }

    #[test]
    fn zero_channels_is_an_error() {
        let mut px = [0.5];
        assert_eq!(run(&mut px, 0, 0, "{}"), 1);
    }
}
