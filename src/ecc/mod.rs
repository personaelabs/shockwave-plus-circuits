use frontend::FieldGC;
use frontend::Wire;

pub mod add;
pub mod double;
pub mod mul;

#[derive(Copy, Clone)]
pub struct AffinePoint<F: FieldGC> {
    pub x: Wire<F>,
    pub y: Wire<F>,
}

impl<F: FieldGC> AffinePoint<F> {
    pub fn new(x: Wire<F>, y: Wire<F>) -> Self {
        Self { x, y }
    }
}
