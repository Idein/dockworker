#[cfg(unix)]
mod unix {
    use std::convert::TryFrom;
    use std::io;
    use std::os::raw::c_int;

    pub use self::NixSignal::*;
    use nix::sys::signal::{Signal as NixSignal, SignalIterator as NixSignalIterator};

    use crate::errors::Error;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Signal(NixSignal);

    #[derive(Debug, Clone)]
    pub struct SignalIterator(NixSignalIterator);

    impl Signal {
        pub fn as_i32(&self) -> i32 {
            self.0 as i32
        }

        pub fn iterator() -> SignalIterator {
            SignalIterator(NixSignal::iterator())
        }

        pub fn from_c_int(signum: c_int) -> Result<Self, Error> {
            Ok(NixSignal::try_from(signum)
                .map_err(|err| io::Error::from_raw_os_error(err as i32))?
                .into())
        }
    }

    impl From<NixSignal> for Signal {
        fn from(sig: NixSignal) -> Self {
            Self(sig)
        }
    }

    impl Iterator for SignalIterator {
        type Item = Signal;
        fn next(&mut self) -> Option<Self::Item> {
            self.0.next().map(Into::into)
        }
    }
}

#[cfg(windows)]
mod windows {
    use std::io;
    use std::os::raw::c_int;

    use crate::errors::Error;

    #[repr(i32)]
    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
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

        pub fn from_c_int(signum: c_int) -> Result<Self, Error> {
            match signum {
                9 => Ok(Signal::SIGKILL),
                15 => Ok(Signal::SIGTERM),
                other => Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unknown signal: {}", other),
                )
                .into()),
            }
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
