use frontend::{ConstraintSystem, FieldGC, Wire};

use crate::Poseidon;

pub fn verify_merkle_proof<F: FieldGC>(
    leaf: Wire<F>,
    siblings: &[Wire<F>],
    path_indices: &[Wire<F>],
    cs: &mut ConstraintSystem<F>,
    poseidon: &mut Poseidon<F>,
) -> Wire<F> {
    let mut node = leaf;
    for (sibling, path) in siblings.iter().zip(path_indices.iter()) {
        let left = poseidon.hash(*sibling, node);
        poseidon.reset();
        let right = poseidon.hash(node, *sibling);
        poseidon.reset();
        node = cs.if_then(path.is_zero(), left).else_then(right);
    }

    node
}

#[cfg(test)]
mod tests {
    use crate::PoseidonConstants;
    use shockwave_plus::PoseidonCurve;

    use super::*;

    use shockwave_plus::Poseidon as PoseidonNative;
    use shockwave_plus::PoseidonConstants as PoseidonConstantsNative;

    type Fp = frontend::ark_secp256k1::Fq;
    const TREE_DEPTH: usize = 5;

    #[test]
    pub fn test_verify_merkle_proof() {
        let poseidon_constants = PoseidonConstantsNative::new(PoseidonCurve::SECP256K1);
        let mut poseidon = PoseidonNative::new(PoseidonCurve::SECP256K1);

        let synthesizer = |cs: &mut ConstraintSystem<Fp>| {
            let leaf = cs.alloc_priv_input();
            let siblings = cs.alloc_priv_inputs(TREE_DEPTH);
            let path_indices = cs.alloc_priv_inputs(TREE_DEPTH);

            let constants =
                PoseidonConstants::from_native_constants(poseidon_constants.clone(), cs);

            let mut poseidon_chip = Poseidon::<Fp>::new(cs, constants);

            let node = verify_merkle_proof(leaf, &siblings, &path_indices, cs, &mut poseidon_chip);
            cs.expose_public(node);
        };

        let siblings = [
            Fp::from(1u32),
            Fp::from(2u32),
            Fp::from(3u32),
            Fp::from(4u32),
            Fp::from(5u32),
        ];
        let path_indices = [0, 1, 1, 0, 0];

        let leaf = Fp::from(3u32);
        let mut node = leaf;
        for (sibling, sel) in siblings.iter().zip(path_indices.iter()) {
            if sel & 1 == 1 {
                node = poseidon.hash(&[node, *sibling]);
            } else {
                node = poseidon.hash(&[*sibling, node]);
            }
            poseidon.reset();
        }

        let expected_root = node;

        let mut cs = ConstraintSystem::new();
        let mut priv_input = vec![];
        priv_input.push(leaf);
        priv_input.extend_from_slice(&siblings);
        priv_input.extend_from_slice(
            &path_indices
                .iter()
                .map(|x| Fp::from(*x))
                .collect::<Vec<Fp>>(),
        );

        let pub_input = [expected_root];
        let witness = cs.gen_witness(synthesizer, &pub_input, &priv_input);

        cs.set_constraints(&synthesizer);
        assert!(cs.is_sat(&witness, &pub_input));
    }
}
