use frontend::{FieldExt, Wire};

pub mod add;
pub mod double;

#[derive(Copy, Clone)]
pub struct AffinePoint<F: FieldExt> {
    x: Wire<F>,
    y: Wire<F>,
}

impl<F: FieldExt> AffinePoint<F> {
    pub fn new(x: Wire<F>, y: Wire<F>) -> Self {
        Self { x, y }
    }
}
