use std::result::Result;

use frontend::{ConstraintSystem, Wire};
use shockwave_plus::{FieldGC, IOPattern, PoseidonCurve, PoseidonSponge, SpongeOp};

use crate::PoseidonChip;

// Implements SAFE (Sponge API for Field Elements): https://hackmd.io/bHgsH6mMStCVibM_wYvb2w
#[derive(Clone)]
pub struct PoseidonSpongeChip<F: FieldGC> {
    pub absorb_pos: usize,
    pub squeeze_pos: usize,
    pub io_count: usize,
    pub io_pattern: IOPattern,
    pub rate: usize,
    pub capacity: usize,
    pub poseidon: PoseidonChip<F>,
}

impl<F: FieldGC> PoseidonSpongeChip<F> {
    pub fn new(
        domain_separator: &[u8],
        io_pattern: IOPattern,
        curve: PoseidonCurve,
        cs_ptr: *mut ConstraintSystem<F>,
    ) -> Self {
        let cs = unsafe { &mut *cs_ptr };

        let constants = shockwave_plus::PoseidonConstants::<F>::new(curve);
        let poseidon = PoseidonChip::new(cs, constants);

        let tag = cs.alloc_const(PoseidonSponge::<F>::compute_tag(
            domain_separator,
            &io_pattern,
        ));

        let zero = cs.alloc_const(F::ZERO);
        let state = [tag, zero, zero];

        let mut poseidon = poseidon.clone();
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
