use super::AffinePoint;
use frontend::{ConstraintSystem, FieldExt};

// Incomplete addition for short-Weierstrass curves.
// We follow the specification from the halo2 book;
// https://zcash.github.io/halo2/design/gadgets/sinsemilla.html?highlight=incomplete#incomplete-addition
pub fn ec_add_incomplete<F: FieldExt>(
    p: AffinePoint<F>,
    q: AffinePoint<F>,
    cs: &mut ConstraintSystem<F>,
) -> AffinePoint<F> {
    let p_x = p.x;
    let p_y = p.y;

    let q_x = q.x;
    let q_y = q.y;

    let dx = p_x.sub(q_x, cs);
    let dy = p_y.sub(q_y, cs);

    let lambda = dy.div_or_zero(dx, cs);

    let out_x = lambda.square(cs).sub(p_x, cs).sub(q_x, cs);
    let out_y = lambda.mul(p_x.sub(out_x, cs), cs).sub(p_y, cs);

    AffinePoint::new(out_x, out_y)
}

// Complete addition for short-Weierstrass curves.
// We follow the specification from the halo2 book.
// https://zcash.github.io/halo2/design/gadgets/ecc/addition.html#complete-addition
pub fn ec_add_complete<F: FieldExt>(
    p: AffinePoint<F>,
    q: AffinePoint<F>,
    cs: &mut ConstraintSystem<F>,
) -> AffinePoint<F> {
    let p_x = p.x;
    let p_y = p.y;

    let q_x = q.x;
    let q_y = q.y;

    let is_x_equal = p_x.is_equal(q_x, cs);

    let p_is_zero = p_x.is_zero(cs);
    let q_is_zero = q_x.is_zero(cs);

    let both_zeros = p_is_zero.and(q_is_zero, cs);
    let is_sym = is_x_equal.and(p_y.is_equal(q_y.neg(cs), cs), cs);
    let is_out_zero = both_zeros.or(is_sym, cs);

    let zero = cs.alloc_const(F::ZERO);

    let inc_add = ec_add_incomplete(p, q, cs);

    let out_x = cs
        .if_then(is_out_zero, zero)
        .elif(p_is_zero, q_x, cs)
        .elif(q_is_zero, p_x, cs)
        .else_then(inc_add.x, cs);

    let out_y = cs
        .if_then(is_out_zero, zero)
        .elif(p_is_zero, q_y, cs)
        .elif(q_is_zero, p_y, cs)
        .else_then(inc_add.y, cs);

    AffinePoint::new(out_x, out_y)
}

#[cfg(test)]
mod tests {
    use frontend::halo2curves::ff::Field;
    use frontend::halo2curves::group::Curve;
    use frontend::halo2curves::secp256k1::Fq;
    use frontend::halo2curves::secp256k1::Secp256k1Affine;
    use frontend::test_circuit;
    use frontend::wasm_deps::*;

    type F = frontend::halo2curves::secp256k1::Fp;

    use super::*;

    #[test]
    pub fn test_add_incomplete() {
        test_circuit!(
            |cs: &mut ConstraintSystem<F>| {
                let p_x = cs.alloc_priv_input();
                let p_y = cs.alloc_priv_input();

                let q_x = cs.alloc_priv_input();
                let q_y = cs.alloc_priv_input();

                let p = AffinePoint::<F>::new(p_x, p_y);
                let q = AffinePoint::<F>::new(q_x, q_y);

                let out = ec_add_incomplete(p, q, cs);

                cs.expose_public(out.x);
                cs.expose_public(out.y);
            },
            F
        );

        let p = Secp256k1Affine::generator();
        let q = (Secp256k1Affine::generator() * Fq::from(3)).to_affine();

        let out = (p + q).to_affine();

        let pub_input = vec![out.x, out.y];
        let priv_input = vec![p.x, p.y, q.x, q.y];

        mock_run(&pub_input, &priv_input)
    }

    #[test]
    pub fn test_add_complete() {
        test_circuit!(
            |cs: &mut ConstraintSystem<F>| {
                let p_x = cs.alloc_priv_input();
                let p_y = cs.alloc_priv_input();

                let q_x = cs.alloc_priv_input();
                let q_y = cs.alloc_priv_input();

                let p = AffinePoint::<F>::new(p_x, p_y);
                let q = AffinePoint::<F>::new(q_x, q_y);

                let out = ec_add_complete(p, q, cs);

                cs.expose_public(out.x);
                cs.expose_public(out.y);
            },
            F
        );

        let zero = (Secp256k1Affine::generator() * Fq::ZERO).to_affine();

        let p_nonzero = Secp256k1Affine::generator();
        let q_nonzero = (Secp256k1Affine::generator() * Fq::from(3)).to_affine();

        let cases = [
            (zero, zero),
            (zero, p_nonzero),
            (p_nonzero, zero),
            (p_nonzero, -q_nonzero),
            (p_nonzero, q_nonzero),
        ];

        for (p, q) in cases {
            let out = (p + q).to_affine();
            let pub_input = vec![out.x, out.y];
            let priv_input = vec![p.x, p.y, q.x, q.y];

            mock_run(&pub_input, &priv_input);
        }
    }
}
