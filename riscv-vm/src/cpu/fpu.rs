//! F/D extension execution helpers.
//!
//! Values live in a 64-bit FP register file; f32 values are NaN-boxed (upper
//! 32 bits all-ones) per the RISC-V spec. Arithmetic uses host IEEE-754
//! (Rust f32/f64) with results canonicalized to the RISC-V canonical NaN.
//!
//! Rounding: host arithmetic runs in round-nearest-even, which is what
//! compiled code uses almost exclusively for arithmetic. Float->int
//! conversions honor all five rounding modes explicitly (they are computed
//! in the integer domain), which covers the encodings compilers actually
//! emit (RTZ for casts, RNE/DYN elsewhere).

/// fflags bits (fcsr[4:0]).
pub const FFLAG_NX: u32 = 1 << 0; // inexact
pub const FFLAG_UF: u32 = 1 << 1; // underflow
pub const FFLAG_OF: u32 = 1 << 2; // overflow
pub const FFLAG_DZ: u32 = 1 << 3; // divide by zero
pub const FFLAG_NV: u32 = 1 << 4; // invalid operation

/// Canonical NaN bit patterns.
pub const CANONICAL_NAN_F32: u32 = 0x7FC0_0000;
pub const CANONICAL_NAN_F64: u64 = 0x7FF8_0000_0000_0000;

/// Rounding modes (rm field / frm).
pub const RM_RNE: u32 = 0; // round to nearest, ties to even
pub const RM_RTZ: u32 = 1; // round towards zero
pub const RM_RDN: u32 = 2; // round down
pub const RM_RUP: u32 = 3; // round up
pub const RM_RMM: u32 = 4; // round to nearest, ties to max magnitude
pub const RM_DYN: u32 = 7; // use frm

// ============================================================================
// NaN boxing
// ============================================================================

/// Read an f32 operand from an FP register, honoring NaN boxing: a value
/// that is not properly boxed reads as the canonical NaN.
#[inline(always)]
pub fn unbox_f32(bits: u64) -> f32 {
    if bits >> 32 == 0xFFFF_FFFF {
        f32::from_bits(bits as u32)
    } else {
        f32::from_bits(CANONICAL_NAN_F32)
    }
}

/// NaN-box an f32 result for storage in the 64-bit register file.
#[inline(always)]
pub fn box_f32(value: f32) -> u64 {
    0xFFFF_FFFF_0000_0000 | value.to_bits() as u64
}

/// Canonicalize NaN results (RISC-V requires the canonical NaN payload).
#[inline(always)]
pub fn canonical_f32(value: f32) -> f32 {
    if value.is_nan() {
        f32::from_bits(CANONICAL_NAN_F32)
    } else {
        value
    }
}

#[inline(always)]
pub fn canonical_f64(value: f64) -> f64 {
    if value.is_nan() {
        f64::from_bits(CANONICAL_NAN_F64)
    } else {
        value
    }
}

#[inline(always)]
fn is_snan_f32(v: f32) -> bool {
    let b = v.to_bits();
    v.is_nan() && (b & 0x0040_0000) == 0
}

#[inline(always)]
fn is_snan_f64(v: f64) -> bool {
    let b = v.to_bits();
    v.is_nan() && (b & 0x0008_0000_0000_0000) == 0
}

// ============================================================================
// Arithmetic with flag reporting
// ============================================================================

/// Flags produced by a binary f32 op with a possibly-NaN result.
#[inline]
fn arith_flags_f32(a: f32, b: f32, result: f32) -> u32 {
    let mut flags = 0;
    if result.is_nan() && (is_snan_f32(a) || is_snan_f32(b) || (!a.is_nan() && !b.is_nan())) {
        flags |= FFLAG_NV;
    }
    if result.is_infinite() && !a.is_infinite() && !b.is_infinite() {
        flags |= FFLAG_OF | FFLAG_NX;
    }
    flags
}

