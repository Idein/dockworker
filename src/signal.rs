#[cfg(unix)]
mod unix {
    use nix::sys::signal::{Signal as NixSignal, SignalIterator as NixSignalIterator};

    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
    pub struct Signal(NixSignal);

    #[derive(Debug, Clone)]
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

    #[derive(Debug, Clone)]
    pub struct SignalIterator(std::vec::IntoIter<Signal>);

    impl Signal {
        pub fn as_i32(&self) -> i32 {
            self.clone() as i32
        }

        pub fn iterator() -> SignalIterator {
            use self::Signal::*;
            SignalIterator(vec![SIGKILL, SIGTERM].into_iter())
        }
    }

    impl Iterator for SignalIterator {
        type Item = Signal;
        fn next(&mut self) -> Option<Self::Item> {
            self.0.next()
        }
    }

    #[cfg(test)]
    mod test {
        use super::*;
        #[test]
        fn iterator() {
            let mut it = Signal::iterator();
            assert_eq!(it.next(), Some(Signal::SIGKILL));
            assert_eq!(it.next(), Some(Signal::SIGTERM));
            assert_eq!(it.next(), None);
        }
    }
}

#[cfg(unix)]
pub use self::unix::*;
#[cfg(windows)]
pub use self::windows::*;
