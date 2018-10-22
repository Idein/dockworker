#[cfg(unix)]
mod unix {
    use nix::sys::signal::{Signal as NixSignal, SignalIterator as NixSignalIterator};

    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
    pub struct Signal(NixSignal);

    pub struct SignalIterator(NixSignalIterator);

    impl Signal {
        fn new(signal: NixSignal) -> Self {
            Signal(signal)
        }

        pub fn as_i32(&self) -> i32 {
            self.0 as i32
        }

        pub fn iterator() -> SignalIterator {
            SignalIterator::new(NixSignal::iterator())
        }
    }

    impl From<NixSignal> for Signal {
        fn from(signal: NixSignal) -> Self {
            Self::new(signal)
        }
    }

    impl SignalIterator {
        fn new(iter: NixSignalIterator) -> Self {
            Self { 0: iter }
        }
    }

    impl Iterator for SignalIterator {
        type Item = Signal;
        fn next(&mut self) -> Option<Self::Item> {
            self.0.next().map(Signal::new)
        }
    }
}

#[cfg(windows)]
mod windows {
    #[repr(i32)]
    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
    pub enum Signal {
        SIGKILL = 9,
        SIGTERM = 15,
    }

    pub struct SignalIterator(Vec<Signal>);

    impl Signal {
        pub fn as_i32(&self) -> i32 {
            self.clone() as i32
        }

        pub fn iterator() -> SignalIterator {
            use self::Signal::*;
            SignalIterator::new(vec![SIGKILL, SIGTERM])
        }
    }

    impl Iterator for SignalIterator {
        type Item = Signal;
        fn next(&mut self) -> Option<Self::Item> {
            self.0.pop()
        }
    }
}

#[cfg(unix)]
pub use self::unix::*;
#[cfg(windows)]
pub use self::windows::*;
