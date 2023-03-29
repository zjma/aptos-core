
<a name="0x4_token"></a>

# Module `0x4::token`

This defines an object-based Token. The key differentiating features from the Aptos standard
token are:
* Decouple token ownership from token data.
* Explicit data model for token metadata via adjacent resources
* Extensible framework for tokens

TODO:
* Update Object<T> to be a viable input as a transaction arg and then update all readers as view.


-  [Resource `Token`](#0x4_token_Token)
-  [Struct `BurnRef`](#0x4_token_BurnRef)
-  [Struct `MutatorRef`](#0x4_token_MutatorRef)
-  [Struct `MutationEvent`](#0x4_token_MutationEvent)
-  [Constants](#@Constants_0)
-  [Function `create`](#0x4_token_create)
-  [Function `create_token_address`](#0x4_token_create_token_address)
-  [Function `create_token_seed`](#0x4_token_create_token_seed)
-  [Function `generate_mutator_ref`](#0x4_token_generate_mutator_ref)
-  [Function `generate_burn_ref`](#0x4_token_generate_burn_ref)
-  [Function `address_from_burn_ref`](#0x4_token_address_from_burn_ref)
-  [Function `creator`](#0x4_token_creator)
-  [Function `collection`](#0x4_token_collection)
-  [Function `collection_object`](#0x4_token_collection_object)
-  [Function `creation_name`](#0x4_token_creation_name)
-  [Function `description`](#0x4_token_description)
-  [Function `name`](#0x4_token_name)
-  [Function `uri`](#0x4_token_uri)
-  [Function `royalty`](#0x4_token_royalty)
-  [Function `burn`](#0x4_token_burn)
-  [Function `set_description`](#0x4_token_set_description)
-  [Function `set_name`](#0x4_token_set_name)
-  [Function `set_uri`](#0x4_token_set_uri)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-framework/doc/event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-framework/doc/object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
<b>use</b> <a href="collection.md#0x4_collection">0x4::collection</a>;
<b>use</b> <a href="royalty.md#0x4_royalty">0x4::royalty</a>;
</code></pre>



<a name="0x4_token_Token"></a>

## Resource `Token`

Represents the common fields to all tokens.


<pre><code><b>struct</b> <a href="token.md#0x4_token_Token">Token</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="collection.md#0x4_collection_Collection">collection::Collection</a>&gt;</code>
</dt>
<dd>
 The collection from which this token resides.
</dd>
<dt>
<code>collection_id: u64</code>
</dt>
<dd>
 Unique identifier within the collection, optional, 0 means unassigned
</dd>
<dt>
<code>description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 A brief description of the token.
</dd>
<dt>
<code>name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 The name of the token, which should be unique within the collection; the length of name
 should be smaller than 128, characters, eg: "Aptos Animal #1234"
</dd>
<dt>
<code>creation_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;</code>
</dt>
<dd>
 The creation name of the token. Since tokens are created with the name as part of the
 object id generation.
</dd>
<dt>
<code>uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 The Uniform Resource Identifier (uri) pointing to the JSON file stored in off-chain
 storage; the URL length will likely need a maximum any suggestions?
</dd>
<dt>
<code>mutation_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="token.md#0x4_token_MutationEvent">token::MutationEvent</a>&gt;</code>
</dt>
<dd>
 Emitted upon any mutation of the token.
</dd>
</dl>


</details>

<a name="0x4_token_BurnRef"></a>

## Struct `BurnRef`

This enables burning an NFT, if possible, it will also delete the object. Note, the data
in inner and self occupies 32-bytes each, rather than have both, this data structure makes
a small optimization to support either and take a fixed amount of 34-bytes.


<pre><code><b>struct</b> <a href="token.md#0x4_token_BurnRef">BurnRef</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>inner: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/doc/object.md#0x1_object_DeleteRef">object::DeleteRef</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>self: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<b>address</b>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x4_token_MutatorRef"></a>

## Struct `MutatorRef`

This enables mutating descritpion and URI by higher level services.


<pre><code><b>struct</b> <a href="token.md#0x4_token_MutatorRef">MutatorRef</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>self: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x4_token_MutationEvent"></a>

## Struct `MutationEvent`

Contains the mutated fields name. This makes the life of indexers easier, so that they can
directly understand the behavior in a writeset.


<pre><code><b>struct</b> <a href="token.md#0x4_token_MutationEvent">MutationEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>mutated_field_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x4_token_EFIELD_NOT_MUTABLE"></a>

Attempted to mutate an immutable field


<pre><code><b>const</b> <a href="token.md#0x4_token_EFIELD_NOT_MUTABLE">EFIELD_NOT_MUTABLE</a>: u64 = 3;
</code></pre>



<a name="0x4_token_ENOT_CREATOR"></a>

The provided signer is not the creator


<pre><code><b>const</b> <a href="token.md#0x4_token_ENOT_CREATOR">ENOT_CREATOR</a>: u64 = 2;
</code></pre>



<a name="0x4_token_ETOKEN_DOES_NOT_EXIST"></a>



<pre><code><b>const</b> <a href="token.md#0x4_token_ETOKEN_DOES_NOT_EXIST">ETOKEN_DOES_NOT_EXIST</a>: u64 = 1;
</code></pre>



<a name="0x4_token_create"></a>

## Function `create`

Creates a new token object and returns the ConstructorRef for additional specialization.


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_create">create</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="royalty.md#0x4_royalty">royalty</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="royalty.md#0x4_royalty_Royalty">royalty::Royalty</a>&gt;, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/doc/object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_create">create</a>(
    creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    collection_name: String,
    description: String,
    name: String,
    <a href="royalty.md#0x4_royalty">royalty</a>: Option&lt;Royalty&gt;,
    uri: String,
): ConstructorRef {
    <b>let</b> creator_address = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);
    <b>let</b> seed = <a href="token.md#0x4_token_create_token_seed">create_token_seed</a>(&collection_name, &name);

    <b>let</b> collection_addr = <a href="collection.md#0x4_collection_create_collection_address">collection::create_collection_address</a>(&creator_address, &collection_name);
    <b>let</b> <a href="collection.md#0x4_collection">collection</a> = <a href="../../aptos-framework/doc/object.md#0x1_object_address_to_object">object::address_to_object</a>&lt;Collection&gt;(collection_addr);
    <b>let</b> id = <a href="collection.md#0x4_collection_increment_supply">collection::increment_supply</a>(&<a href="collection.md#0x4_collection">collection</a>);

    <b>let</b> constructor_ref = <a href="../../aptos-framework/doc/object.md#0x1_object_create_named_object">object::create_named_object</a>(creator, seed);
    <b>let</b> object_signer = <a href="../../aptos-framework/doc/object.md#0x1_object_generate_signer">object::generate_signer</a>(&constructor_ref);

    <b>let</b> <a href="token.md#0x4_token">token</a> = <a href="token.md#0x4_token_Token">Token</a> {
        <a href="collection.md#0x4_collection">collection</a>,
        collection_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_get_with_default">option::get_with_default</a>(&<b>mut</b> id, 0),
        description,
        name,
        creation_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
        uri,
        mutation_events: <a href="../../aptos-framework/doc/object.md#0x1_object_new_event_handle">object::new_event_handle</a>(&object_signer),
    };
    <b>move_to</b>(&object_signer, <a href="token.md#0x4_token">token</a>);

    <b>if</b> (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&<a href="royalty.md#0x4_royalty">royalty</a>)) {
        <a href="royalty.md#0x4_royalty_init">royalty::init</a>(&constructor_ref, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&<b>mut</b> <a href="royalty.md#0x4_royalty">royalty</a>))
    };
    constructor_ref
}
</code></pre>



</details>

<a name="0x4_token_create_token_address"></a>

## Function `create_token_address`

Generates the collections address based upon the creators address and the collection's name


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_create_token_address">create_token_address</a>(creator: &<b>address</b>, <a href="collection.md#0x4_collection">collection</a>: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_create_token_address">create_token_address</a>(creator: &<b>address</b>, <a href="collection.md#0x4_collection">collection</a>: &String, name: &String): <b>address</b> {
    <a href="../../aptos-framework/doc/object.md#0x1_object_create_object_address">object::create_object_address</a>(creator, <a href="token.md#0x4_token_create_token_seed">create_token_seed</a>(<a href="collection.md#0x4_collection">collection</a>, name))
}
</code></pre>



</details>

<a name="0x4_token_create_token_seed"></a>

## Function `create_token_seed`

Named objects are derived from a seed, the collection's seed is its name.


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_create_token_seed">create_token_seed</a>(<a href="collection.md#0x4_collection">collection</a>: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_create_token_seed">create_token_seed</a>(<a href="collection.md#0x4_collection">collection</a>: &String, name: &String): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> seed = *<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_bytes">string::bytes</a>(<a href="collection.md#0x4_collection">collection</a>);
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> seed, b"::");
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> seed, *<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_bytes">string::bytes</a>(name));
    seed
}
</code></pre>



</details>

<a name="0x4_token_generate_mutator_ref"></a>

## Function `generate_mutator_ref`

Creates a MutatorRef, which gates the ability to mutate any fields that support mutation.


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_generate_mutator_ref">generate_mutator_ref</a>(ref: &<a href="../../aptos-framework/doc/object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>): <a href="token.md#0x4_token_MutatorRef">token::MutatorRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_generate_mutator_ref">generate_mutator_ref</a>(ref: &ConstructorRef): <a href="token.md#0x4_token_MutatorRef">MutatorRef</a> {
    <b>let</b> <a href="../../aptos-framework/doc/object.md#0x1_object">object</a> = <a href="../../aptos-framework/doc/object.md#0x1_object_object_from_constructor_ref">object::object_from_constructor_ref</a>&lt;<a href="token.md#0x4_token_Token">Token</a>&gt;(ref);
    <a href="token.md#0x4_token_MutatorRef">MutatorRef</a> { self: <a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(&<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>) }
}
</code></pre>



</details>

<a name="0x4_token_generate_burn_ref"></a>

## Function `generate_burn_ref`

Creates a BurnRef, which gates the ability to burn the given token.


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_generate_burn_ref">generate_burn_ref</a>(ref: &<a href="../../aptos-framework/doc/object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>): <a href="token.md#0x4_token_BurnRef">token::BurnRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_generate_burn_ref">generate_burn_ref</a>(ref: &ConstructorRef): <a href="token.md#0x4_token_BurnRef">BurnRef</a> {
    <b>let</b> (inner, self) = <b>if</b> (<a href="../../aptos-framework/doc/object.md#0x1_object_can_generate_delete_ref">object::can_generate_delete_ref</a>(ref)) {
        <b>let</b> delete_ref = <a href="../../aptos-framework/doc/object.md#0x1_object_generate_delete_ref">object::generate_delete_ref</a>(ref);
        (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(delete_ref), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>())
    } <b>else</b> {
        <b>let</b> addr = <a href="../../aptos-framework/doc/object.md#0x1_object_address_from_constructor_ref">object::address_from_constructor_ref</a>(ref);
        (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(addr))
    };
    <a href="token.md#0x4_token_BurnRef">BurnRef</a> { self, inner }
}
</code></pre>



</details>

<a name="0x4_token_address_from_burn_ref"></a>

## Function `address_from_burn_ref`

Extracts the tokens address from a BurnRef.


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_address_from_burn_ref">address_from_burn_ref</a>(ref: &<a href="token.md#0x4_token_BurnRef">token::BurnRef</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_address_from_burn_ref">address_from_burn_ref</a>(ref: &<a href="token.md#0x4_token_BurnRef">BurnRef</a>): <b>address</b> {
    <b>if</b> (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&ref.inner)) {
        <a href="../../aptos-framework/doc/object.md#0x1_object_address_from_delete_ref">object::address_from_delete_ref</a>(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&ref.inner))
    } <b>else</b> {
        *<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&ref.self)
    }
}
</code></pre>



</details>

<a name="0x4_token_creator"></a>

## Function `creator`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_creator">creator</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_creator">creator</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: Object&lt;T&gt;): <b>address</b> <b>acquires</b> <a href="token.md#0x4_token_Token">Token</a> {
    <a href="collection.md#0x4_collection_creator">collection::creator</a>(borrow(&<a href="token.md#0x4_token">token</a>).<a href="collection.md#0x4_collection">collection</a>)
}
</code></pre>



</details>

<a name="0x4_token_collection"></a>

## Function `collection`



<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection">collection</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection">collection</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: Object&lt;T&gt;): String <b>acquires</b> <a href="token.md#0x4_token_Token">Token</a> {
    <a href="collection.md#0x4_collection_name">collection::name</a>(borrow(&<a href="token.md#0x4_token">token</a>).<a href="collection.md#0x4_collection">collection</a>)
}
</code></pre>



</details>

<a name="0x4_token_collection_object"></a>

## Function `collection_object`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_collection_object">collection_object</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="collection.md#0x4_collection_Collection">collection::Collection</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_collection_object">collection_object</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: Object&lt;T&gt;): Object&lt;Collection&gt; <b>acquires</b> <a href="token.md#0x4_token_Token">Token</a> {
    borrow(&<a href="token.md#0x4_token">token</a>).<a href="collection.md#0x4_collection">collection</a>
}
</code></pre>



</details>

<a name="0x4_token_creation_name"></a>

## Function `creation_name`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_creation_name">creation_name</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_creation_name">creation_name</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: Object&lt;T&gt;): String <b>acquires</b> <a href="token.md#0x4_token_Token">Token</a> {
    <b>let</b> <a href="token.md#0x4_token">token</a> = borrow(&<a href="token.md#0x4_token">token</a>);
    <b>if</b> (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&<a href="token.md#0x4_token">token</a>.creation_name)) {
        *<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&<a href="token.md#0x4_token">token</a>.creation_name)
    } <b>else</b> {
        <a href="token.md#0x4_token">token</a>.name
    }
}
</code></pre>



</details>

<a name="0x4_token_description"></a>

## Function `description`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_description">description</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_description">description</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: Object&lt;T&gt;): String <b>acquires</b> <a href="token.md#0x4_token_Token">Token</a> {
    borrow(&<a href="token.md#0x4_token">token</a>).description
}
</code></pre>



</details>

<a name="0x4_token_name"></a>

## Function `name`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_name">name</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_name">name</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: Object&lt;T&gt;): String <b>acquires</b> <a href="token.md#0x4_token_Token">Token</a> {
    borrow(&<a href="token.md#0x4_token">token</a>).name
}
</code></pre>



