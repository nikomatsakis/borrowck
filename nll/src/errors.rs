use env::Point;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;

pub struct ErrorReporting {
    reported_errors: Vec<ReportedError>,
    expected_errors: HashMap<Point, String>,
}

#[derive(Debug)]
pub struct ReportedError {
    point: Point,
    message: String,
}

impl ErrorReporting {
    pub fn new() -> Self {
        ErrorReporting {
            expected_errors: HashMap::new(),
            reported_errors: vec![],
        }
    }

    pub fn report_error(&mut self, point: Point, message: String) {
        self.reported_errors.push(ReportedError { point, message });
    }

    pub fn expect_error(&mut self, point: Point, message: &str) {
        let old_entry = self.expected_errors.insert(point, message.to_string());
        assert!(old_entry.is_none());
    }

    pub fn reconcile_errors(&mut self) -> Result<(), Box<Error>> {
        while let Some(reported_error) = self.reported_errors.pop() {
            if let Some(expected_message) = self.expected_errors.remove(&reported_error.point) {
                if reported_error.message.contains(&expected_message) {
                    continue;
                }
            }
            return Err(Box::new(reported_error));
        }

        for &expected_point in self.expected_errors.keys() {
            return Err(Box::new(ReportedError {
                point: expected_point,
                message: format!("no error reported on this point, but we expected one")
            }));
        }

        Ok(())
    }
}

impl Error for ReportedError {
    fn description(&self) -> &str {
        &self.message
    }

    fn cause(&self) -> Option<&Error> {
        None
    }
}

impl fmt::Display for ReportedError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{:?}: {}", self.point, self.message)
    }
}
