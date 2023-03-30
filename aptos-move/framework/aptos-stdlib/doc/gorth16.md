
<a name="0x1_groth16"></a>

# Module `0x1::groth16`

Generic implementation of Groth16 (proof verification) as defined in https://eprint.iacr.org/2016/260.pdf, Section 3.2.
Actual proof verifiers can be constructed using the pairings supported in the generic algebra module.
See the test cases in this module for an example of constructing with BLS12-381 curves.


-  [Function `verify_proof`](#0x1_groth16_verify_proof)
-  [Function `verify_proof_prepared`](#0x1_groth16_verify_proof_prepared)


<pre><code><b>use</b> <a href="algebra.md#0x1_algebra">0x1::algebra</a>;
<b>use</b> <a href="../../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a name="0x1_groth16_verify_proof"></a>

## Function `verify_proof`

Proof verification as specifid in the original paper, with the following input.
- Verification key: $\left([\alpha]_1, [\beta]_2, [\gamma]_2, [\delta]_2, \left\\{ \left[ \frac{\beta \cdot u_i(x) + \alpha \cdot v_i(x) + w_i(x)}{\gamma} \right]_1 \right\\}\_{i=0}^l \right)$.
- Public inputs: $\\{a_i\\}_{i=1}^l$.
- Proof $\left( \left[ A \right]_1, \left[ B \right]_2, \left[ C \right]_1 \right)$.


<pre><code><b>public</b> <b>fun</b> <a href="gorth16.md#0x1_groth16_verify_proof">verify_proof</a>&lt;G1, G2, Gt, S&gt;(vk_alpha_g1: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G1&gt;, vk_beta_g2: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;, vk_gamma_g2: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;, vk_delta_g2: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;, vk_uvw_gamma_g1: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G1&gt;&gt;, public_inputs: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;&gt;, proof_a: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G1&gt;, proof_b: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;, proof_c: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G1&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="gorth16.md#0x1_groth16_verify_proof">verify_proof</a>&lt;G1,G2,Gt,S&gt;(
    vk_alpha_g1: &Element&lt;G1&gt;,
    vk_beta_g2: &Element&lt;G2&gt;,
    vk_gamma_g2: &Element&lt;G2&gt;,
    vk_delta_g2: &Element&lt;G2&gt;,
    vk_uvw_gamma_g1: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Element&lt;G1&gt;&gt;,
    public_inputs: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Element&lt;S&gt;&gt;,
    proof_a: &Element&lt;G1&gt;,
    proof_b: &Element&lt;G2&gt;,
    proof_c: &Element&lt;G1&gt;,
): bool {
    <b>let</b> left = pairing&lt;G1,G2,Gt&gt;(proof_a, proof_b);
    <b>let</b> scalars = <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[from_u64&lt;S&gt;(1)];
    std::vector::append(&<b>mut</b> scalars, *public_inputs);
    <b>let</b> right = zero&lt;Gt&gt;();
    <b>let</b> right = add(&right, &pairing&lt;G1,G2,Gt&gt;(vk_alpha_g1, vk_beta_g2));
    <b>let</b> right = add(&right, &pairing(&multi_scalar_mul(vk_uvw_gamma_g1, &scalars), vk_gamma_g2));
    <b>let</b> right = add(&right, &pairing(proof_c, vk_delta_g2));
    eq(&left, &right)
}
</code></pre>



</details>

<a name="0x1_groth16_verify_proof_prepared"></a>

## Function `verify_proof_prepared`

Proof verification optimized for low verification latency but requiring pre-computation, with the following input.
- Prepared verification key: $\left([\alpha]_1 \cdot [\beta]_2, -[\gamma]_2, -[\delta]_2, \left\\{ \left[ \frac{\beta \cdot u_i(x) + \alpha \cdot v_i(x) + w_i(x)}{\gamma} \right]_1 \right\\}\_{i=0}^l \right)$.
- Public inputs: $\\{a_i\\}_{i=1}^l$.
- Proof: $\left( \left[ A \right]_1, \left[ B \right]_2, \left[ C \right]_1 \right)$.


<pre><code><b>public</b> <b>fun</b> <a href="gorth16.md#0x1_groth16_verify_proof_prepared">verify_proof_prepared</a>&lt;G1, G2, Gt, GtParent, S&gt;(pvk_alpha_g1_beta_g2: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;GtParent&gt;, pvk_gamma_g2_neg: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;, pvk_delta_g2_neg: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;, pvk_uvw_gamma_g1: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G1&gt;&gt;, public_inputs: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;&gt;, proof_a: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G1&gt;, proof_b: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;, proof_c: &<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G1&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="gorth16.md#0x1_groth16_verify_proof_prepared">verify_proof_prepared</a>&lt;G1,G2,Gt,GtParent,S&gt;(
    pvk_alpha_g1_beta_g2: &Element&lt;GtParent&gt;,
    pvk_gamma_g2_neg: &Element&lt;G2&gt;,
    pvk_delta_g2_neg: &Element&lt;G2&gt;,
    pvk_uvw_gamma_g1: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Element&lt;G1&gt;&gt;,
    public_inputs: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Element&lt;S&gt;&gt;,
    proof_a: &Element&lt;G1&gt;,
    proof_b: &Element&lt;G2&gt;,
    proof_c: &Element&lt;G1&gt;,
): bool {
    <b>let</b> scalars = <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[from_u64&lt;S&gt;(1)];
    std::vector::append(&<b>mut</b> scalars, *public_inputs);
    <b>let</b> g1_elements = <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[*proof_a, multi_scalar_mul(pvk_uvw_gamma_g1, &scalars), *proof_c];
    <b>let</b> g2_elements = <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[*proof_b, *pvk_gamma_g2_neg, *pvk_delta_g2_neg];
    eq(pvk_alpha_g1_beta_g2, &upcast(&multi_pairing&lt;G1,G2,Gt&gt;(&g1_elements, &g2_elements)))
}
</code></pre>



</details>


[move-book]: https://aptos.dev/guides/move-guides/book/SUMMARY
