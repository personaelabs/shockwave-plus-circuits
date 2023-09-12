use crate::ec_double;

use super::{add::ec_add_complete, AffinePoint};
use frontend::FieldGC;
use frontend::{ConstraintSystem, Wire};

// Naive double-and-add algorithm
pub fn ec_mul<F: FieldGC>(
    p: AffinePoint<F>,
    s_bits: &[Wire<F>],
    cs: &mut ConstraintSystem<F>,
) -> AffinePoint<F> {
    let infinity = AffinePoint::new(cs.alloc_const(F::ZERO), cs.alloc_const(F::ZERO));
    let mut result = infinity;
    let mut current = p;

    for s_i in s_bits {
        let t_x = *s_i * current.x;
        let t_y = *s_i * current.y;
        let t = AffinePoint::new(t_x, t_y);

        result = ec_add_complete(t, result, cs);
        current = ec_double(current, cs);
    }

    result.x.println();

    result
}

#[cfg(test)]
mod tests {
    use frontend::ark_ff::BigInteger;
    use frontend::ark_ff::PrimeField;
    use frontend::ark_secp256k1::Fr;

    use ark_ec::{AffineRepr, CurveGroup};
    use frontend::ark_secp256k1::Affine as Secp256k1Affine;
    type Fp = frontend::ark_secp256k1::Fq;

    use super::*;

    #[test]
    pub fn test_mul() {
        let synthesizer = |cs: &mut ConstraintSystem<Fp>| {
            let p_x = cs.alloc_priv_input();
            let p_y = cs.alloc_priv_input();

            let s_bits = cs.alloc_priv_inputs(256);

            let p = AffinePoint::<Fp>::new(p_x, p_y);

            let out = ec_mul(p, &s_bits, cs);

            cs.expose_public(out.x);
            cs.expose_public(out.y);
        };

        let p = Secp256k1Affine::generator();
        let s = Fr::from(3u32);

        let s_bits = s
            .into_bigint()
            .to_bits_le()
            .iter()
            .map(|b| Fp::from(*b))
            .collect::<Vec<Fp>>();

        let out = (p * s).into_affine();

        let pub_input = vec![out.x, out.y];
        let mut priv_input = vec![p.x, p.y];
        priv_input.extend_from_slice(&s_bits);

        let mut cs = ConstraintSystem::new();
        let witness = cs.gen_witness(synthesizer, &pub_input, &priv_input);

        cs.set_constraints(&synthesizer);

        println!("Num constraints: {}", cs.num_constraints.unwrap());
        println!("Num vars: {}", cs.num_vars());

        assert!(cs.is_sat(&witness, &pub_input));
    }
}