#[inline]
fn arith_flags_f64(a: f64, b: f64, result: f64) -> u32 {
    let mut flags = 0;
    if result.is_nan() && (is_snan_f64(a) || is_snan_f64(b) || (!a.is_nan() && !b.is_nan())) {
        flags |= FFLAG_NV;
    }
    if result.is_infinite() && !a.is_infinite() && !b.is_infinite() {
        flags |= FFLAG_OF | FFLAG_NX;
    }
    flags
}

macro_rules! binop_f32 {
    ($name:ident, $op:tt) => {
        #[inline]
        pub fn $name(a: f32, b: f32) -> (f32, u32) {
            let r = a $op b;
            (canonical_f32(r), arith_flags_f32(a, b, r))
        }
    };
}
macro_rules! binop_f64 {
    ($name:ident, $op:tt) => {
        #[inline]
        pub fn $name(a: f64, b: f64) -> (f64, u32) {
            let r = a $op b;
            (canonical_f64(r), arith_flags_f64(a, b, r))
        }
    };
}

binop_f32!(fadd_s, +);
binop_f32!(fsub_s, -);
binop_f32!(fmul_s, *);
binop_f64!(fadd_d, +);
binop_f64!(fsub_d, -);
binop_f64!(fmul_d, *);

#[inline]
pub fn fdiv_s(a: f32, b: f32) -> (f32, u32) {
    let r = a / b;
    let mut flags = arith_flags_f32(a, b, r);
    if b == 0.0 && !a.is_nan() && a != 0.0 && !a.is_infinite() {
        flags |= FFLAG_DZ;
    }
    (canonical_f32(r), flags)
}

#[inline]
pub fn fdiv_d(a: f64, b: f64) -> (f64, u32) {
    let r = a / b;
    let mut flags = arith_flags_f64(a, b, r);
    if b == 0.0 && !a.is_nan() && a != 0.0 && !a.is_infinite() {
        flags |= FFLAG_DZ;
    }
    (canonical_f64(r), flags)
}

#[inline]
pub fn fsqrt_s(a: f32) -> (f32, u32) {
    let r = a.sqrt();
    let mut flags = 0;
    if a < 0.0 || is_snan_f32(a) {
        flags |= FFLAG_NV;
    }
    (canonical_f32(r), flags)
}

#[inline]
pub fn fsqrt_d(a: f64) -> (f64, u32) {
    let r = a.sqrt();
    let mut flags = 0;
    if a < 0.0 || is_snan_f64(a) {
        flags |= FFLAG_NV;
    }
    (canonical_f64(r), flags)
}

/// Fused multiply-add: (a * b) + c with a single rounding.
#[inline]
pub fn fmadd_s(a: f32, b: f32, c: f32) -> (f32, u32) {
    let r = a.mul_add(b, c);
    let mut flags = 0;
    if r.is_nan() {
        // Invalid if inf * 0 or any signaling NaN input.
        if (a.is_infinite() && b == 0.0)
            || (b.is_infinite() && a == 0.0)
            || is_snan_f32(a)
            || is_snan_f32(b)
            || is_snan_f32(c)
            || (!a.is_nan() && !b.is_nan() && !c.is_nan())
        {
            flags |= FFLAG_NV;
        }
    }
    (canonical_f32(r), flags)
}

#[inline]
pub fn fmadd_d(a: f64, b: f64, c: f64) -> (f64, u32) {
    let r = a.mul_add(b, c);
    let mut flags = 0;
    if r.is_nan() {
        if (a.is_infinite() && b == 0.0)
            || (b.is_infinite() && a == 0.0)
            || is_snan_f64(a)
            || is_snan_f64(b)
            || is_snan_f64(c)
            || (!a.is_nan() && !b.is_nan() && !c.is_nan())
        {
            flags |= FFLAG_NV;
        }
    }
    (canonical_f64(r), flags)
}

