use std::time::Duration;

struct Timeout<S> {
    inner: S,
    timeout: Duration
}

