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
    use frontend::ark_secp256k1::Fr;
    use frontend::wasm_deps::*;

    use ark_ec::{AffineRepr, CurveGroup};
    use frontend::ark_secp256k1::Affine as Secp256k1Affine;
    type F = frontend::ark_secp256k1::Fq;

    use super::*;

    #[test]
    fn test_ec_double() {
        let synthesizer = |cs: &mut ConstraintSystem<F>| {
            let p_x = cs.alloc_priv_input();
            let p_y = cs.alloc_priv_input();

            let p = AffinePoint::<F>::new(p_x, p_y);

            let out = ec_double(p, cs);

            cs.expose_public(out.x);
            cs.expose_public(out.y);
        };

        let p = (Secp256k1Affine::generator() * Fr::from(3)).into_affine();
        let p_double = (p + p).into_affine();

        let pub_input = [p_double.x, p_double.y];
        let priv_input = [p.x, p.y];

        let mut cs = ConstraintSystem::new();
        let witness = cs.gen_witness(synthesizer, &pub_input, &priv_input);

        assert!(cs.is_sat(&witness, &pub_input, synthesizer));
    }
}
