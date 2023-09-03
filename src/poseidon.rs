use frontend::ark_ff::PrimeField;
use frontend::{ConstraintSystem, Wire};

pub struct PoseidonConstants<F: PrimeField> {
    round_constants: Vec<Wire<F>>,
    mds_matrix: Vec<Vec<Wire<F>>>,
    num_full_rounds: usize,
    num_partial_rounds: usize,
}

pub struct Poseidon<F: PrimeField> {
    state: [Wire<F>; 3],
    pos: usize,
    constants: PoseidonConstants<F>,
    cs: *mut ConstraintSystem<F>,
}

impl<F: PrimeField> Poseidon<F> {
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
        self.pos = 0;
    }

    fn add_constants(&mut self) {
        // Add round constants
        for i in 0..self.state.len() {
            self.state[i] = self.state[i] + self.constants.round_constants[self.pos + i];
        }
    }

    // MDS matrix multiplication
    fn matrix_mul(&mut self) {
        let mut result = [self.cs().one(); 3];

        for (i, val) in self.constants.mds_matrix.iter().enumerate() {
            let mut tmp = self.cs().one();
            for (j, element) in self.state.iter().enumerate() {
                if j == 0 {
                    tmp = val[j] * *element;
                } else {
                    tmp = tmp + (val[j] * *element);
                }
            }
            result[i] = tmp;
        }

        self.state = result;
    }

    fn s_box(&mut self, x: Wire<F>) -> Wire<F> {
        let square = x * x;
        let quadruple = square * square;
        quadruple * x
    }

    fn full_round(&mut self) {
        let t = self.state.len();

        // Apply s-box
        self.add_constants();

        // S-boxes
        for i in 0..t {
            self.state[i] = self.s_box(self.state[i]);
        }

        self.matrix_mul();

        // Update the position of the round constants that are added
        self.pos += self.state.len();
    }

    fn partial_round(&mut self) {
        self.add_constants();

        // S-box
        self.state[0] = self.s_box(self.state[0]);

        self.matrix_mul();

        // Update the position of the round constants that are added
        self.pos += self.state.len();
    }

    pub fn hash(&mut self, i1: Wire<F>, i2: Wire<F>) -> Wire<F> {
        self.state[1] = i1;
        self.state[2] = i2;

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

        self.state[1]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shockwave_plus::poseidon_constants::secp256k1::{
        MDS_MATRIX, NUM_FULL_ROUNDS, NUM_PARTIAL_ROUNDS, ROUND_CONSTANTS,
    };
    use shockwave_plus::Poseidon as PoseidonNative;
    use shockwave_plus::PoseidonConstants as PoseidonConstantsNative;
    type Fp = frontend::ark_secp256k1::Fq;

    #[test]
    fn test_poseidon() {
        let synthesizer = |cs: &mut ConstraintSystem<Fp>| {
            let i1 = cs.alloc_priv_input();
            let i2 = cs.alloc_priv_input();

            let constants = PoseidonConstants {
                round_constants: ROUND_CONSTANTS.iter().map(|c| cs.alloc_const(*c)).collect(),
                mds_matrix: MDS_MATRIX
                    .iter()
                    .map(|row| row.iter().map(|c| cs.alloc_const(*c)).collect())
                    .collect(),
                num_full_rounds: NUM_FULL_ROUNDS,
                num_partial_rounds: NUM_PARTIAL_ROUNDS,
            };

            let mut poseidon_chip = Poseidon::<Fp>::new(cs, constants);
            let result = poseidon_chip.hash(i1, i2);

            cs.expose_public(result);
        };

        let priv_input = [Fp::from(1234567), Fp::from(109987)];
        let mut poseidon = PoseidonNative::new(PoseidonConstantsNative::new(
            ROUND_CONSTANTS.to_vec(),
            vec![
                MDS_MATRIX[0].to_vec(),
                MDS_MATRIX[1].to_vec(),
                MDS_MATRIX[2].to_vec(),
            ],
            NUM_FULL_ROUNDS,
            NUM_PARTIAL_ROUNDS,
        ));
        let expected_hash = poseidon.hash(&priv_input);

        let mut cs = ConstraintSystem::new();
        let pub_input = vec![expected_hash];
        let witness = cs.gen_witness(synthesizer, &pub_input, &priv_input);

        assert!(cs.is_sat(&witness, &pub_input, synthesizer));
    }
}