
<a name="0x1_groth16"></a>

# Module `0x1::groth16`



-  [Struct `VerifyingKey`](#0x1_groth16_VerifyingKey)
-  [Struct `PreparedVerifyingKey`](#0x1_groth16_PreparedVerifyingKey)
-  [Struct `Proof`](#0x1_groth16_Proof)
-  [Function `new_vk`](#0x1_groth16_new_vk)
-  [Function `new_pvk`](#0x1_groth16_new_pvk)
-  [Function `prepare_verifying_key`](#0x1_groth16_prepare_verifying_key)
-  [Function `new_proof`](#0x1_groth16_new_proof)
-  [Function `verify_proof`](#0x1_groth16_verify_proof)
-  [Function `verify_proof_with_pvk`](#0x1_groth16_verify_proof_with_pvk)


<pre><code><b>use</b> <a href="algebra.md#0x1_algebra">0x1::algebra</a>;
<b>use</b> <a href="../../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a name="0x1_groth16_VerifyingKey"></a>

## Struct `VerifyingKey`

A Groth16 verifying key.


<pre><code><b>struct</b> <a href="gorth16.md#0x1_groth16_VerifyingKey">VerifyingKey</a>&lt;G1, G2, Gt&gt; <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>alpha_g1: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G1&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>beta_g2: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>gamma_g2: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>delta_g2: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>gamma_abc_g1: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G1&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_groth16_PreparedVerifyingKey"></a>

## Struct `PreparedVerifyingKey`

A Groth16 verifying key pre-processed for faster verification.


<pre><code><b>struct</b> <a href="gorth16.md#0x1_groth16_PreparedVerifyingKey">PreparedVerifyingKey</a>&lt;G1, G2, Gt&gt; <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>alpha_g1_beta_g2: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;Gt&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>gamma_g2_neg: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>delta_g2_neg: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>gamma_abc_g1: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G1&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_groth16_Proof"></a>

## Struct `Proof`

A Groth16 proof.


<pre><code><b>struct</b> <a href="gorth16.md#0x1_groth16_Proof">Proof</a>&lt;G1, G2, Gt&gt; <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>a: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G1&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>b: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>c: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G1&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_groth16_new_vk"></a>

## Function `new_vk`

Create a new Groth16 verifying key.


<pre><code><b>public</b> <b>fun</b> <a href="gorth16.md#0x1_groth16_new_vk">new_vk</a>&lt;G1, G2, Gt&gt;(alpha_g1: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G1&gt;, beta_g2: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;, gamma_g2: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;, delta_g2: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;, gamma_abc_g1: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G1&gt;&gt;): <a href="gorth16.md#0x1_groth16_VerifyingKey">groth16::VerifyingKey</a>&lt;G1, G2, Gt&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="gorth16.md#0x1_groth16_new_vk">new_vk</a>&lt;G1,G2,Gt&gt;(alpha_g1: Element&lt;G1&gt;, beta_g2: Element&lt;G2&gt;, gamma_g2: Element&lt;G2&gt;, delta_g2: Element&lt;G2&gt;, gamma_abc_g1: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Element&lt;G1&gt;&gt;): <a href="gorth16.md#0x1_groth16_VerifyingKey">VerifyingKey</a>&lt;G1,G2,Gt&gt; {
    <a href="gorth16.md#0x1_groth16_VerifyingKey">VerifyingKey</a> {
        alpha_g1,
        beta_g2,
        gamma_g2,
        delta_g2,
        gamma_abc_g1,
    }
}
</code></pre>



</details>

<a name="0x1_groth16_new_pvk"></a>

## Function `new_pvk`

Create a new pre-processed Groth16 verifying key.


<pre><code><b>public</b> <b>fun</b> <a href="gorth16.md#0x1_groth16_new_pvk">new_pvk</a>&lt;G1, G2, Gt&gt;(alpha_g1_beta_g2: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;Gt&gt;, gamma_g2_neg: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;, delta_g2_neg: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;, gamma_abc_g1: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G1&gt;&gt;): <a href="gorth16.md#0x1_groth16_PreparedVerifyingKey">groth16::PreparedVerifyingKey</a>&lt;G1, G2, Gt&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="gorth16.md#0x1_groth16_new_pvk">new_pvk</a>&lt;G1,G2,Gt&gt;(alpha_g1_beta_g2: Element&lt;Gt&gt;, gamma_g2_neg: Element&lt;G2&gt;, delta_g2_neg: Element&lt;G2&gt;, gamma_abc_g1: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Element&lt;G1&gt;&gt;): <a href="gorth16.md#0x1_groth16_PreparedVerifyingKey">PreparedVerifyingKey</a>&lt;G1,G2,Gt&gt; {
    <a href="gorth16.md#0x1_groth16_PreparedVerifyingKey">PreparedVerifyingKey</a> {
        alpha_g1_beta_g2,
        gamma_g2_neg,
        delta_g2_neg,
        gamma_abc_g1,
    }
}
</code></pre>



</details>

<a name="0x1_groth16_prepare_verifying_key"></a>

## Function `prepare_verifying_key`

Pre-process a Groth16 verification key <code>vk</code> for faster verification.


<pre><code><b>public</b> <b>fun</b> <a href="gorth16.md#0x1_groth16_prepare_verifying_key">prepare_verifying_key</a>&lt;G1, G2, Gt&gt;(vk: &<a href="gorth16.md#0x1_groth16_VerifyingKey">groth16::VerifyingKey</a>&lt;G1, G2, Gt&gt;): <a href="gorth16.md#0x1_groth16_PreparedVerifyingKey">groth16::PreparedVerifyingKey</a>&lt;G1, G2, Gt&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="gorth16.md#0x1_groth16_prepare_verifying_key">prepare_verifying_key</a>&lt;G1,G2,Gt&gt;(vk: &<a href="gorth16.md#0x1_groth16_VerifyingKey">VerifyingKey</a>&lt;G1,G2,Gt&gt;): <a href="gorth16.md#0x1_groth16_PreparedVerifyingKey">PreparedVerifyingKey</a>&lt;G1,G2,Gt&gt; {
    <a href="gorth16.md#0x1_groth16_PreparedVerifyingKey">PreparedVerifyingKey</a> {
        alpha_g1_beta_g2: pairing&lt;G1,G2,Gt&gt;(&vk.alpha_g1, &vk.beta_g2),
        gamma_g2_neg: neg(&vk.gamma_g2),
        delta_g2_neg: neg(&vk.delta_g2),
        gamma_abc_g1: vk.gamma_abc_g1,
    }
}
</code></pre>



</details>

<a name="0x1_groth16_new_proof"></a>

## Function `new_proof`

Create a Groth16 proof.


<pre><code><b>public</b> <b>fun</b> <a href="gorth16.md#0x1_groth16_new_proof">new_proof</a>&lt;G1, G2, Gt&gt;(a: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G1&gt;, b: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G2&gt;, c: <a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;G1&gt;): <a href="gorth16.md#0x1_groth16_Proof">groth16::Proof</a>&lt;G1, G2, Gt&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="gorth16.md#0x1_groth16_new_proof">new_proof</a>&lt;G1,G2,Gt&gt;(a: Element&lt;G1&gt;, b: Element&lt;G2&gt;, c: Element&lt;G1&gt;): <a href="gorth16.md#0x1_groth16_Proof">Proof</a>&lt;G1,G2,Gt&gt; {
    <a href="gorth16.md#0x1_groth16_Proof">Proof</a> { a, b, c }
}
</code></pre>



</details>

<a name="0x1_groth16_verify_proof"></a>

## Function `verify_proof`

Verify a Groth16 proof.


<pre><code><b>public</b> <b>fun</b> <a href="gorth16.md#0x1_groth16_verify_proof">verify_proof</a>&lt;G1, G2, Gt, S&gt;(vk: &<a href="gorth16.md#0x1_groth16_VerifyingKey">groth16::VerifyingKey</a>&lt;G1, G2, Gt&gt;, public_inputs: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;&gt;, proof: &<a href="gorth16.md#0x1_groth16_Proof">groth16::Proof</a>&lt;G1, G2, Gt&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="gorth16.md#0x1_groth16_verify_proof">verify_proof</a>&lt;G1,G2,Gt,S&gt;(vk: &<a href="gorth16.md#0x1_groth16_VerifyingKey">VerifyingKey</a>&lt;G1,G2,Gt&gt;, public_inputs: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Element&lt;S&gt;&gt;, proof: &<a href="gorth16.md#0x1_groth16_Proof">Proof</a>&lt;G1,G2,Gt&gt;): bool {
    <b>let</b> left = pairing&lt;G1,G2,Gt&gt;(&proof.a, &proof.b);
    <b>let</b> right_1 = pairing&lt;G1,G2,Gt&gt;(&vk.alpha_g1, &vk.beta_g2);
    <b>let</b> scalars = <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[from_u64&lt;S&gt;(1)];
    std::vector::append(&<b>mut</b> scalars, *public_inputs);
    <b>let</b> right_2 = pairing(&multi_scalar_mul(&vk.gamma_abc_g1, &scalars), &vk.gamma_g2);
    <b>let</b> right_3 = pairing(&proof.c, &vk.delta_g2);
    <b>let</b> right = add(&add(&right_1, &right_2), &right_3);
    eq(&left, &right)
}
</code></pre>



</details>

<a name="0x1_groth16_verify_proof_with_pvk"></a>

## Function `verify_proof_with_pvk`

Verify a Groth16 proof <code>proof</code> against the public inputs <code>public_inputs</code> with a prepared verification key <code>pvk</code>.


<pre><code><b>public</b> <b>fun</b> <a href="gorth16.md#0x1_groth16_verify_proof_with_pvk">verify_proof_with_pvk</a>&lt;G1, G2, Gt, S&gt;(pvk: &<a href="gorth16.md#0x1_groth16_PreparedVerifyingKey">groth16::PreparedVerifyingKey</a>&lt;G1, G2, Gt&gt;, public_inputs: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="algebra.md#0x1_algebra_Element">algebra::Element</a>&lt;S&gt;&gt;, proof: &<a href="gorth16.md#0x1_groth16_Proof">groth16::Proof</a>&lt;G1, G2, Gt&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="gorth16.md#0x1_groth16_verify_proof_with_pvk">verify_proof_with_pvk</a>&lt;G1,G2,Gt,S&gt;(pvk: &<a href="gorth16.md#0x1_groth16_PreparedVerifyingKey">PreparedVerifyingKey</a>&lt;G1,G2,Gt&gt;, public_inputs: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Element&lt;S&gt;&gt;, proof: &<a href="gorth16.md#0x1_groth16_Proof">Proof</a>&lt;G1,G2,Gt&gt;): bool {
    <b>let</b> scalars = <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[from_u64&lt;S&gt;(1)];
    std::vector::append(&<b>mut</b> scalars, *public_inputs);
    <b>let</b> g1_elements: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Element&lt;G1&gt;&gt; = <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[proof.a, multi_scalar_mul(&pvk.gamma_abc_g1, &scalars), proof.c];
    <b>let</b> g2_elements: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Element&lt;G2&gt;&gt; = <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[proof.b, pvk.gamma_g2_neg, pvk.delta_g2_neg];

    eq(&pvk.alpha_g1_beta_g2, &multi_pairing&lt;G1,G2,Gt&gt;(&g1_elements, &g2_elements))
}
</code></pre>



</details>


[move-book]: https://aptos.dev/guides/move-guides/book/SUMMARY