</details>

<a name="0x4_token_uri"></a>

## Function `uri`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_uri">uri</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_uri">uri</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: Object&lt;T&gt;): String <b>acquires</b> <a href="token.md#0x4_token_Token">Token</a> {
    borrow(&<a href="token.md#0x4_token">token</a>).uri
}
</code></pre>



</details>

<a name="0x4_token_royalty"></a>

## Function `royalty`



<pre><code><b>public</b> <b>fun</b> <a href="royalty.md#0x4_royalty">royalty</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="royalty.md#0x4_royalty_Royalty">royalty::Royalty</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="royalty.md#0x4_royalty">royalty</a>&lt;T: key&gt;(<a href="token.md#0x4_token">token</a>: Object&lt;T&gt;): Option&lt;Royalty&gt; <b>acquires</b> <a href="token.md#0x4_token_Token">Token</a> {
    borrow(&<a href="token.md#0x4_token">token</a>);
    <b>let</b> <a href="royalty.md#0x4_royalty">royalty</a> = <a href="royalty.md#0x4_royalty_get">royalty::get</a>(<a href="token.md#0x4_token">token</a>);
    <b>if</b> (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&<a href="royalty.md#0x4_royalty">royalty</a>)) {
        <a href="royalty.md#0x4_royalty">royalty</a>
    } <b>else</b> {
        <b>let</b> creator = <a href="token.md#0x4_token_creator">creator</a>(<a href="token.md#0x4_token">token</a>);
        <b>let</b> collection_name = <a href="collection.md#0x4_collection">collection</a>(<a href="token.md#0x4_token">token</a>);
        <b>let</b> collection_address = <a href="collection.md#0x4_collection_create_collection_address">collection::create_collection_address</a>(&creator, &collection_name);
        <b>let</b> <a href="collection.md#0x4_collection">collection</a> = <a href="../../aptos-framework/doc/object.md#0x1_object_address_to_object">object::address_to_object</a>&lt;<a href="collection.md#0x4_collection_Collection">collection::Collection</a>&gt;(collection_address);
        <a href="royalty.md#0x4_royalty_get">royalty::get</a>(<a href="collection.md#0x4_collection">collection</a>)
    }
}
</code></pre>



