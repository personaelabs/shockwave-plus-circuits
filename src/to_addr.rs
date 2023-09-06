use frontend::FieldGC;
use frontend::Wire;
use keccak::{RATE, RC, RHO_OFFSETS, ROUNDS};
use num_bigint::BigUint;

use crate::bitops::{from_bits, not_a_and_b_64, rotate_left_64, xor_64};

pub fn to_addr<F: FieldGC>(input: [Wire<F>; 512]) -> Wire<F> {
    let cs = input[0].cs();
    let zero = cs.alloc_const(F::ZERO);
    let one = cs.one();

    // Pad
    let mut pad = [zero; RATE - 512];
    pad[0] = cs.one();
    pad[pad.len() - 1] = cs.one();

    let mut padded_input = [zero; 1600];
    padded_input[..512].copy_from_slice(&input);
    padded_input[512..(512 + pad.len())].copy_from_slice(&pad);

    let mut state = [[zero; 64]; 25];

    for i in 0..25 {
        state[i] = padded_input[i * 64..(i + 1) * 64].try_into().unwrap();
    }

    // Assign the round constants
    let rc: [[Wire<F>; 64]; 24] = RC.map(|c| {
        let mut c_assigned = Vec::with_capacity(64);
        for i in 0..64 {
            if c >> i & 1 == 1 {
                c_assigned.push(one);
            } else {
                c_assigned.push(zero);
            }
        }

        c_assigned.try_into().unwrap()
    });

    for i in 0..ROUNDS {
        // Theta
        let mut c = [[zero; 64]; 5];
        let mut d = [[zero; 64]; 5];

        for y in 0..5 {
            for x in 0..5 {
                c[x] = xor_64(c[x], state[x + y * 5]);
            }
        }

        for x in 0..5 {
            d[x] = xor_64(c[(x + 4) % 5], rotate_left_64(c[(x + 1) % 5], 1));
        }

        for y in 0..5 {
            for x in 0..5 {
                state[x + y * 5] = xor_64(state[x + y * 5], d[x]);
            }
        }

        // ############################################
        // Rho
        // ############################################
        let mut rho_x = 0;
        let mut rho_y = 1;
        for _ in 0..24 {
            // Rotate each lane by an offset
            let index = rho_x + 5 * rho_y;
            state[index] = rotate_left_64(state[index], (RHO_OFFSETS[rho_y][rho_x] % 64) as usize);

            let rho_x_prev = rho_x;
            rho_x = rho_y;
            rho_y = (2 * rho_x_prev + 3 * rho_y) % 5;
        }

        // ############################################
        // Pi
        // ############################################

        let state_cloned = state.clone();
        for y in 0..5 {
            for x in 0..5 {
                let index = ((x + 3 * y) % 5) + x * 5;
                state[x + y * 5] = state_cloned[index];
            }
        }

        // ############################################
        // Chi
        // ############################################

        let state_cloned = state.clone();
        for y in 0..5 {
            for x in 0..5 {
                let index = x + y * 5;
                state[index] = xor_64(
                    state_cloned[index],
                    not_a_and_b_64(
                        state_cloned[(x + 1) % 5 + y * 5],
                        state_cloned[(x + 2) % 5 + y * 5],
                    ),
                );
            }
        }

        // ############################################
        // Iota
        // ############################################

        state[0] = xor_64(state[0], rc[i]);
    }

    let state_0 = from_bits(&state[0]);
    let state_1 = from_bits(&state[1]) * cs.alloc_const(F::from(BigUint::from(1u32) << 64));
    let state_2 = from_bits(&state[2]) * cs.alloc_const(F::from(BigUint::from(1u32) << 128));
    let state_3 = from_bits(&state[3]) * cs.alloc_const(F::from(BigUint::from(1u32) << 192));

    let out = state_0 + state_1 + state_2 + state_3;

    out
}

#[cfg(test)]
mod tests {
    use frontend::ark_ff::Field;
    use frontend::wasm_deps::*;
    use frontend::ConstraintSystem;
    use keccak::keccak256;

    use super::*;

    type Fp = frontend::ark_secp256k1::Fq;

    fn to_addr_circuit<F: FieldGC>(cs: &mut ConstraintSystem<F>) {
        let pub_key_bits = cs.alloc_priv_inputs(512);

        let addr = to_addr(pub_key_bits.try_into().unwrap());
        cs.expose_public(addr);
    }

    #[test]
    fn test_to_addr() {
        let pub_key_str = "4f355bdcb7cc0af728ef3cceb9615d90684bb5b2ca5f859ab0f0b704075871aa385b6b1b8ead809ca67454d9683fcf2ba03456d6fe2c4abe2b07f0fbdbb2f1c1";
        let pub_key_bytes = hex::decode(pub_key_str).unwrap();

        let pub_key_bits = pub_key_bytes
            .iter()
            .map(|b| {
                // Little-endian bits
                let mut bits = Vec::with_capacity(8);
                for i in 0..8 {
                    bits.push(if (*b >> i) & 1 == 1 {
                        Fp::ONE
                    } else {
                        Fp::ZERO
                    });
                }

                bits
            })
            .flatten()
            .collect::<Vec<Fp>>();

        let synthesizer = |cs: &mut ConstraintSystem<Fp>| {
            to_addr_circuit(cs);
        };

        let mut cs = ConstraintSystem::new();

        let priv_input = pub_key_bits;
        let addr = Fp::from(BigUint::from_bytes_le(&keccak256(&pub_key_bytes)));
        let pub_input = [addr];

        let witness = cs.gen_witness(synthesizer, &pub_input, &priv_input);

        cs.set_constraints(&synthesizer);
        assert!(cs.is_sat(&witness, &pub_input));
    }
}
