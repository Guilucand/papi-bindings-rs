//! This package provides bindings to the PAPI performance counters
//! library.

#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(dead_code)]
#[allow(non_upper_case_globals)]
#[allow(deref_nullptr)]
mod bindings;
pub mod counter;
pub mod events_set;

use std::ffi::CStr;
use std::fmt::Debug;
use std::os::raw::{c_int, c_ulong};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::bindings::*;

fn papi_version_number(maj: u32, min: u32, rev: u32, inc: u32) -> u32 {
    (maj << 24) | (min << 16) | (rev << 8) | inc
}

#[link(name = "papi")]
extern "C" {}

#[allow(dead_code)]
pub struct PapiError {
    code: i32,
}

impl Debug for PapiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        print!("Hi");
        let err_msg_buf = unsafe { PAPI_strerror(self.code) };
        print!("Hi ok");
        let err_msg = unsafe { CStr::from_ptr(err_msg_buf) };
        print!("Hi ok2");
        write!(
            f,
            "PapiError with error code {}, i.e \"{}\"",
            self.code,
            err_msg.to_str().unwrap_or_else(|_| "NULL")
        )
    }
}
pub(crate) fn check_error(code: i32) -> Result<(), PapiError> {
    if code == (PAPI_OK as i32) {
        Ok(())
    } else {
        Err(PapiError { code })
    }
}

static THREAD_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

thread_local! {
    static THREAD_INDEX: u64 = THREAD_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
}

extern "C" fn get_thread_id() -> c_ulong {
    THREAD_INDEX.with(|id| *id as c_ulong)
}

pub fn initialize(multithread: bool) -> Result<(), PapiError> {
    unsafe {
        let cur_version = Command::new("papi_version")
            .output()
            .expect("papi_version not found")
            .stdout;
        let cur_version = String::from_utf8(cur_version).unwrap();
        let mut digits = cur_version
            .trim()
            .split(' ')
            .last()
            .unwrap()
            .split('.')
            .map(|d| d.parse::<u32>().unwrap());
        let maj = digits.next().unwrap();
        let min = digits.next().unwrap();
        let rev = digits.next().unwrap();
        let inc = digits.next().unwrap();
        let cur_version = (papi_version_number(maj, min, rev, inc) & 0xffff0000) as c_int;
        let version = PAPI_library_init(cur_version);
        if version != cur_version {
            return Err(PapiError { code: version });
        }

        if multithread {
            check_error(PAPI_thread_init(Some(get_thread_id)))?;
        }
    }

    Ok(())
}

pub fn is_initialized() -> bool {
    unsafe { check_error(PAPI_is_initialized()).is_ok() }
}

// The only reasonable action for counters_in_use is to
// retry. Otherwise, you might as well just fail yourself.
#[derive(PartialEq, Eq)]
pub enum Action {
    Retry,
}

#[cfg(test)]
mod tests {
    use crate::counter::Counter;
    use crate::events_set::EventsSet;
    use crate::initialize;
    use crate::PapiError;

    #[test]
    fn test_papi_error() {
        // https://bitbucket.org/icl/papi/wiki/PAPI-Error-Handling.md
        // source for expected error messages
        initialize(true).unwrap();
        let error = PapiError { code: -7 };
        let msg = format!("{error:?}");
        assert_eq!(
            msg,
            "PapiError with error code -7, i.e \"Event does not exist\""
        );
    }

    #[test]
    fn test_fib() {
        initialize(true).unwrap();

        let counters = vec![
            Counter::from_name("ix86arch::INSTRUCTION_RETIRED").unwrap(),
            Counter::from_name("ix86arch::MISPREDICTED_BRANCH_RETIRED").unwrap(),
        ];

        let mut event_set = EventsSet::new(&counters).unwrap();

        for fv in 1..15 {
            event_set.start().unwrap();
            let x = fib(fv);
            let counters = event_set.stop().unwrap();
            println!(
                "Computed fib({}) = {} in {} instructions [mispredicted: {}].",
                fv, x, counters[0], counters[1]
            );
        }
    }

    fn fib(n: isize) -> isize {
        if n < 2 {
            1
        } else {
            fib(n - 1) + fib(n - 2)
        }
    }
}
