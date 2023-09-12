use std::result::Result;

use frontend::{ConstraintSystem, Wire};
use shockwave_plus::{FieldGC, IOPattern, PoseidonConstants, PoseidonCurve, PoseidonSponge};

use crate::PoseidonChip;

// Implements SAFE (Sponge API for Field Elements): https://hackmd.io/bHgsH6mMStCVibM_wYvb2w
#[derive(Clone)]
pub struct PoseidonSpongeChip<F: FieldGC, const WIDTH: usize> {
    pub absorb_pos: usize,
    pub squeeze_pos: usize,
    pub io_count: usize,
    pub io_pattern: IOPattern,
    pub rate: usize,
    pub capacity: usize,
    pub poseidon: PoseidonChip<F, WIDTH>,
}

impl<F: FieldGC, const WIDTH: usize> PoseidonSpongeChip<F, WIDTH> {
    pub fn new(
        domain_separator: &[u8],
        io_pattern: IOPattern,
        curve: PoseidonCurve,
        cs_ptr: *mut ConstraintSystem<F>,
    ) -> Self {
        let cs = unsafe { &mut *cs_ptr };

        let constants = PoseidonConstants::<F>::new(curve, WIDTH);

        let tag = cs.alloc_const(PoseidonSponge::<F, WIDTH>::compute_tag(
            domain_separator,
            &io_pattern,
        ));

        let zero = cs.alloc_const(F::ZERO);
        let mut state = [zero; WIDTH];
        state[0] = tag;

        let poseidon = PoseidonChip::new(cs, constants);
        let mut poseidon = poseidon;
        poseidon.state = state;

        Self {
            absorb_pos: 0,
            squeeze_pos: 0,
            io_count: 0,
            io_pattern,
            rate: 2,
            capacity: 1,
            poseidon,
        }
    }

    pub fn absorb(&mut self, x: &[Wire<F>]) {
        if x.len() == 0 {
            return;
        }

        for x_i in x {
            if self.absorb_pos == self.rate {
                self.permute();
                self.absorb_pos = 0
            }

            self.poseidon.state[self.absorb_pos] = *x_i;
            self.absorb_pos += 1;
        }

        // assert_eq!(self.io_pattern.0[self.io_count], SpongeOp::Absorb(x.len()));

        self.io_count += 1;
        self.squeeze_pos = self.rate;
    }

    pub fn squeeze(&mut self, length: usize) -> Vec<Wire<F>> {
        let mut y = Vec::with_capacity(length);
        if length == 0 {
            return vec![];
        }

        for _ in 0..length {
            if self.squeeze_pos == self.rate {
                self.permute();
                self.squeeze_pos = 0;
                self.absorb_pos = 0;
            }

            y.push(self.poseidon.state[self.squeeze_pos]);
            self.squeeze_pos += 1;
        }

        self.io_count += 1;
        y
    }

    pub fn finish(&self) -> Result<(), String> {
        if self.io_count != self.io_pattern.0.len() {
            return Err("IO pattern mismatch".to_string());
        }

        Ok(())
    }

    fn permute(&mut self) {
        self.poseidon.permute();
        self.poseidon.pos = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shockwave_plus::PoseidonCurve;

    type Fp = frontend::ark_secp256k1::Fq;

    const WIDTH: usize = 3;
    const RATE: usize = 2;

    fn poseidon_sponge_circuit<F: FieldGC>(cs: &mut ConstraintSystem<F>) {
        let input = cs.alloc_priv_inputs(RATE);

        let mut poseidon_sponge = PoseidonSpongeChip::<F, WIDTH>::new(
            b"test",
            IOPattern::new(vec![]),
            PoseidonCurve::SECP256K1,
            cs,
        );
        poseidon_sponge.absorb(&input);
        let result = poseidon_sponge.squeeze(1)[0];
        result.println();
        cs.expose_public(result);
    }

    #[test]
    fn test_poseidon_sponge() {
        let synthesizer = |cs: &mut ConstraintSystem<Fp>| {
            poseidon_sponge_circuit(cs);
        };

        let input = [Fp::from(1234567), Fp::from(109987)];

        // Compute the expected hash
        let mut poseidon = PoseidonSponge::<Fp, WIDTH>::new(
            b"test",
            PoseidonCurve::SECP256K1,
            IOPattern::new(vec![]),
        );
        poseidon.absorb(&input);
        let expected_hash = poseidon.squeeze(1)[0];

        let mut cs = ConstraintSystem::new();
        let priv_input = input;
        let pub_input = vec![expected_hash];
        let witness = cs.gen_witness(synthesizer, &pub_input, &priv_input);

        cs.set_constraints(&synthesizer);
        assert!(cs.is_sat(&witness, &pub_input));

        println!("Num constraints: {}", cs.num_constraints.unwrap());
        println!("Num vars: {}", cs.num_vars());
    }
}
