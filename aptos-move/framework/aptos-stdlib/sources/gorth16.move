module aptos_std::groth16 {
    use aptos_std::algebra::{Element, pairing, neg, from_u64, multi_scalar_mul, add, eq, multi_pairing};
    #[test_only]
    use aptos_std::algebra_bls12381;
    #[test_only]
    use aptos_std::algebra::{deserialize, enable_cryptography_algebra_natives};
    #[test_only]
    use aptos_std::algebra_bls12381::{Fr, FrFormatLsb, G1AffineFormatCompressed, G2AffineFormatCompressed, GtFormat};

    /// A Groth16 verifying key.
    struct VerifyingKey<phantom G1, phantom G2, phantom Gt> has drop {
        alpha_g1: Element<G1>,
        beta_g2: Element<G2>,
        gamma_g2: Element<G2>,
        delta_g2: Element<G2>,
        gamma_abc_g1: vector<Element<G1>>,
    }

    /// A Groth16 verifying key pre-processed for faster verification.
    struct PreparedVerifyingKey<phantom G1, phantom G2, phantom Gt> has drop {
        alpha_g1_beta_g2: Element<Gt>,
        gamma_g2_neg: Element<G2>,
        delta_g2_neg: Element<G2>,
        gamma_abc_g1: vector<Element<G1>>,
    }

    /// A Groth16 proof.
    struct Proof<phantom G1, phantom G2, phantom Gt> has drop {
        a: Element<G1>,
        b: Element<G2>,
        c: Element<G1>,
    }

    /// Create a new Groth16 verifying key.
    public fun new_vk<G1,G2,Gt>(alpha_g1: Element<G1>, beta_g2: Element<G2>, gamma_g2: Element<G2>, delta_g2: Element<G2>, gamma_abc_g1: vector<Element<G1>>): VerifyingKey<G1,G2,Gt> {
        VerifyingKey {
            alpha_g1,
            beta_g2,
            gamma_g2,
            delta_g2,
            gamma_abc_g1,
        }
    }

    /// Create a new pre-processed Groth16 verifying key.
    public fun new_pvk<G1,G2,Gt>(alpha_g1_beta_g2: Element<Gt>, gamma_g2_neg: Element<G2>, delta_g2_neg: Element<G2>, gamma_abc_g1: vector<Element<G1>>): PreparedVerifyingKey<G1,G2,Gt> {
        PreparedVerifyingKey {
            alpha_g1_beta_g2,
            gamma_g2_neg,
            delta_g2_neg,
            gamma_abc_g1,
        }
    }

    /// Pre-process a Groth16 verification key `vk` for faster verification.
    public fun prepare_verifying_key<G1,G2,Gt>(vk: &VerifyingKey<G1,G2,Gt>): PreparedVerifyingKey<G1,G2,Gt> {
        PreparedVerifyingKey {
            alpha_g1_beta_g2: pairing<G1,G2,Gt>(&vk.alpha_g1, &vk.beta_g2),
            gamma_g2_neg: neg(&vk.gamma_g2),
            delta_g2_neg: neg(&vk.delta_g2),
            gamma_abc_g1: vk.gamma_abc_g1,
        }
    }

    /// Create a Groth16 proof.
    public fun new_proof<G1,G2,Gt>(a: Element<G1>, b: Element<G2>, c: Element<G1>): Proof<G1,G2,Gt> {
        Proof { a, b, c }
    }

    /// Verify a Groth16 proof.
    public fun verify_proof<G1,G2,Gt,S>(vk: &VerifyingKey<G1,G2,Gt>, public_inputs: &vector<Element<S>>, proof: &Proof<G1,G2,Gt>): bool {
        let left = pairing<G1,G2,Gt>(&proof.a, &proof.b);
        let right_1 = pairing<G1,G2,Gt>(&vk.alpha_g1, &vk.beta_g2);
        let scalars = vector[from_u64<S>(1)];
        std::vector::append(&mut scalars, *public_inputs);
        let right_2 = pairing(&multi_scalar_mul(&vk.gamma_abc_g1, &scalars), &vk.gamma_g2);
        let right_3 = pairing(&proof.c, &vk.delta_g2);
        let right = add(&add(&right_1, &right_2), &right_3);
        eq(&left, &right)
    }

    /// Verify a Groth16 proof `proof` against the public inputs `public_inputs` with a prepared verification key `pvk`.
    public fun verify_proof_with_pvk<G1,G2,Gt,S>(pvk: &PreparedVerifyingKey<G1,G2,Gt>, public_inputs: &vector<Element<S>>, proof: &Proof<G1,G2,Gt>): bool {
        let scalars = vector[from_u64<S>(1)];
        std::vector::append(&mut scalars, *public_inputs);
        let g1_elements: vector<Element<G1>> = vector[proof.a, multi_scalar_mul(&pvk.gamma_abc_g1, &scalars), proof.c];
        let g2_elements: vector<Element<G2>> = vector[proof.b, pvk.gamma_g2_neg, pvk.delta_g2_neg];

        eq(&pvk.alpha_g1_beta_g2, &multi_pairing<G1,G2,Gt>(&g1_elements, &g2_elements))
    }

    #[test(fx = @std)]
    fun test_verify_mimc_proof(fx: signer) {
        enable_cryptography_algebra_natives(&fx);

        let gamma_abc_g1: vector<Element<algebra_bls12381::G1Affine>> = vector[
            std::option::extract(&mut deserialize<algebra_bls12381::G1Affine, G1AffineFormatCompressed>(&x"b0df760d0f2d67fdff69d0ed3a0653dd8808df3c407ea4d0e27f8612c3fbb748cb4372d33cac512ee5ef4ee1683c3fe5")),
            std::option::extract(&mut deserialize<algebra_bls12381::G1Affine, G1AffineFormatCompressed>(&x"96ec80d6b1050bbfc209f727678acce8788c05475771daffdd444ad8786c7a40195d859850fe2e72be3054e9fb8ce805")),
        ];

        let vk = new_vk<algebra_bls12381::G1Affine, algebra_bls12381::G2Affine, algebra_bls12381::Gt>(
            std::option::extract(&mut deserialize<algebra_bls12381::G1Affine, G1AffineFormatCompressed>(&x"9819f632fa8d724e351d25081ea31ccf379991ac25c90666e07103fffb042ed91c76351cd5a24041b40e26d231a5087e")), //alpha_g1
            std::option::extract(&mut deserialize<algebra_bls12381::G2Affine, G2AffineFormatCompressed>(&x"871f36a996c71a89499ffe99aa7d3f94decdd2ca8b070dbb467e42d25aad918af6ec94d61b0b899c8f724b2b549d99fc1623a0e51b6cfbea220e70e7da5803c8ad1144a67f98934a6bf2881ec6407678fd52711466ad608d676c60319a299824")), //beta_g2
            std::option::extract(&mut deserialize<algebra_bls12381::G2Affine, G2AffineFormatCompressed>(&x"96750d8445596af8d679487c7267ae9734aeac584ace191d225680a18ecff8ebae6dd6a5fd68e4414b1611164904ee120363c2b49f33a873d6cfc26249b66327a0de03e673b8139f79809e8b641586cde9943fa072ee5ed701c81b3fd426c220")), //gamma_g2
            std::option::extract(&mut deserialize<algebra_bls12381::G2Affine, G2AffineFormatCompressed>(&x"8d3ac832f2508af6f01872ada87ea66d2fb5b099d34c5bac81e7482c956276dfc234c8d2af5fd2394b5440d0708a2c9f124a53c0755e9595cf9f8adade5deefcb8a574a67debd3b74d08c49c23ddc14cd6d48b65dce500c8a5d330e760fe85bb")), //delta_g2
            gamma_abc_g1
        );

        let proof = new_proof<algebra_bls12381::G1Affine, algebra_bls12381::G2Affine, algebra_bls12381::Gt>(
            std::option::extract(&mut deserialize<algebra_bls12381::G1Affine, G1AffineFormatCompressed>(&x"8a7c7364403d91bfa5c723ce93b920c8d2e559ea5e7e34eb68cea437aa4f26bf56ba22d9400988a86f2943c79401e959")),
            std::option::extract(&mut deserialize<algebra_bls12381::G2Affine, G2AffineFormatCompressed>(&x"9352f8a2f9ff60d390e363d063354e9728adf39c91294499575855e803dd80eeaa1488cd24d1b80eb1b2625011e22a5d139e24f2c7ac3508874ec4bdb9c71ddf109e7853d641d23ed27bef265248d78eabe9137c03b088d8adbdf39e10f87eab")),
            std::option::extract(&mut deserialize<algebra_bls12381::G1Affine, G1AffineFormatCompressed>(&x"896f68b438e076d3017e64aa47621fcd69b45f49a7038e2b1b9ed4f2de9b8eb8e0a76785a39a08f024435811a73a6818"))
        );

        let public_inputs: vector<Element<Fr>> = vector[
            std::option::extract(&mut deserialize<Fr, FrFormatLsb>(&x"0ee291cfc951388c3c7f7c85ff2dfd42bbc66a6b4acaef9a5a51ce955125a74f")),
        ];
        assert!(verify_proof(&vk, &public_inputs, &proof), 1);

        let pvk = prepare_verifying_key(&vk);
        assert!(verify_proof_with_pvk(&pvk, &public_inputs, &proof), 1);
    }

    #[test(fx = @std)]
    fun test_verify_mimc_proof_with_pvk(fx: signer) {
        enable_cryptography_algebra_natives(&fx);

        let gamma_abc_g1: vector<Element<algebra_bls12381::G1Affine>> = vector[
            std::option::extract(&mut deserialize<algebra_bls12381::G1Affine, G1AffineFormatCompressed>(&x"b0df760d0f2d67fdff69d0ed3a0653dd8808df3c407ea4d0e27f8612c3fbb748cb4372d33cac512ee5ef4ee1683c3fe5")),
            std::option::extract(&mut deserialize<algebra_bls12381::G1Affine, G1AffineFormatCompressed>(&x"96ec80d6b1050bbfc209f727678acce8788c05475771daffdd444ad8786c7a40195d859850fe2e72be3054e9fb8ce805")),
        ];

        let pvk = new_pvk<algebra_bls12381::G1Affine,algebra_bls12381::G2Affine,algebra_bls12381::Gt>(
            std::option::extract(&mut deserialize<algebra_bls12381::Gt, GtFormat>(&x"15cee98b42f8d158f421bce13983e23597123817a3b19b006294b9145f3f382686706ad9161d6234661fb1a32da19d0e2a9e672901fe4abe9efd4da96bcdb8324459b93aa48a8abb92ddd28ef053f118e190eddd6c6212bc09428ea05e709104290e37f320a3aac1dcf96f66efd9f5826b69cd075b72801ef54ccb740a0947bb3f73174e5d2fdc04292f58841ad9cc0d0c25021dfd8d592943b5e61c97f1ba68dcabd7de970ecc347c04bbaf9a062d9d49476f0b5bc77b2b9c7222781c53b713c0aae7a4cc57ff8cfb433d27fb1328d0c5453dbb97f3a70e9ce3b1da52cee2047cad225410b6dacb28e7b6876795d005cf0aefb7f25350d0197a5c2aa7369a5e06a210580bba1cc1941e1871a465cf68c84f32a29e6e898e4961a2b1fd5f8f03f03b1e1a0e191becdc8f01fb15adeb7cb6cc39e686edfcf7d65e952cf5e19a477fb5f6d2dab61a4d6c07777c1842150646c8b6fcb5989d9e524a97e7bf8b7be6b12983205970f16aeaccbdbe6cd565fa570dc45b0ad8f51c46e1f05e9f3f230dcf7567db5fc9a59a55c39139c7b357103c26bca9b70032cccff2345b76f596901ea81dc28f1d490a129501cf02204e00e8b59770188d69379144629239933523a8ec71ce6f91fbd01b2b9c411f89948183fea3949d89919e239a4aadb2347803e97ae8f7f20ade26da001f803cd61eb9bf8a67356f7cf6ec1744720b078eb992529f5c219bf16d5ef2e233a04572730e7c9572eadd9aa63c69c9f7dcf3423b1dc4c9b2032c8a7bbe91505283163a85413ecf0a0095fe1899b29f60011226f009")), //alpha_g1_beta_g2
            std::option::extract(&mut deserialize<algebra_bls12381::G2Affine, G2AffineFormatCompressed>(&x"b6750d8445596af8d679487c7267ae9734aeac584ace191d225680a18ecff8ebae6dd6a5fd68e4414b1611164904ee120363c2b49f33a873d6cfc26249b66327a0de03e673b8139f79809e8b641586cde9943fa072ee5ed701c81b3fd426c220")), //gamma_g2_neg
            std::option::extract(&mut deserialize<algebra_bls12381::G2Affine, G2AffineFormatCompressed>(&x"ad3ac832f2508af6f01872ada87ea66d2fb5b099d34c5bac81e7482c956276dfc234c8d2af5fd2394b5440d0708a2c9f124a53c0755e9595cf9f8adade5deefcb8a574a67debd3b74d08c49c23ddc14cd6d48b65dce500c8a5d330e760fe85bb")), //delta_g2_neg
            gamma_abc_g1
        );

        let proof = new_proof<algebra_bls12381::G1Affine, algebra_bls12381::G2Affine, algebra_bls12381::Gt>(
            std::option::extract(&mut deserialize<algebra_bls12381::G1Affine, G1AffineFormatCompressed>(&x"8a7c7364403d91bfa5c723ce93b920c8d2e559ea5e7e34eb68cea437aa4f26bf56ba22d9400988a86f2943c79401e959")),
            std::option::extract(&mut deserialize<algebra_bls12381::G2Affine, G2AffineFormatCompressed>(&x"9352f8a2f9ff60d390e363d063354e9728adf39c91294499575855e803dd80eeaa1488cd24d1b80eb1b2625011e22a5d139e24f2c7ac3508874ec4bdb9c71ddf109e7853d641d23ed27bef265248d78eabe9137c03b088d8adbdf39e10f87eab")),
            std::option::extract(&mut deserialize<algebra_bls12381::G1Affine, G1AffineFormatCompressed>(&x"896f68b438e076d3017e64aa47621fcd69b45f49a7038e2b1b9ed4f2de9b8eb8e0a76785a39a08f024435811a73a6818"))
        );

        let public_inputs: vector<Element<Fr>> = vector[
            std::option::extract(&mut deserialize<Fr, FrFormatLsb>(&x"0ee291cfc951388c3c7f7c85ff2dfd42bbc66a6b4acaef9a5a51ce955125a74f")),
        ];
        assert!(verify_proof_with_pvk(&pvk, &public_inputs, &proof), 1);
    }
}