/// FMIN/FMAX with RISC-V NaN semantics: if one operand is NaN, return the
/// other; if both are NaN, return canonical NaN. -0.0 < +0.0.
#[inline]
pub fn fmin_s(a: f32, b: f32) -> (f32, u32) {
    let flags = if is_snan_f32(a) || is_snan_f32(b) { FFLAG_NV } else { 0 };
    let r = match (a.is_nan(), b.is_nan()) {
        (true, true) => f32::from_bits(CANONICAL_NAN_F32),
        (true, false) => b,
        (false, true) => a,
        (false, false) => {
            if a == 0.0 && b == 0.0 {
                if a.is_sign_negative() { a } else { b }
            } else {
                a.min(b)
            }
        }
    };
    (r, flags)
}

#[inline]
pub fn fmax_s(a: f32, b: f32) -> (f32, u32) {
    let flags = if is_snan_f32(a) || is_snan_f32(b) { FFLAG_NV } else { 0 };
    let r = match (a.is_nan(), b.is_nan()) {
        (true, true) => f32::from_bits(CANONICAL_NAN_F32),
        (true, false) => b,
        (false, true) => a,
        (false, false) => {
            if a == 0.0 && b == 0.0 {
                if a.is_sign_positive() { a } else { b }
            } else {
                a.max(b)
            }
        }
    };
    (r, flags)
}

#[inline]
pub fn fmin_d(a: f64, b: f64) -> (f64, u32) {
    let flags = if is_snan_f64(a) || is_snan_f64(b) { FFLAG_NV } else { 0 };
    let r = match (a.is_nan(), b.is_nan()) {
        (true, true) => f64::from_bits(CANONICAL_NAN_F64),
        (true, false) => b,
        (false, true) => a,
        (false, false) => {
            if a == 0.0 && b == 0.0 {
                if a.is_sign_negative() { a } else { b }
            } else {
                a.min(b)
            }
        }
    };
    (r, flags)
}

#[inline]
pub fn fmax_d(a: f64, b: f64) -> (f64, u32) {
    let flags = if is_snan_f64(a) || is_snan_f64(b) { FFLAG_NV } else { 0 };
    let r = match (a.is_nan(), b.is_nan()) {
        (true, true) => f64::from_bits(CANONICAL_NAN_F64),
        (true, false) => b,
        (false, true) => a,
        (false, false) => {
            if a == 0.0 && b == 0.0 {
                if a.is_sign_positive() { a } else { b }
            } else {
                a.max(b)
            }
        }
    };
    (r, flags)
}

// ============================================================================
// Comparisons (results go to integer registers)
// ============================================================================

/// FEQ: quiet comparison; NV only on signaling NaN.
#[inline]
pub fn feq_s(a: f32, b: f32) -> (u64, u32) {
    let flags = if is_snan_f32(a) || is_snan_f32(b) { FFLAG_NV } else { 0 };
    ((a == b) as u64, flags)
}

/// FLT/FLE: signaling comparison; NV on any NaN.
#[inline]
pub fn flt_s(a: f32, b: f32) -> (u64, u32) {
    let flags = if a.is_nan() || b.is_nan() { FFLAG_NV } else { 0 };
    ((a < b) as u64, flags)
}

#[inline]
pub fn fle_s(a: f32, b: f32) -> (u64, u32) {
    let flags = if a.is_nan() || b.is_nan() { FFLAG_NV } else { 0 };
    ((a <= b) as u64, flags)
}

#[inline]
pub fn feq_d(a: f64, b: f64) -> (u64, u32) {
    let flags = if is_snan_f64(a) || is_snan_f64(b) { FFLAG_NV } else { 0 };
    ((a == b) as u64, flags)
}

#[inline]
pub fn flt_d(a: f64, b: f64) -> (u64, u32) {
    let flags = if a.is_nan() || b.is_nan() { FFLAG_NV } else { 0 };
    ((a < b) as u64, flags)
}

#[inline]
pub fn fle_d(a: f64, b: f64) -> (u64, u32) {
    let flags = if a.is_nan() || b.is_nan() { FFLAG_NV } else { 0 };
    ((a <= b) as u64, flags)
}

// ============================================================================
// Conversions
// ============================================================================

