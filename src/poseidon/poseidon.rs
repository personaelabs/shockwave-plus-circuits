use frontend::FieldGC;
use frontend::{ConstraintSystem, Wire};
use shockwave_plus::PoseidonConstants;

#[derive(Clone)]
pub struct PoseidonChip<F: FieldGC> {
    pub state: [Wire<F>; 3],
    pub pos: usize,
    constants: PoseidonConstants<F>,
    cs: *mut ConstraintSystem<F>,
}

impl<F: FieldGC> PoseidonChip<F> {
    pub fn new(cs_ptr: *mut ConstraintSystem<F>, constants: PoseidonConstants<F>) -> Self {
        let cs = unsafe { &mut *cs_ptr };
        let tag = cs.alloc_const(F::from(3u32));
        let init_state = [
            tag,
            cs.alloc_const(F::from(0u32)),
            cs.alloc_const(F::from(0u32)),
        ];

        Self {
            state: init_state,
            constants,
            pos: 0,
            cs: cs_ptr,
        }
    }

    fn cs(&self) -> &mut ConstraintSystem<F> {
        unsafe { &mut *self.cs as &mut ConstraintSystem<F> }
    }

    pub fn reset(&mut self) {
        let cs = self.cs();
        let tag = cs.alloc_const(F::from(3u32));
        self.state = [tag, cs.one(), cs.one()];
        self.pos = 0;
    }

    // MDS matrix multiplication
    fn matrix_mul(&mut self) {
        let mut result = [self.cs().one(); 3];

        for (i, matrix) in self.constants.mds_matrix.iter().enumerate() {
            let deg2_comb_a = [
                (self.state[0], matrix[0]),
                (self.state[1], matrix[1]),
                (self.state[2], matrix[2]),
            ];

            let deg2_comb_b = [(self.cs().one(), F::ONE)];
            let deg_2_comb_c = [];
            result[i] = self
                .cs()
                .deg_2_comb(&deg2_comb_a, &deg2_comb_b, &deg_2_comb_c);
        }

        self.state = result;
    }

    fn full_round(&mut self) {
        let t = self.state.len();

        // Add round constants and apply the S-boxes
        for i in 0..t {
            let deg2_comb_a = [
                (self.state[i], F::ONE),
                (self.cs().one(), self.constants.round_keys[self.pos + i]),
            ];

            let deg2_comb_b = deg2_comb_a;

            let square = self.cs().deg_2_comb(&deg2_comb_a, &deg2_comb_b, &[]);
            let quadruple = square * square;

            let deg2_comb_a_2 = [(quadruple, F::ONE)];

            let deg2_comb_b = deg2_comb_a;

            self.state[i] = self.cs().deg_2_comb(&deg2_comb_a_2, &deg2_comb_b, &[]);
        }

        self.matrix_mul();

        // Update the position of the round constants that are added
        self.pos += self.state.len();
    }

    fn partial_round(&mut self) {
        // Apply the round constants
        for i in 1..3 {
            self.state[i] = self
                .cs()
                .add_const(self.state[i], self.constants.round_keys[self.pos + i]);
        }

        // S-box
        let deg2_comb_a = [
            (self.state[0], F::ONE),
            (self.cs().one(), self.constants.round_keys[self.pos + 0]),
        ];

        let deg2_comb_b = deg2_comb_a;

        let square = self.cs().deg_2_comb(&deg2_comb_a, &deg2_comb_b, &[]);
        let quadruple = square * square;

        let deg2_comb_a_2 = [(quadruple, F::ONE)];

        let deg2_comb_b = deg2_comb_a;

        self.state[0] = self.cs().deg_2_comb(&deg2_comb_a_2, &deg2_comb_b, &[]);

        self.matrix_mul();

        // Update the position of the round constants that are added
        self.pos += self.state.len();
    }

    pub fn permute(&mut self) {
        // ########################
        // First half of the full rounds
        // ########################

        // First half of full rounds
        for _ in 0..self.constants.num_full_rounds / 2 {
            self.full_round();
        }

        // Partial rounds
        for _ in 0..self.constants.num_partial_rounds {
            self.partial_round();
        }

        // Second half of full rounds
        for _ in 0..self.constants.num_full_rounds / 2 {
            self.full_round();
        }
    }

    pub fn hash(&mut self, i1: Wire<F>, i2: Wire<F>) -> Wire<F> {
        self.state[1] = i1;
        self.state[2] = i2;

        self.permute();

        self.state[1]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_std::{end_timer, start_timer};
    use frontend::circuit;
    use frontend::wasm_deps::*;
    use shockwave_plus::Poseidon;
    use shockwave_plus::PoseidonConstants;
    use shockwave_plus::PoseidonCurve;

    type Fp = frontend::ark_secp256k1::Fq;

    fn poseidon_circuit<F: FieldGC>(cs: &mut ConstraintSystem<F>) {
        let i1 = cs.alloc_priv_input();
        let i2 = cs.alloc_priv_input();

        let constants_native = PoseidonConstants::<F>::new(PoseidonCurve::SECP256K1);
        // let constants = PoseidonChipConstants::from_native_constants(constants_native.clone(), cs);

        let mut poseidon_chip = PoseidonChip::<F>::new(cs, constants_native);
        let n = 4943;
        for _ in 0..n {
            poseidon_chip.hash(i1, i2);
            poseidon_chip.reset();
        }
        let result = poseidon_chip.hash(i1, i2);
        cs.expose_public(result);
    }

    circuit!(
        |cs: &mut ConstraintSystem<Fp>| {
            poseidon_circuit(cs);
        },
        Fp
    );

    #[test]
    fn test_poseidon() {
        let synthesizer = |cs: &mut ConstraintSystem<Fp>| {
            poseidon_circuit(cs);
        };

        let priv_input = [Fp::from(1234567), Fp::from(109987)];
        let mut poseidon = Poseidon::new(PoseidonCurve::SECP256K1);
        let expected_hash = poseidon.hash(&priv_input);

        let mut cs = ConstraintSystem::new();
        let pub_input = vec![expected_hash];
        let witness_gen_timer = start_timer!(|| "Witness generation");
        let witness = cs.gen_witness(synthesizer, &pub_input, &priv_input);
        end_timer!(witness_gen_timer);

        cs.set_constraints(&synthesizer);

        println!("Num constraints: {}", cs.num_constraints.unwrap());
        println!("Num vars: {}", cs.num_vars());

        assert!(cs.is_sat(&witness, &pub_input));
    }
}
