use frontend::FieldGC;
use frontend::Wire;

pub fn xor_64<F: FieldGC>(a: [Wire<F>; 64], b: [Wire<F>; 64]) -> [Wire<F>; 64] {
    let cs = a[0].cs();
    assert_eq!(a.len(), b.len());
    let mut out = [cs.one(); 64];
    for i in 0..64 {
        out[i] = bit_xor(a[i], b[i]);
    }

    out
}

pub fn not_64<F: FieldGC>(a: [Wire<F>; 64]) -> [Wire<F>; 64] {
    let cs = a[0].cs();
    let mut out = [cs.one(); 64];
    for i in 0..64 {
        out[i] = a[i].not(cs);
    }

    out
}

pub fn and_64<F: FieldGC>(a: [Wire<F>; 64], b: [Wire<F>; 64]) -> [Wire<F>; 64] {
    let cs = a[0].cs();
    assert_eq!(a.len(), b.len());
    let mut out = [cs.one(); 64];
    for i in 0..64 {
        out[i] = a[i].and(b[i], cs);
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
    a + b - cs.mul_const(a * b, F::from(2u32))
}

// Interprets the bits as LSB first.
pub fn from_bits<F: FieldGC>(bits: &[Wire<F>]) -> Wire<F> {
    let cs = bits[0].cs();
    let mut sum = cs.alloc_const(F::ZERO);

    let mut pow = F::from(1u32);
    for bit in bits {
        sum += *bit * cs.alloc_const(pow);
        pow *= F::from(2u32);
    }

    sum
}

#[cfg(test)]
mod tests {
    use super::*;
    use frontend::ark_ff::Field;
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
        assert!(cs.is_sat(&witness, &pub_input, synthesizer));
    }
}
