//! Hexadecimal float literals (`[+-]0x<hex>[.<hex>]p[+-]<dec>`), parsed exactly into `f64`.
//!
//! The grammar and the exactness rules follow the CC0 `hexf-parse` crate: a literal whose value
//! `f64` holds inexactly is an error.

/// Why a literal fails to parse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseHexfError {
    /// The input holds no characters.
    Empty,
    /// The input breaks the grammar.
    Invalid,
    /// The value lies outside what `f64` holds exactly.
    Inexact,
}

/// Reads one hexadecimal digit.
const fn hex_digit(c: u8) -> Option<u64> {
    match c {
        b'0'..=b'9' => Some((c - b'0') as u64),
        b'a'..=b'f' => Some((c - b'a' + 10) as u64),
        b'A'..=b'F' => Some((c - b'A' + 10) as u64),
        _ => None,
    }
}

/// Splits a literal into its sign, its mantissa and its binary exponent.
fn parse(s: &[u8]) -> Result<(bool, u64, isize), ParseHexfError> {
    let (negative, s) = match s.split_first() {
        Some((b'+', rest)) => (false, rest),
        Some((b'-', rest)) => (true, rest),
        Some(_) => (false, s),
        None => return Err(ParseHexfError::Empty),
    };
    let mut s = s
        .strip_prefix(b"0x")
        .or_else(|| s.strip_prefix(b"0X"))
        .ok_or(ParseHexfError::Invalid)?;

    let mut acc = 0u64;
    let mut digit_seen = false;
    while let Some(digit) = s.first().copied().and_then(hex_digit) {
        s = &s[1..];
        digit_seen = true;
        if acc >> 60 != 0 {
            return Err(ParseHexfError::Inexact);
        }
        acc = (acc << 4) | digit;
    }

    // Trailing zero digits stay counted apart, so that they shift the mantissa only when a
    // nonzero digit follows them.
    let mut nfracs = 0isize;
    let mut nzeroes = 0isize;
    let mut frac_digit_seen = false;
    if let Some(rest) = s.strip_prefix(b".") {
        s = rest;
        while let Some(digit) = s.first().copied().and_then(hex_digit) {
            s = &s[1..];
            frac_digit_seen = true;
            if digit == 0 {
                nzeroes = nzeroes.checked_add(1).ok_or(ParseHexfError::Inexact)?;
                continue;
            }
            let nnewdigits = nzeroes.checked_add(1).ok_or(ParseHexfError::Inexact)?;
            nfracs = nfracs
                .checked_add(nnewdigits)
                .ok_or(ParseHexfError::Inexact)?;
            nzeroes = 0;
            if acc != 0 {
                if nnewdigits >= 16 || acc >> (64 - nnewdigits * 4) != 0 {
                    return Err(ParseHexfError::Inexact);
                }
                acc <<= nnewdigits * 4;
            }
            acc |= digit;
        }
    }
    if !(digit_seen || frac_digit_seen) {
        return Err(ParseHexfError::Invalid);
    }

    let s = match s.split_first() {
        Some((b'p' | b'P', rest)) => rest,
        _ => return Err(ParseHexfError::Invalid),
    };
    let (negative_exponent, mut s) = match s.split_first() {
        Some((b'+', rest)) => (false, rest),
        Some((b'-', rest)) => (true, rest),
        Some(_) => (false, s),
        None => return Err(ParseHexfError::Invalid),
    };
    if s.is_empty() {
        return Err(ParseHexfError::Invalid);
    }
    let mut exponent = 0isize;
    while let Some((&c, rest)) = s.split_first() {
        if !c.is_ascii_digit() {
            return Err(ParseHexfError::Invalid);
        }
        s = rest;
        // A zero mantissa reads every exponent as zero.
        if acc != 0 {
            exponent = exponent
                .checked_mul(10)
                .and_then(|v| v.checked_add((c - b'0') as isize))
                .ok_or(ParseHexfError::Inexact)?;
        }
    }
    if negative_exponent {
        exponent = -exponent;
    }

    if acc == 0 {
        return Ok((negative, 0, 0));
    }
    let exponent = nfracs
        .checked_mul(4)
        .and_then(|v| exponent.checked_sub(v))
        .ok_or(ParseHexfError::Inexact)?;
    Ok((negative, acc, exponent))
}

/// `2^exponent` for an exponent in the normal range of `f64`.
fn pow2(exponent: isize) -> f64 {
    f64::from_bits(((exponent + 1023) as u64) << 52)
}

/// Builds the `f64` of `mantissa * 2^exponent`, exactly.
fn convert(negative: bool, mantissa: u64, exponent: isize) -> Result<f64, ParseHexfError> {
    if !(-0xffff..=0xffff).contains(&exponent) {
        return Err(ParseHexfError::Inexact);
    }
    let trailing = mantissa.trailing_zeros() & 63;
    let mantissa = mantissa >> trailing;
    let exponent = exponent + trailing as isize;

    let leading = mantissa.leading_zeros();
    let normalexp = exponent + (63 - leading as isize);
    let mantissa_digits = f64::MANTISSA_DIGITS as isize;
    let mantissasize = if normalexp < f64::MIN_EXP as isize - mantissa_digits {
        return Err(ParseHexfError::Inexact);
    } else if normalexp < (f64::MIN_EXP - 1) as isize {
        mantissa_digits - f64::MIN_EXP as isize + normalexp + 1
    } else if normalexp < f64::MAX_EXP as isize {
        mantissa_digits
    } else {
        return Err(ParseHexfError::Inexact);
    };
    if mantissa >> mantissasize != 0 {
        return Err(ParseHexfError::Inexact);
    }

    let mut value = mantissa as f64;
    if negative {
        value = -value;
    }
    // Two power-of-two factors keep every intermediate product normal, so each multiplication
    // stays exact.
    let half = exponent / 2;
    Ok(value * pow2(half) * pow2(exponent - half))
}

/// Parses a hexadecimal float literal into the `f64` it names exactly.
pub fn parse_hexf64(s: &str) -> Result<f64, ParseHexfError> {
    let (negative, mantissa, exponent) = parse(s.as_bytes())?;
    convert(negative, mantissa, exponent)
}
