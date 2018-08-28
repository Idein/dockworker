#[cfg(unix)]
mod unix {
    use nix::sys::signal::Signal as NixSignal;

    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
    pub struct Signal(NixSignal);

    impl Signal {
        fn new(signal: NixSignal) -> Self {
            Signal(signal)
        }

        pub fn as_i32(&self) -> i32 {
            self.0 as i32
        }
    }

    impl From<NixSignal> for Signal {
        fn from(signal: NixSignal) -> Self {
            Self::new(signal)
        }
    }
}

#[cfg(windows)]
mod windows {
    #[repr(i32)]
    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
    pub enum Signal {
        SIGKILL = 9,
        SIGTERM = 15,
    }

    impl Signal {
        pub fn as_i32(&self) -> i32 {
            self.clone() as i32
        }
    }
}

#[cfg(unix)]
pub use self::unix::Signal;
#[cfg(windows)]
pub use self::windows::Signal;
