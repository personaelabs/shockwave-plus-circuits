use frontend::{ConstraintSystem, FieldExt, Wire};

fn felt_to_bits<F: FieldExt>(x: F) -> Vec<bool> {
    x.to_repr()
        .iter()
        .map(|byte| (0..8).map(move |i| (byte >> i) & 1u8 == 1u8))
        .flatten()
        .collect::<Vec<bool>>()
}

pub fn to_bits<F: FieldExt>(a: Wire<F>, cs: &mut ConstraintSystem<F>) -> Vec<Wire<F>> {
    let a_bits = felt_to_bits(a.val(cs).unwrap_or(F::ZERO))
        .iter()
        .map(|b| cs.alloc_var(F::from(*b as u64)))
        .collect::<Vec<Wire<F>>>();

    let mut sum = cs.alloc_const(F::ZERO);

    let mut pow = F::from(1);
    for a_i in &a_bits {
        let term = a_i.mul_const(pow, cs);
        sum = sum.add(term, cs);

        pow *= F::from(2);
    }

    sum.assert_equal(a, cs);

    a_bits
}

#[cfg(test)]
mod tests {
    use super::*;
    use frontend::test_circuit;
    use frontend::wasm_deps::*;

    type F = frontend::halo2curves::secp256k1::Fp;

    #[test]
    pub fn test_felt_to_bits() {
        let x = 12345;
        let x_felt = F::from(x);
        let bits = felt_to_bits(x_felt);

        for i in 0..64 {
            let expected = (x >> i) & 1 == 1;
            assert_eq!(bits[i], expected);
        }
    }

    #[test]
    pub fn test_to_bits() {
        test_circuit!(
            |cs: &mut ConstraintSystem<F>| {
                let a = cs.alloc_priv_input();
                let bits = to_bits(a, cs);

                for bit in bits {
                    cs.expose_public(bit);
                }
            },
            F
        );

        let x = F::from(12345);
        let expected_bits = felt_to_bits(x);

        let pub_input = expected_bits
            .iter()
            .map(|b| F::from(*b as u64))
            .collect::<Vec<F>>();
        let priv_input = vec![x];

        mock_run(&pub_input, &priv_input);
    }
}
