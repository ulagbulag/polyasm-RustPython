//! Clocks of the embedder host: [`Instant`] and [`SystemTime`].

use core::{
    error, fmt,
    ops::{Add, AddAssign, Sub, SubAssign},
};

pub use core::time::{Duration, TryFromFloatSecsError};

use crate::{consts::NANOS_PER_SEC, host::host};

/// A reading of the monotonic clock of the host.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Instant(Duration);

/// A reading of the wall clock of the host.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SystemTime {
    nanos: i128,
}

/// The error [`SystemTime::duration_since`] answers when the other time is later.
#[derive(Clone, Debug)]
pub struct SystemTimeError(Duration);

/// The Unix epoch, 1970-01-01 00:00:00 UTC.
pub const UNIX_EPOCH: SystemTime = SystemTime { nanos: 0 };

fn duration_nanos(duration: Duration) -> i128 {
    i128::try_from(duration.as_nanos()).unwrap_or(i128::MAX)
}

fn nanos_duration(nanos: i128) -> Option<Duration> {
    let nanos = u128::try_from(nanos).ok()?;
    let per_sec = NANOS_PER_SEC as u128;
    let secs = u64::try_from(nanos / per_sec).ok()?;
    Some(Duration::new(secs, (nanos % per_sec) as u32))
}

impl Instant {
    /// The current reading of the monotonic clock.
    #[must_use]
    pub fn now() -> Self {
        Self(
            host()
                .monotonic()
                .unwrap_or_else(|e| panic!("the monotonic clock of the host answers: {e}")),
        )
    }

    #[must_use]
    pub fn duration_since(&self, earlier: Self) -> Duration {
        self.saturating_duration_since(earlier)
    }

    #[must_use]
    pub fn checked_duration_since(&self, earlier: Self) -> Option<Duration> {
        self.0.checked_sub(earlier.0)
    }

    #[must_use]
    pub fn saturating_duration_since(&self, earlier: Self) -> Duration {
        self.0.saturating_sub(earlier.0)
    }

    #[must_use]
    pub fn elapsed(&self) -> Duration {
        Self::now().duration_since(*self)
    }

    #[must_use]
    pub fn checked_add(&self, duration: Duration) -> Option<Self> {
        self.0.checked_add(duration).map(Self)
    }

    #[must_use]
    pub fn checked_sub(&self, duration: Duration) -> Option<Self> {
        self.0.checked_sub(duration).map(Self)
    }
}

impl Add<Duration> for Instant {
    type Output = Self;

    fn add(self, other: Duration) -> Self {
        self.checked_add(other)
            .expect("overflow when adding duration to instant")
    }
}

impl AddAssign<Duration> for Instant {
    fn add_assign(&mut self, other: Duration) {
        *self = *self + other;
    }
}

impl Sub<Duration> for Instant {
    type Output = Self;

    fn sub(self, other: Duration) -> Self {
        self.checked_sub(other)
            .expect("overflow when subtracting duration from instant")
    }
}

impl SubAssign<Duration> for Instant {
    fn sub_assign(&mut self, other: Duration) {
        *self = *self - other;
    }
}

impl Sub<Self> for Instant {
    type Output = Duration;

    fn sub(self, other: Self) -> Duration {
        self.duration_since(other)
    }
}

impl fmt::Debug for Instant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl SystemTime {
    pub const UNIX_EPOCH: Self = UNIX_EPOCH;

    /// The current reading of the wall clock.
    #[must_use]
    pub fn now() -> Self {
        let since_epoch = host()
            .wall_clock()
            .unwrap_or_else(|e| panic!("the wall clock of the host answers: {e}"));
        Self {
            nanos: duration_nanos(since_epoch),
        }
    }

    /// The time `nanos` nanoseconds after the Unix epoch; a negative count lies before it.
    #[must_use]
    pub(crate) const fn from_unix_nanos(nanos: i64) -> Self {
        Self {
            nanos: nanos as i128,
        }
    }

    /// The span from `earlier` to `self`.
    pub fn duration_since(&self, earlier: Self) -> Result<Duration, SystemTimeError> {
        let diff = self.nanos - earlier.nanos;
        if diff >= 0 {
            Ok(nanos_duration(diff).unwrap_or(Duration::MAX))
        } else {
            Err(SystemTimeError(
                nanos_duration(-diff).unwrap_or(Duration::MAX),
            ))
        }
    }

    pub fn elapsed(&self) -> Result<Duration, SystemTimeError> {
        Self::now().duration_since(*self)
    }

    #[must_use]
    pub fn checked_add(&self, duration: Duration) -> Option<Self> {
        self.nanos
            .checked_add(duration_nanos(duration))
            .map(|nanos| Self { nanos })
    }

    #[must_use]
    pub fn checked_sub(&self, duration: Duration) -> Option<Self> {
        self.nanos
            .checked_sub(duration_nanos(duration))
            .map(|nanos| Self { nanos })
    }
}

impl Add<Duration> for SystemTime {
    type Output = Self;

    fn add(self, duration: Duration) -> Self {
        self.checked_add(duration)
            .expect("overflow when adding duration to instant")
    }
}

impl AddAssign<Duration> for SystemTime {
    fn add_assign(&mut self, other: Duration) {
        *self = *self + other;
    }
}

impl Sub<Duration> for SystemTime {
    type Output = Self;

    fn sub(self, duration: Duration) -> Self {
        self.checked_sub(duration)
            .expect("overflow when subtracting duration from instant")
    }
}

impl SubAssign<Duration> for SystemTime {
    fn sub_assign(&mut self, other: Duration) {
        *self = *self - other;
    }
}

impl fmt::Debug for SystemTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SystemTime")
            .field("nanos_since_epoch", &self.nanos)
            .finish()
    }
}

impl SystemTimeError {
    /// How much later the other time is.
    #[must_use]
    pub const fn duration(&self) -> Duration {
        self.0
    }
}

impl fmt::Display for SystemTimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("second time provided was later than self")
    }
}

impl error::Error for SystemTimeError {}
