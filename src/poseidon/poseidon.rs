use frontend::FieldGC;
use frontend::{ConstraintSystem, Wire};
use shockwave_plus::PoseidonConstants;

#[derive(Clone)]
pub struct PoseidonChip<F: FieldGC, const WIDTH: usize> {
    pub state: [Wire<F>; WIDTH],
    pub pos: usize,
    constants: PoseidonConstants<F>,
    cs: *mut ConstraintSystem<F>,
}

impl<F: FieldGC, const WIDTH: usize> PoseidonChip<F, WIDTH> {
    pub fn new(cs_ptr: *mut ConstraintSystem<F>, constants: PoseidonConstants<F>) -> Self {
        let cs = unsafe { &mut *cs_ptr };
        let zero = cs.alloc_const(F::ZERO);
        let init_state = [zero; WIDTH];

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
        let zero = cs.alloc_const(F::ZERO);
        self.state = [zero; WIDTH];
        self.pos = 0;
    }

    // MDS matrix multiplication
    fn matrix_mul(&mut self) {
        let mut result = [self.cs().one(); WIDTH];

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
}