/// Apply a rounding mode to a float, returning the integral-valued float.
#[inline]
fn round_f64(value: f64, rm: u32) -> f64 {
    match rm {
        RM_RTZ => value.trunc(),
        RM_RDN => value.floor(),
        RM_RUP => value.ceil(),
        RM_RMM => value.round(), // ties away from zero
        // RNE (and unknown encodings): ties to even
        _ => {
            let r = value.round();
            if (value - value.trunc()).abs() == 0.5 {
                // Tie: round to even
                let t = value.trunc();
                if (t as i64) % 2 == 0 { t } else { r }
            } else {
                r
            }
        }
    }
}

/// Float -> signed integer conversion with RISC-V saturation semantics.
/// Returns (result, flags). NaN converts to i_max with NV.
#[inline]
pub fn fcvt_to_i64(value: f64, rm: u32, min: i64, max: i64) -> (i64, u32) {
    if value.is_nan() {
        return (max, FFLAG_NV);
    }
    let rounded = round_f64(value, rm);
    if rounded < min as f64 {
        (min, FFLAG_NV)
    } else if rounded > max as f64 {
        (max, FFLAG_NV)
    } else {
        let flags = if rounded != value { FFLAG_NX } else { 0 };
        (rounded as i64, flags)
    }
}

/// Float -> unsigned integer conversion with RISC-V saturation semantics.
#[inline]
pub fn fcvt_to_u64(value: f64, rm: u32, max: u64) -> (u64, u32) {
    if value.is_nan() {
        return (max, FFLAG_NV);
    }
    let rounded = round_f64(value, rm);
    if rounded < 0.0 {
        (0, FFLAG_NV)
    } else if rounded > max as f64 {
        (max, FFLAG_NV)
    } else {
        let flags = if rounded != value { FFLAG_NX } else { 0 };
        (rounded as u64, flags)
    }
}

// ============================================================================
// Sign injection
// ============================================================================

#[inline]
pub fn fsgnj_s(a: f32, b: f32, mode: u32) -> f32 {
    let abits = a.to_bits();
    let bsign = b.to_bits() & 0x8000_0000;
    let bits = match mode {
        0 => (abits & 0x7FFF_FFFF) | bsign,          // FSGNJ
        1 => (abits & 0x7FFF_FFFF) | (bsign ^ 0x8000_0000), // FSGNJN
        _ => abits ^ bsign,                          // FSGNJX
    };
    f32::from_bits(bits)
}

#[inline]
pub fn fsgnj_d(a: f64, b: f64, mode: u32) -> f64 {
    let abits = a.to_bits();
    let bsign = b.to_bits() & 0x8000_0000_0000_0000;
    let bits = match mode {
        0 => (abits & 0x7FFF_FFFF_FFFF_FFFF) | bsign,
        1 => (abits & 0x7FFF_FFFF_FFFF_FFFF) | (bsign ^ 0x8000_0000_0000_0000),
        _ => abits ^ bsign,
    };
    f64::from_bits(bits)
}

// ============================================================================
// Classification (FCLASS)
// ============================================================================

#[inline]
pub fn fclass_f32(v: f32) -> u64 {
    let bits = v.to_bits();
    let sign = bits >> 31 == 1;
    if v.is_nan() {
        if is_snan_f32(v) { 1 << 8 } else { 1 << 9 }
    } else if v.is_infinite() {
        if sign { 1 << 0 } else { 1 << 7 }
    } else if v == 0.0 {
        if sign { 1 << 3 } else { 1 << 4 }
    } else if v.is_subnormal() {
        if sign { 1 << 2 } else { 1 << 5 }
    } else if sign {
        1 << 1
    } else {
        1 << 6
    }
}

