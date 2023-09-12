use frontend::{ConstraintSystem, FieldGC, Wire};
use shockwave_plus::{IOPattern, PoseidonCurve, SpongeOp};

use crate::PoseidonSpongeChip;

const ARTY: usize = 2;
const SPONGE_WIDTH: usize = ARTY + 1; // The sponge capacity is one, so the width is arity + 1

pub fn verify_merkle_proof<F: FieldGC>(
    leaf: Wire<F>,
    siblings: &[Wire<F>],
    path_indices: &[Wire<F>],
    cs: &mut ConstraintSystem<F>,
) -> Wire<F> {
    let mut poseidon_sponge = PoseidonSpongeChip::<F, SPONGE_WIDTH>::new(
        SPONGE_WIDTH.to_string().as_bytes(),
        IOPattern::new(vec![SpongeOp::Absorb(2), SpongeOp::Squeeze(1)]),
        PoseidonCurve::SECP256K1,
        cs,
    );

    let mut node = leaf;
    for (sibling, path) in siblings.iter().zip(path_indices.iter()) {
        poseidon_sponge.absorb(&[node, *sibling]);
        let left = poseidon_sponge.squeeze(1)[0];
        poseidon_sponge.absorb(&[*sibling, node]);
        let right = poseidon_sponge.squeeze(1)[0];
        node = cs.if_then(path.is_zero(), left).else_then(right);
    }

    node
}

#[cfg(test)]
mod tests {
    use shockwave_plus::PoseidonCurve;
    use shockwave_plus::PoseidonSponge;

    use super::*;

    type Fp = frontend::ark_secp256k1::Fq;
    const TREE_DEPTH: usize = 5;

    #[test]
    pub fn test_verify_merkle_proof() {
        let mut poseidon_sponge = PoseidonSponge::<Fp, SPONGE_WIDTH>::new(
            SPONGE_WIDTH.to_string().as_bytes(),
            PoseidonCurve::SECP256K1,
            IOPattern::new(vec![SpongeOp::Absorb(2), SpongeOp::Squeeze(1)]),
        );

        let synthesizer = |cs: &mut ConstraintSystem<Fp>| {
            let leaf = cs.alloc_priv_input();
            let siblings = cs.alloc_priv_inputs(TREE_DEPTH);
            let path_indices = cs.alloc_priv_inputs(TREE_DEPTH);

            let node = verify_merkle_proof(leaf, &siblings, &path_indices, cs);
            cs.expose_public(node);
        };

        let siblings = (0..TREE_DEPTH)
            .map(|i| Fp::from(i as u64))
            .collect::<Vec<Fp>>();

        let path_indices = (0..TREE_DEPTH).map(|i| i % 3).collect::<Vec<usize>>();

        // Compute the expected root

        let leaf = Fp::from(3u32);
        let mut node = leaf;
        for (sibling, sel) in siblings.iter().zip(path_indices.iter()) {
            if sel & 1 == 1 {
                poseidon_sponge.absorb(&[node, *sibling]);
                node = poseidon_sponge.squeeze(1)[0];
            } else {
                poseidon_sponge.absorb(&[*sibling, node]);
                node = poseidon_sponge.squeeze(1)[0];
            }
        }

        let expected_root = node;

        // Run the circuit

        let mut cs = ConstraintSystem::new();
        let mut priv_input = vec![];
        priv_input.push(leaf);
        priv_input.extend_from_slice(&siblings);
        priv_input.extend_from_slice(
            &path_indices
                .iter()
                .map(|x| Fp::from(*x as u64))
                .collect::<Vec<Fp>>(),
        );

        let pub_input = [expected_root];
        let witness = cs.gen_witness(synthesizer, &pub_input, &priv_input);

        cs.set_constraints(&synthesizer);
        assert!(cs.is_sat(&witness, &pub_input));
    }
}
