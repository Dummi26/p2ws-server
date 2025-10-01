use std::time::Duration;

use tokio::time::Instant;

#[derive(Clone, Copy)]
pub struct RatelimitSettings {
    time_per_message: Duration,
    burst_size: u32,
    drop_instead_of_blocking: bool,
}

impl RatelimitSettings {
    pub fn new(time_per_message: Duration) -> Self {
        Self {
            time_per_message,
            burst_size: 1,
            drop_instead_of_blocking: false,
        }
    }
    /// Allow this many messages in a row without any delay inbetween
    /// and without the ratelimit being triggered (but it needs to
    /// be charged up by sending messages at a rate below the limit
    /// before the burst). A value of `0` and a value of `1` do the same
    /// thing, allowing one message (or, no burst) before applying the ratelimit.
    pub fn allow_bursts(mut self, burst_size: u32) -> Self {
        self.burst_size = burst_size.max(1);
        self
    }
    /// When the ratelimit is activated, ignore the newest received message
    /// instead of waiting until enough time has elapsed.
    /// This will cause packet loss in the server, but will decrease the chance
    /// that the buffers storing the received bytes will fill up.
    /// Enabling this might be useful if clients are sending a lot of data.
    pub fn drop_instead_of_blocking(mut self) -> Self {
        self.drop_instead_of_blocking = true;
        self
    }
    /// This is the inverse of `drop_instead_of_blocking()`.
    pub fn block_instead_of_dropping(mut self) -> Self {
        self.drop_instead_of_blocking = false;
        self
    }
}

pub struct Ratelimiter {
    /// NOTE: this can be a time in the future.
    /// Do not assume `Instant::now() >= last_message.unwrap()`.
    last_message: Option<Instant>,
    time_per_message: Duration,
    burst_size: u32,
    drop_instead_of_blocking: bool,
}

impl RatelimitSettings {
    pub fn ratelimiter(&self) -> Ratelimiter {
        Ratelimiter {
            last_message: None,
            time_per_message: self.time_per_message,
            burst_size: self.burst_size,
            drop_instead_of_blocking: self.drop_instead_of_blocking,
        }
    }
}

impl Ratelimiter {
    /// The all-in-one handler for ratelimiting messages.
    /// Simply call this after receiving but before handling a message.
    /// If it returns `true`, handle the message,
    /// if it returns `false`, drop and ignore the message.
    ///
    /// Operates in dropping or blocking mode depending on
    /// how the ratelimit is configured:
    ///
    /// If `drop_instead_of_blocking` is disabled:
    /// Acts like `wait_if_necessary_on_recv()`, waiting for the
    /// ratelimit to no longer apply before returning `false`.
    ///
    /// If `drop_instead_of_blocking` is enabled:
    /// Checks `is_waiting_necessary()`. If waiting would be necessary, returns `true`.
    /// If waiting would not be necessary, calls `handled_message` and returns `false`.
    /// `should_drop_message()` never blocks when `drop_instead_of_blocking` is enabled.
    pub async fn should_drop_message(&mut self) -> bool {
        let now = Instant::now();
        if self.drop_instead_of_blocking {
            if self.is_waiting_necessary(now) {
                true
            } else {
                self.handled_message(now);
                false
            }
        } else {
            self.wait_if_necessary_on_recv(now).await;
            false
        }
    }

    // Should be called after receiving a message, but before handling it.
    // If too many messages are received at once, this may not return instantly,
    // effectively limiting how many messages the user can put through.
    pub async fn wait_if_necessary_on_recv(&mut self, now: Instant) {
        // NOTE: this works (tested, tho burst begins to charge only after first message, but this is probably good)
        self.last_message = Some(if let Some(last_message) = self.last_message {
            if now >= last_message + self.time_per_message {
                (last_message + self.time_per_message)
                    .max(now - self.time_per_message * (self.burst_size - 1))
            } else {
                tokio::time::sleep(last_message + self.time_per_message - now).await;
                last_message + self.time_per_message
            }
        } else {
            now
        });
    }

    // Returns `true` if calling `wait_if_necessary_on_recv` would not return immediately.
    // Can be used to drop messages which exceed the ratelimit instead of delaying them.
    pub fn is_waiting_necessary(&self, now: Instant) -> bool {
        // NOTE: must have the same logic as `wait_if_necessary_on_recv`
        if let Some(last_message) = self.last_message {
            if now >= last_message + self.time_per_message {
                false
            } else {
                true
            }
        } else {
            false
        }
    }

    /// This will never block, but it will always reset the ratelimit
    /// so that the next call to `wait_if_necessary_on_recv` will return
    /// after `time_per_message` has passed since `dont_wait_on_recv` was called.
    /// In other words, it resets the ratelimit and removes the user's
    /// built-up burst size, if there was any.
    pub fn dont_wait_on_recv(&mut self, now: Instant) {
        self.last_message = Some(now);
    }

    /// This will never block, but it will change the state in
    /// the same way as `wait_if_necessary_on_recv`.
    ///
    /// If you use `is_waiting_necessary` instead of `wait_if_necessary_on_recv`,
    /// call this method when you have handled a message
    /// (after `is_waiting_necessary` returns `false`).
    ///
    /// NOTE: Calling this when `is_waiting_necessary` returns `true`
    /// will cause the ratelimit to not allow messages for more than the configured `time_per_message` amount,
    /// as a message has been handled when the connection should have been ratelimited instead.
    /// This effect will stack, so calling this method very often will just make the ratelimit apply for an increasingly long time.
    pub fn handled_message(&mut self, now: Instant) {
        self.last_message = Some(if let Some(last_message) = self.last_message {
            if now >= last_message + self.time_per_message {
                (last_message + self.time_per_message)
                    .max(now - self.time_per_message * (self.burst_size - 1))
            } else {
                // this may go into the future, see method docs
                last_message + self.time_per_message
            }
        } else {
            now
        });
    }
}