</details>

<a name="0x4_token_burn"></a>

## Function `burn`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_burn">burn</a>(burn_ref: <a href="token.md#0x4_token_BurnRef">token::BurnRef</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_burn">burn</a>(burn_ref: <a href="token.md#0x4_token_BurnRef">BurnRef</a>) <b>acquires</b> <a href="token.md#0x4_token_Token">Token</a> {
    <b>let</b> addr = <b>if</b> (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&burn_ref.inner)) {
        <b>let</b> delete_ref = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&<b>mut</b> burn_ref.inner);
        <b>let</b> addr = <a href="../../aptos-framework/doc/object.md#0x1_object_address_from_delete_ref">object::address_from_delete_ref</a>(&delete_ref);
        <a href="../../aptos-framework/doc/object.md#0x1_object_delete">object::delete</a>(delete_ref);
        addr
    } <b>else</b> {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&<b>mut</b> burn_ref.self)
    };

    <b>if</b> (<a href="royalty.md#0x4_royalty_exists_at">royalty::exists_at</a>(addr)) {
        <a href="royalty.md#0x4_royalty_delete">royalty::delete</a>(addr)
    };

    <b>let</b> <a href="token.md#0x4_token_Token">Token</a> {
        <a href="collection.md#0x4_collection">collection</a>,
        collection_id: _,
        description: _,
        name: _,
        creation_name: _,
        uri: _,
        mutation_events,
    } = <b>move_from</b>&lt;<a href="token.md#0x4_token_Token">Token</a>&gt;(addr);

    <a href="../../aptos-framework/doc/event.md#0x1_event_destroy_handle">event::destroy_handle</a>(mutation_events);
    <a href="collection.md#0x4_collection_decrement_supply">collection::decrement_supply</a>(&<a href="collection.md#0x4_collection">collection</a>);
}
</code></pre>



