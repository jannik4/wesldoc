use crate::Error;
use std::cell::RefCell;

#[derive(Debug, Default)]
pub struct ErrorSink {
    error: RefCell<Option<Error>>,
}

impl ErrorSink {
    // TODO: currently uses first error. maybe add priority to errors? or collect all?
    pub fn report(&self, error: Error) {
        let mut current = self.error.borrow_mut();
        if current.is_none() {
            *current = Some(error);
        }
    }

    pub fn into_result(self) -> Result<(), Error> {
        match self.error.into_inner() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}
