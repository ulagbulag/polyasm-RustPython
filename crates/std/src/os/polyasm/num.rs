//! The float math std defines on `f32` and `f64` outside `core`, as a prelude trait.
//!
//! [`FloatMath`] sits in the prelude, so `x.sqrt()`, `x.powf(y)` and the rest resolve on
//! PolyASM as they do with std; `libm` computes them.

/// Float math of std beyond `core`.
pub trait FloatMath: Copy {
    #[must_use]
    fn floor(self) -> Self;
    #[must_use]
    fn ceil(self) -> Self;
    #[must_use]
    fn round(self) -> Self;
    #[must_use]
    fn round_ties_even(self) -> Self;
    #[must_use]
    fn trunc(self) -> Self;
    #[must_use]
    fn fract(self) -> Self;
    #[must_use]
    fn mul_add(self, a: Self, b: Self) -> Self;
    #[must_use]
    fn div_euclid(self, rhs: Self) -> Self;
    #[must_use]
    fn rem_euclid(self, rhs: Self) -> Self;
    #[must_use]
    fn powi(self, n: i32) -> Self;
    #[must_use]
    fn powf(self, n: Self) -> Self;
    #[must_use]
    fn sqrt(self) -> Self;
    #[must_use]
    fn exp(self) -> Self;
    #[must_use]
    fn exp2(self) -> Self;
    #[must_use]
    fn ln(self) -> Self;
    #[must_use]
    fn log(self, base: Self) -> Self;
    #[must_use]
    fn log2(self) -> Self;
    #[must_use]
    fn log10(self) -> Self;
    #[must_use]
    fn cbrt(self) -> Self;
    #[must_use]
    fn hypot(self, other: Self) -> Self;
    #[must_use]
    fn sin(self) -> Self;
    #[must_use]
    fn cos(self) -> Self;
    #[must_use]
    fn tan(self) -> Self;
    #[must_use]
    fn asin(self) -> Self;
    #[must_use]
    fn acos(self) -> Self;
    #[must_use]
    fn atan(self) -> Self;
    #[must_use]
    fn atan2(self, other: Self) -> Self;
    #[must_use]
    fn sin_cos(self) -> (Self, Self);
    #[must_use]
    fn exp_m1(self) -> Self;
    #[must_use]
    fn ln_1p(self) -> Self;
    #[must_use]
    fn sinh(self) -> Self;
    #[must_use]
    fn cosh(self) -> Self;
    #[must_use]
    fn tanh(self) -> Self;
    #[must_use]
    fn asinh(self) -> Self;
    #[must_use]
    fn acosh(self) -> Self;
    #[must_use]
    fn atanh(self) -> Self;
}

