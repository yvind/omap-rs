/// The allowed error in bezier conversion for line objects and area objects
/// None represents not converting to bezier when writing
#[derive(Debug, Clone, Copy, Default)]
pub struct BezierError {
    /// Allowed error when converting line objects to bezier curves
    /// None represents not converting to bezier
    line_error: Option<f64>,
    /// Allowed error when converting area objects to bezier curves
    /// None represents not converting to bezier
    area_error: Option<f64>,
}

impl BezierError {
    /// Create a new BezierError object
    /// If an error is less than 0.1 it is set to None
    pub fn new(line_error: Option<f64>, area_error: Option<f64>) -> BezierError {
        BezierError {
            line_error: line_error.filter(|&le| le >= 0.1),
            area_error: area_error.filter(|&ae| ae >= 0.1),
        }
    }

    /// Get the line error
    pub fn get_line_error(&self) -> Option<f64> {
        self.line_error
    }

    /// Get the area error
    pub fn get_area_error(&self) -> Option<f64> {
        self.area_error
    }

    /// Consume the object and get the line error and area error as a tuple
    pub fn into_inner(self) -> (Option<f64>, Option<f64>) {
        (self.line_error, self.area_error)
    }
}
