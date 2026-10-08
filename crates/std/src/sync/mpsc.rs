//! Channels between parts of the one guest thread.
//!
//! A message waits in a queue the senders and the receiver share. The one guest thread is both
//! the sender and the receiver, so a receive finds every message sent before it, and a receive
//! on an empty queue answers at once: the one thread that fills the queue is the receiving one.

use alloc_crate::{collections::VecDeque, sync::Arc};
use core::{fmt, time::Duration};

use super::Mutex;

/// The queue and the end counts a channel shares.
struct Shared<T> {
    queue: VecDeque<T>,
    senders: usize,
    receiver: bool,
}

/// Creates a channel: the sending half and the receiving half.
#[must_use]
pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let shared = Arc::new(Mutex::new(Shared {
        queue: VecDeque::new(),
        senders: 1,
        receiver: true,
    }));
    (
        Sender {
            shared: shared.clone(),
        },
        Receiver { shared },
    )
}

/// The sending half of a channel.
pub struct Sender<T> {
    shared: Arc<Mutex<Shared<T>>>,
}

impl<T> Sender<T> {
    /// Queues `value`; a channel whose receiver is gone hands it back.
    pub fn send(&self, value: T) -> Result<(), SendError<T>> {
        let mut shared = self.shared.lock().unwrap();
        if !shared.receiver {
            return Err(SendError(value));
        }
        shared.queue.push_back(value);
        Ok(())
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        self.shared.lock().unwrap().senders += 1;
        Self {
            shared: self.shared.clone(),
        }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        self.shared.lock().unwrap().senders -= 1;
    }
}

impl<T> fmt::Debug for Sender<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sender").finish_non_exhaustive()
    }
}

/// The receiving half of a channel.
pub struct Receiver<T> {
    shared: Arc<Mutex<Shared<T>>>,
}

impl<T> Receiver<T> {
    /// Takes the oldest queued message.
    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        let mut shared = self.shared.lock().unwrap();
        match shared.queue.pop_front() {
            Some(value) => Ok(value),
            None if shared.senders == 0 => Err(TryRecvError::Disconnected),
            None => Err(TryRecvError::Empty),
        }
    }

    /// Takes the oldest queued message; an empty queue answers [`RecvError`].
    pub fn recv(&self) -> Result<T, RecvError> {
        self.try_recv().map_err(|_| RecvError)
    }

    /// Takes the oldest queued message; an empty queue waits out `timeout` first.
    pub fn recv_timeout(&self, timeout: Duration) -> Result<T, RecvTimeoutError> {
        match self.try_recv() {
            Ok(value) => Ok(value),
            Err(TryRecvError::Disconnected) => Err(RecvTimeoutError::Disconnected),
            Err(TryRecvError::Empty) => {
                crate::thread::sleep(timeout);
                self.try_recv().map_err(|error| match error {
                    TryRecvError::Empty => RecvTimeoutError::Timeout,
                    TryRecvError::Disconnected => RecvTimeoutError::Disconnected,
                })
            }
        }
    }

    /// Iterates over the queued messages.
    #[must_use]
    pub fn try_iter(&self) -> TryIter<'_, T> {
        TryIter { receiver: self }
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        self.shared.lock().unwrap().receiver = false;
    }
}

impl<T> fmt::Debug for Receiver<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Receiver").finish_non_exhaustive()
    }
}

/// The queued messages of a [`Receiver`].
pub struct TryIter<'a, T> {
    receiver: &'a Receiver<T>,
}

impl<T> Iterator for TryIter<'_, T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.receiver.try_recv().ok()
    }
}

/// A send to a channel whose receiver is gone; it carries the message back.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SendError<T>(pub T);

impl<T> fmt::Debug for SendError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SendError { .. }")
    }
}

impl<T> fmt::Display for SendError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("sending on a closed channel")
    }
}

impl<T> core::error::Error for SendError<T> {}

/// A receive on an empty queue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecvError;

impl fmt::Display for RecvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("receiving on an empty channel")
    }
}

impl core::error::Error for RecvError {}

/// The state of an empty queue [`Receiver::try_recv`] reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TryRecvError {
    /// The queue is empty and a sender remains.
    Empty,
    /// The queue is empty and every sender is gone.
    Disconnected,
}

impl fmt::Display for TryRecvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Empty => "receiving on an empty channel",
            Self::Disconnected => "receiving on a closed channel",
        })
    }
}

impl core::error::Error for TryRecvError {}

/// The state of an empty queue [`Receiver::recv_timeout`] reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecvTimeoutError {
    /// The timeout passed with the queue empty.
    Timeout,
    /// The queue is empty and every sender is gone.
    Disconnected,
}

impl fmt::Display for RecvTimeoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Timeout => "timed out waiting on a channel",
            Self::Disconnected => "receiving on a closed channel",
        })
    }
}

impl core::error::Error for RecvTimeoutError {}
