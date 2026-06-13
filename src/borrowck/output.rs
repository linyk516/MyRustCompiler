use crate::borrowck::error::BorrowError;

#[derive(Debug, Clone)]
pub struct BorrowckOutput {
    pub errors: Vec<BorrowError>,
}
