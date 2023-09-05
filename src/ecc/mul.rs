use super::{
    add::{ec_add_complete, ec_add_incomplete},
    double::ec_double,
    AffinePoint,
};
use frontend::FieldGC;
use frontend::{ConstraintSystem, Wire};

//
// Variable-base scalar multiplication addition for secp256k1.
// We follow the specification from the halo2 book;
// https://zcash.github.io/halo2/design/gadgets/ecc/var-base-scalar-mul.html
pub fn ec_mul<F: FieldGC>(
    t: AffinePoint<F>,
    k_bits: &[Wire<F>],
    cs: &mut ConstraintSystem<F>,
) -> AffinePoint<F> {
    let mut acc = ec_double(t, cs);

    let minus_t_y = -t.y;

    for i in (0..255).rev() {
        let k_i = k_bits[i + 1];
        let px = t.x;
        let py = cs.if_then(k_i, t.y).else_then(minus_t_y);

        let p = AffinePoint::new(px, py);

        if i < 4 {
            acc = ec_add_complete(ec_add_complete(p, acc, cs), acc, cs);
        } else {
            acc = ec_add_incomplete(ec_add_incomplete(p, acc), acc);
        }
    }

    let zero = cs.alloc_const(F::ZERO);

    let final_lhs_x = cs.if_then(k_bits[0], t.x).else_then(zero);
    let final_lhs_y = cs.if_then(k_bits[0], minus_t_y).else_then(zero);

    acc.x.print_val();
    let final_lhs = AffinePoint::new(final_lhs_x, final_lhs_y);

    acc = ec_add_complete(acc, final_lhs, cs);
    acc.x.print_val();

    acc
}

#[cfg(test)]
mod tests {
    use frontend::ark_ff::BigInteger;
    use frontend::ark_ff::PrimeField;
    use frontend::ark_secp256k1::Fr;
    use std::str::FromStr;

    use ark_ec::{AffineRepr, CurveGroup};
    use frontend::ark_secp256k1::Affine as Secp256k1Affine;
    type Fp = frontend::ark_secp256k1::Fq;

    use super::*;

    #[test]
    pub fn test_mul() {
        let synthesizer = |cs: &mut ConstraintSystem<Fp>| {
            let p_x = cs.alloc_priv_input();
            let p_y = cs.alloc_priv_input();

            let k_bits = cs.alloc_priv_inputs(256);

            let p = AffinePoint::<Fp>::new(p_x, p_y);

            let out = ec_mul(p, &k_bits, cs);
            out.x.print_val();

            cs.expose_public(out.x);
            cs.expose_public(out.y);
        };

        let p = Secp256k1Affine::generator();
        let s = Fr::from(3u32);

        // (q - 2^256) % q;
        let t_q = Fr::from_str(
            "115792089237316195423570985008687907852405143892509244725752742275123193348738",
        )
        .unwrap();

        let k = s + t_q;
        let k_bits = k
            .into_bigint()
            .to_bits_be()
            .iter()
            .map(|b| Fp::from(*b))
            .collect::<Vec<Fp>>();

        let out = (p * s).into_affine();

        let pub_input = vec![out.x, out.y];
        let mut priv_input = vec![p.x, p.y];
        priv_input.extend_from_slice(&k_bits);

        let mut cs = ConstraintSystem::new();
        let witness = cs.gen_witness(synthesizer, &pub_input, &priv_input);
        assert!(cs.is_sat(&witness, &pub_input, synthesizer));
    }
}
