use super::AffinePoint;
use frontend::ark_ff::PrimeField;
use frontend::ConstraintSystem;

// Doubling for short-Weierstrass curves
pub fn ec_double<F: PrimeField>(p: AffinePoint<F>, cs: &mut ConstraintSystem<F>) -> AffinePoint<F> {
    // lambda = (3 * x^2) / (2 * y)
    let lambda =
        (cs.alloc_const(F::from(3u32)) * (p.x * p.x)) / (cs.alloc_const(F::from(2u32)) * p.y);

    // x = lambda^2 - 2 * x
    let out_x = (lambda * lambda) - (p.x * cs.alloc_const(F::from(2u32)));
    // y = lambda * (x - out_x) - y
    let out_y = lambda * (p.x - out_x) - p.y;

    AffinePoint::new(out_x, out_y)
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_ec_double() {}
}