macro_rules! float_math {
    ($t:ty, $one:expr, $zero:expr,
     floor = $floor:path, ceil = $ceil:path, round = $round:path, rint = $rint:path,
     trunc = $trunc:path, fma = $fma:path, fmod = $fmod:path, pow = $pow:path,
     sqrt = $sqrt:path, exp = $exp:path, exp2 = $exp2:path, log = $log:path,
     log2 = $log2:path, log10 = $log10:path, cbrt = $cbrt:path, hypot = $hypot:path,
     sin = $sin:path, cos = $cos:path, tan = $tan:path, asin = $asin:path, acos = $acos:path,
     atan = $atan:path, atan2 = $atan2:path, expm1 = $expm1:path, log1p = $log1p:path,
     sinh = $sinh:path, cosh = $cosh:path, tanh = $tanh:path, asinh = $asinh:path,
     acosh = $acosh:path, atanh = $atanh:path $(,)?) => {
        impl FloatMath for $t {
            fn floor(self) -> Self {
                $floor(self)
            }

            fn ceil(self) -> Self {
                $ceil(self)
            }

            fn round(self) -> Self {
                $round(self)
            }

            fn round_ties_even(self) -> Self {
                $rint(self)
            }

            fn trunc(self) -> Self {
                $trunc(self)
            }

            fn fract(self) -> Self {
                self - $trunc(self)
            }

            fn mul_add(self, a: Self, b: Self) -> Self {
                $fma(self, a, b)
            }

            fn div_euclid(self, rhs: Self) -> Self {
                let q = $trunc(self / rhs);
                if $fmod(self, rhs) < $zero {
                    return if rhs > $zero { q - $one } else { q + $one };
                }
                q
            }

            fn rem_euclid(self, rhs: Self) -> Self {
                let r = $fmod(self, rhs);
                if r < $zero { r + rhs.abs() } else { r }
            }

            fn powi(self, n: i32) -> Self {
                let mut base = self;
                let mut exp = n.unsigned_abs();
                let mut acc = $one;
                while exp > 0 {
                    if exp & 1 == 1 {
                        acc *= base;
                    }
                    exp >>= 1;
                    base *= base;
                }
                if n < 0 { $one / acc } else { acc }
            }

            fn powf(self, n: Self) -> Self {
                $pow(self, n)
            }

            fn sqrt(self) -> Self {
                $sqrt(self)
            }

            fn exp(self) -> Self {
                $exp(self)
            }

            fn exp2(self) -> Self {
                $exp2(self)
            }

            fn ln(self) -> Self {
                $log(self)
            }

            fn log(self, base: Self) -> Self {
                $log(self) / $log(base)
            }

            fn log2(self) -> Self {
                $log2(self)
            }

            fn log10(self) -> Self {
                $log10(self)
            }

            fn cbrt(self) -> Self {
                $cbrt(self)
            }

            fn hypot(self, other: Self) -> Self {
                $hypot(self, other)
            }

            fn sin(self) -> Self {
                $sin(self)
            }

            fn cos(self) -> Self {
                $cos(self)
            }

            fn tan(self) -> Self {
                $tan(self)
            }

            fn asin(self) -> Self {
                $asin(self)
            }

            fn acos(self) -> Self {
                $acos(self)
            }

            fn atan(self) -> Self {
                $atan(self)
            }

            fn atan2(self, other: Self) -> Self {
                $atan2(self, other)
            }

            fn sin_cos(self) -> (Self, Self) {
                ($sin(self), $cos(self))
            }

            fn exp_m1(self) -> Self {
                $expm1(self)
            }

            fn ln_1p(self) -> Self {
                $log1p(self)
            }

            fn sinh(self) -> Self {
                $sinh(self)
            }

            fn cosh(self) -> Self {
                $cosh(self)
            }

            fn tanh(self) -> Self {
                $tanh(self)
            }

            fn asinh(self) -> Self {
                $asinh(self)
            }

            fn acosh(self) -> Self {
                $acosh(self)
            }

            fn atanh(self) -> Self {
                $atanh(self)
            }
        }
    };
}

float_math!(
    f64,
    1.0,
    0.0,
    floor = libm::floor,
    ceil = libm::ceil,
    round = libm::round,
    rint = libm::rint,
    trunc = libm::trunc,
    fma = libm::fma,
    fmod = libm::fmod,
    pow = libm::pow,
    sqrt = libm::sqrt,
    exp = libm::exp,
    exp2 = libm::exp2,
    log = libm::log,
    log2 = libm::log2,
    log10 = libm::log10,
    cbrt = libm::cbrt,
    hypot = libm::hypot,
    sin = libm::sin,
    cos = libm::cos,
    tan = libm::tan,
    asin = libm::asin,
    acos = libm::acos,
    atan = libm::atan,
    atan2 = libm::atan2,
    expm1 = libm::expm1,
    log1p = libm::log1p,
    sinh = libm::sinh,
    cosh = libm::cosh,
    tanh = libm::tanh,
    asinh = libm::asinh,
    acosh = libm::acosh,
    atanh = libm::atanh,
);

float_math!(
    f32,
    1.0,
    0.0,
    floor = libm::floorf,
    ceil = libm::ceilf,
    round = libm::roundf,
    rint = libm::rintf,
    trunc = libm::truncf,
    fma = libm::fmaf,
    fmod = libm::fmodf,
    pow = libm::powf,
    sqrt = libm::sqrtf,
    exp = libm::expf,
    exp2 = libm::exp2f,
    log = libm::logf,
    log2 = libm::log2f,
    log10 = libm::log10f,
    cbrt = libm::cbrtf,
    hypot = libm::hypotf,
    sin = libm::sinf,
    cos = libm::cosf,
    tan = libm::tanf,
    asin = libm::asinf,
    acos = libm::acosf,
    atan = libm::atanf,
    atan2 = libm::atan2f,
    expm1 = libm::expm1f,
    log1p = libm::log1pf,
    sinh = libm::sinhf,
    cosh = libm::coshf,
    tanh = libm::tanhf,
    asinh = libm::asinhf,
    acosh = libm::acoshf,
    atanh = libm::atanhf,
);