#[inline]
pub fn fclass_f64(v: f64) -> u64 {
    let bits = v.to_bits();
    let sign = bits >> 63 == 1;
    if v.is_nan() {
        if is_snan_f64(v) { 1 << 8 } else { 1 << 9 }
    } else if v.is_infinite() {
        if sign { 1 << 0 } else { 1 << 7 }
    } else if v == 0.0 {
        if sign { 1 << 3 } else { 1 << 4 }
    } else if v.is_subnormal() {
        if sign { 1 << 2 } else { 1 << 5 }
    } else if sign {
        1 << 1
    } else {
        1 << 6
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nan_boxing() {
        let boxed = box_f32(1.5);
        assert_eq!(boxed >> 32, 0xFFFF_FFFF);
        assert_eq!(unbox_f32(boxed), 1.5);
        // Improperly boxed value reads as canonical NaN
        assert!(unbox_f32(1.5f64.to_bits()).is_nan());
    }

    #[test]
    fn test_canonical_nan() {
        let (r, flags) = fadd_s(f32::INFINITY, f32::NEG_INFINITY);
        assert_eq!(r.to_bits(), CANONICAL_NAN_F32);
        assert!(flags & FFLAG_NV != 0);
    }

    #[test]
    fn test_div_by_zero() {
        let (r, flags) = fdiv_d(1.0, 0.0);
        assert!(r.is_infinite());
        assert!(flags & FFLAG_DZ != 0);
    }

    #[test]
    fn test_min_max_nan_semantics() {
        let (r, _) = fmin_s(f32::NAN, 2.0);
        assert_eq!(r, 2.0);
        let (r, _) = fmax_d(f64::NAN, f64::NAN);
        assert_eq!(r.to_bits(), CANONICAL_NAN_F64);
        // -0.0 vs +0.0
        let (r, _) = fmin_s(0.0, -0.0);
        assert!(r.is_sign_negative());
    }

    #[test]
    fn test_conversions() {
        // RTZ truncation
        let (v, _) = fcvt_to_i64(2.7, RM_RTZ, i32::MIN as i64, i32::MAX as i64);
        assert_eq!(v, 2);
        let (v, _) = fcvt_to_i64(-2.7, RM_RTZ, i32::MIN as i64, i32::MAX as i64);
        assert_eq!(v, -2);
        // NaN saturates to max with NV
        let (v, f) = fcvt_to_i64(f64::NAN, RM_RTZ, i32::MIN as i64, i32::MAX as i64);
        assert_eq!(v, i32::MAX as i64);
        assert!(f & FFLAG_NV != 0);
        // Overflow saturates
        let (v, f) = fcvt_to_i64(1e20, RM_RTZ, i32::MIN as i64, i32::MAX as i64);
        assert_eq!(v, i32::MAX as i64);
        assert!(f & FFLAG_NV != 0);
        // Unsigned negative saturates to 0
        let (v, f) = fcvt_to_u64(-1.5, RM_RTZ, u32::MAX as u64);
        assert_eq!(v, 0);
        assert!(f & FFLAG_NV != 0);
        // RNE ties to even
        let (v, _) = fcvt_to_i64(2.5, RM_RNE, i64::MIN, i64::MAX);
        assert_eq!(v, 2);
        let (v, _) = fcvt_to_i64(3.5, RM_RNE, i64::MIN, i64::MAX);
        assert_eq!(v, 4);
    }

    #[test]
    fn test_fclass() {
        assert_eq!(fclass_f32(f32::NEG_INFINITY), 1 << 0);
        assert_eq!(fclass_f32(-1.0), 1 << 1);
        assert_eq!(fclass_f32(-0.0), 1 << 3);
        assert_eq!(fclass_f32(0.0), 1 << 4);
        assert_eq!(fclass_f32(1.0), 1 << 6);
        assert_eq!(fclass_f32(f32::INFINITY), 1 << 7);
        assert_eq!(fclass_f64(f64::from_bits(CANONICAL_NAN_F64)), 1 << 9);
    }

    #[test]
    fn test_sign_injection() {
        assert_eq!(fsgnj_s(1.5, -2.0, 0), -1.5); // FSGNJ
        assert_eq!(fsgnj_s(1.5, -2.0, 1), 1.5); // FSGNJN
        assert_eq!(fsgnj_s(-1.5, -2.0, 2), 1.5); // FSGNJX
        assert_eq!(fsgnj_d(-3.0, 1.0, 0), 3.0);
    }
}
