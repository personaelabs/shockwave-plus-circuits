use frontend::FieldGC;
use frontend::Wire;
use shockwave_plus::ark_ff::BigInteger;

pub fn xor_64<F: FieldGC>(a: [Wire<F>; 64], b: [Wire<F>; 64]) -> [Wire<F>; 64] {
    let cs = a[0].cs();
    assert_eq!(a.len(), b.len());
    let mut out = [cs.one(); 64];
    for i in 0..64 {
        out[i] = bit_xor(a[i], b[i]);
    }

    out
}

// (!a) & b
pub fn not_a_and_b<F: FieldGC>(a: Wire<F>, b: Wire<F>) -> Wire<F> {
    let cs = a.cs();

    // (-a + 1) * b = c
    let deg_2_a = [(a, -F::ONE), (cs.one(), F::ONE)];
    let deg_2_b = [(b, F::ONE)];
    let deg_2_c = [];

    cs.deg_2_comb(&deg_2_a, &deg_2_b, &deg_2_c)
}

pub fn not_a_and_b_64<F: FieldGC>(a: [Wire<F>; 64], b: [Wire<F>; 64]) -> [Wire<F>; 64] {
    let cs = a[0].cs();
    assert_eq!(a.len(), b.len());
    let mut out = [cs.one(); 64];
    for i in 0..64 {
        out[i] = not_a_and_b(a[i], b[i]);
    }

    out
}

pub fn rotate_left_64<F: FieldGC>(a: [Wire<F>; 64], n: usize) -> [Wire<F>; 64] {
    let mut out = Vec::with_capacity(64);
    for i in 0..64 {
        out.push(a[((i as usize).wrapping_sub(n)) % 64]);
    }

    out.try_into().unwrap()
}

pub fn bit_xor<F: FieldGC>(a: Wire<F>, b: Wire<F>) -> Wire<F> {
    let cs = a.cs();

    // -2a * b + a + b = c
    let deg_2_a = [(a, -F::from(2u32))];
    let deg_2_b = [(b, F::ONE)];
    let deg_2_c = [(a, F::ONE), (b, F::ONE)];

    cs.deg_2_comb(&deg_2_a, &deg_2_b, &deg_2_c)
}

// Interprets the bits as LSB first.
pub fn from_bits<F: FieldGC>(bits: &[Wire<F>]) -> Wire<F> {
    let cs = bits[0].cs();

    let mut terms = Vec::with_capacity(64);

    let mut pow = F::from(1u32);
    for bit in bits {
        terms.push((cs.mul_const(*bit, pow), true));
        pow *= F::from(2u32);
    }

    cs.sum(&terms)
}

pub fn to_bits<F: FieldGC>(a: Wire<F>, field_bits: usize) -> Vec<Wire<F>> {
    let cs = a.cs();

    let bits = (0..field_bits)
        .map(|_| cs.alloc_var(F::ZERO))
        .collect::<Vec<Wire<F>>>();

    if cs.is_witness_gen() {
        let a_assigned = cs.wires[a.index];
        let a_bits_native = a_assigned.into_bigint().to_bits_be();
        for i in 0..field_bits {
            cs.wires[bits[i].index] = F::from(a_bits_native[i]);
        }
    }

    let mut sum = cs.alloc_const(F::ZERO);

    let mut pow = F::ONE;
    for a_i in bits.iter().rev() {
        let term = cs.mul_const(*a_i, pow);
        sum += term;

        pow *= F::from(2u32);
    }

    cs.assert_equal(a, sum, "to_bits failed");

    bits
}

#[cfg(test)]
mod tests {
    use super::*;
    use frontend::ark_ff::{Field, PrimeField};
    use frontend::ConstraintSystem;

    type Fp = frontend::ark_secp256k1::Fq;

    #[test]
    pub fn test_from_bits() {
        let synthesizer = |cs: &mut ConstraintSystem<Fp>| {
            let bits = cs.alloc_priv_inputs(256);
            let out = from_bits(&bits);

            cs.expose_public(out);
        };

        let mut bits = vec![Fp::ZERO; 256];
        bits[0] = Fp::ONE;
        let expected = Fp::ONE;

        let priv_input = bits;
        let pub_input = vec![expected];

        let mut cs = ConstraintSystem::new();
        let witness = cs.gen_witness(synthesizer, &pub_input, &priv_input);

        cs.set_constraints(&synthesizer);
        assert!(cs.is_sat(&witness, &pub_input));
    }

    #[test]
    fn test_to_bits() {
        let synthesizer = |cs: &mut ConstraintSystem<Fp>| {
            let val = cs.alloc_priv_input();
            let out = to_bits(val, 256);

            for out_i in out {
                cs.expose_public(out_i);
            }
        };

        let val = Fp::from(123);
        let expected_bits = val
            .into_bigint()
            .to_bits_be()
            .iter()
            .map(|b| Fp::from(*b))
            .collect::<Vec<Fp>>();

        let priv_input = [val];
        let pub_input = expected_bits;

        let mut cs = ConstraintSystem::new();
        let witness = cs.gen_witness(synthesizer, &pub_input, &priv_input);

        cs.set_constraints(&synthesizer);
        assert!(cs.is_sat(&witness, &pub_input));
    }
}