</details>

<a name="0x4_token_set_description"></a>

## Function `set_description`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_set_description">set_description</a>(mutator_ref: &<a href="token.md#0x4_token_MutatorRef">token::MutatorRef</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_set_description">set_description</a>(mutator_ref: &<a href="token.md#0x4_token_MutatorRef">MutatorRef</a>, description: String) <b>acquires</b> <a href="token.md#0x4_token_Token">Token</a> {
    <b>let</b> <a href="token.md#0x4_token">token</a> = borrow_mut(mutator_ref);
    <a href="token.md#0x4_token">token</a>.description = description;
    <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>(
        &<b>mut</b> <a href="token.md#0x4_token">token</a>.mutation_events,
        <a href="token.md#0x4_token_MutationEvent">MutationEvent</a> { mutated_field_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"description") },
    );
}
</code></pre>



</details>

<a name="0x4_token_set_name"></a>

## Function `set_name`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_set_name">set_name</a>(mutator_ref: &<a href="token.md#0x4_token_MutatorRef">token::MutatorRef</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_set_name">set_name</a>(mutator_ref: &<a href="token.md#0x4_token_MutatorRef">MutatorRef</a>, name: String) <b>acquires</b> <a href="token.md#0x4_token_Token">Token</a> {
    <b>let</b> <a href="token.md#0x4_token">token</a> = borrow_mut(mutator_ref);
    <b>if</b> (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(&<a href="token.md#0x4_token">token</a>.creation_name)) {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_fill">option::fill</a>(&<b>mut</b> <a href="token.md#0x4_token">token</a>.creation_name, <a href="token.md#0x4_token">token</a>.name)
    };
    <a href="token.md#0x4_token">token</a>.name = name;
    <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>(
        &<b>mut</b> <a href="token.md#0x4_token">token</a>.mutation_events,
        <a href="token.md#0x4_token_MutationEvent">MutationEvent</a> { mutated_field_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"name") },
    );
}
</code></pre>



</details>

<a name="0x4_token_set_uri"></a>

## Function `set_uri`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_set_uri">set_uri</a>(mutator_ref: &<a href="token.md#0x4_token_MutatorRef">token::MutatorRef</a>, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x4_token_set_uri">set_uri</a>(mutator_ref: &<a href="token.md#0x4_token_MutatorRef">MutatorRef</a>, uri: String) <b>acquires</b> <a href="token.md#0x4_token_Token">Token</a> {
    <b>let</b> <a href="token.md#0x4_token">token</a> = borrow_mut(mutator_ref);
    <a href="token.md#0x4_token">token</a>.uri = uri;
    <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>(
        &<b>mut</b> <a href="token.md#0x4_token">token</a>.mutation_events,
        <a href="token.md#0x4_token_MutationEvent">MutationEvent</a> { mutated_field_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"uri") },
    );
}
</code></pre>



</details>


[move-book]: https://aptos.dev/guides/move-guides/book/SUMMARY
