use frontend::ark_ff::PrimeField;
use frontend::Wire;

pub mod add;
pub mod double;

#[derive(Copy, Clone)]
pub struct AffinePoint<F: PrimeField> {
    x: Wire<F>,
    y: Wire<F>,
}

impl<F: PrimeField> AffinePoint<F> {
    pub fn new(x: Wire<F>, y: Wire<F>) -> Self {
        Self { x, y }
    }
}
