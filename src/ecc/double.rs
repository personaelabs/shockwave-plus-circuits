use super::AffinePoint;
use frontend::{ConstraintSystem, FieldExt};

// Doubling for short-Weierstrass curves
pub fn ec_double<F: FieldExt>(p: AffinePoint<F>, cs: &mut ConstraintSystem<F>) -> AffinePoint<F> {
    let p_x = p.x;
    let p_y = p.y;

    // lambda = (3 * x^2) / (2 * y)
    let lambda = p_x
        .square(cs)
        .mul_const(F::from(3), cs)
        .div(p_y.mul_const(F::from(2), cs), cs);

    // x = lambda^2 - 2 * x
    let out_x = lambda.square(cs).sub(p_x.mul_const(F::from(2), cs), cs);
    // y = lambda * (x - out_x) - y
    let out_y = lambda.mul(p_x.sub(out_x, cs), cs).sub(p_y, cs);

    AffinePoint::new(out_x, out_y)
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_ec_double() {}
}
