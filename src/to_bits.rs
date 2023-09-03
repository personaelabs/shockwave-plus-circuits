use frontend::ark_ff::{BigInteger, PrimeField};
use frontend::{ConstraintSystem, Wire};

fn felt_to_bits<F: PrimeField>(x: F) -> Vec<bool> {
    x.into_bigint().to_bits_le()
}

pub fn to_bits<F: PrimeField>(a: Wire<F>, cs: &mut ConstraintSystem<F>) -> Vec<Wire<F>> {
    let a_bits = felt_to_bits(a.val(cs).unwrap_or(F::ZERO))
        .iter()
        .map(|b| cs.alloc_var(F::from(*b as u64)))
        .collect::<Vec<Wire<F>>>();

    let mut sum = cs.alloc_const(F::ZERO);

    let mut pow = F::from(1u32);
    for a_i in &a_bits {
        let pow_alloc = cs.alloc_const(pow);
        let term = *a_i * pow_alloc;
        sum = sum + term;

        pow *= F::from(2u32);
    }

    sum.assert_equal(a, cs);

    a_bits
}

#[cfg(test)]
mod tests {
    use super::*;

    type Fp = frontend::ark_secp256k1::Fq;

    #[test]
    pub fn test_felt_to_bits() {
        let x = 12345u32;
        let x_felt = Fp::from(x);
        let bits = felt_to_bits(x_felt);

        for i in 0..(32 - x.leading_zeros()) {
            let expected = (x >> i) & 1 == 1;
            assert_eq!(bits[i as usize], expected);
        }
    }

    #[test]
    pub fn test_to_bits() {
        let synthesizer = |cs: &mut ConstraintSystem<Fp>| {
            let a = cs.alloc_priv_input();
            let bits = to_bits(a, cs);

            for bit in bits {
                cs.expose_public(bit);
            }
        };

        let x = Fp::from(12345);
        let expected_bits = felt_to_bits(x);

        let pub_input = expected_bits
            .iter()
            .map(|b| Fp::from(*b as u64))
            .collect::<Vec<Fp>>();
        let priv_input = vec![x];

        let mut cs = ConstraintSystem::new();
        let witness = cs.gen_witness(synthesizer, &pub_input, &priv_input);
        cs.is_sat(&witness, &pub_input, synthesizer);
    }
}
