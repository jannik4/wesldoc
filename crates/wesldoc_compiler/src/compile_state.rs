use crate::Error;
use std::cell::RefCell;

#[derive(Debug, Default)]
pub struct CompileState {
    stats: RefCell<CompileStats>,
    error: RefCell<Option<Error>>,
}

impl CompileState {
    pub fn track_documented(&self, is_documented: bool) {
        let mut stats = self.stats.borrow_mut();
        stats.documented.0 += is_documented as usize;
        stats.documented.1 += 1;
    }

    // TODO: currently uses first error. maybe add priority to errors? or collect all?
    pub fn report_error(&self, error: Error) {
        let mut current = self.error.borrow_mut();
        if current.is_none() {
            *current = Some(error);
        }
    }

    pub fn into_result(self) -> Result<CompileStats, Error> {
        match self.error.into_inner() {
            Some(error) => Err(error),
            None => Ok(self.stats.into_inner()),
        }
    }
}

#[derive(Debug, Default)]
pub struct CompileStats {
    documented: (usize, usize), // (documented, total)
}

impl CompileStats {
    pub fn documented_percentage(&self) -> f64 {
        let (documented, total) = self.documented;
        if total == 0 {
            return 100.0;
        }
        documented as f64 / total as f64 * 100.0
    }
}
