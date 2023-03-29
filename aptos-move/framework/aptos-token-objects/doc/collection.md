
<a name="0x4_collection"></a>

# Module `0x4::collection`

This defines an object-based Collection. A collection acts as a set organizer for a group of
tokens. This includes aspects such as a general description, project URI, name, and may contain
other useful generalizations across this set of tokens.

Being built upon objects enables collections to be relatively flexible. As core primitives it
supports:
* Common fields: name, uri, description, creator
* MutatorRef leaving mutability configuration to a higher level component
* Addressed by a global identifier of creator's address and collection name, thus collections
cannot be deleted as a restriction of the object model.
* Optional support for collection-wide royalties
* Optional support for tracking of supply

This collection does not directly support:
* Events on mint or burn -- that's left to the collection creator.

TODO:
* Consider supporting changing the name of the collection with the MutatorRef. This would
require adding the field original_name.
* Consider supporting changing the aspects of supply with the MutatorRef.
* Add aggregator support when added to framework
* Update Object<T> to be viable input as a transaction arg and then update all readers as view.


-  [Resource `Collection`](#0x4_collection_Collection)
-  [Struct `MutatorRef`](#0x4_collection_MutatorRef)
-  [Struct `MutationEvent`](#0x4_collection_MutationEvent)
-  [Resource `FixedSupply`](#0x4_collection_FixedSupply)
-  [Constants](#@Constants_0)
-  [Function `create_fixed_collection`](#0x4_collection_create_fixed_collection)
-  [Function `create_untracked_collection`](#0x4_collection_create_untracked_collection)
-  [Function `create_collection_address`](#0x4_collection_create_collection_address)
-  [Function `create_collection_seed`](#0x4_collection_create_collection_seed)
-  [Function `increment_supply`](#0x4_collection_increment_supply)
-  [Function `decrement_supply`](#0x4_collection_decrement_supply)
-  [Function `generate_mutator_ref`](#0x4_collection_generate_mutator_ref)
-  [Function `count`](#0x4_collection_count)
-  [Function `creator`](#0x4_collection_creator)
-  [Function `description`](#0x4_collection_description)
-  [Function `name`](#0x4_collection_name)
-  [Function `uri`](#0x4_collection_uri)
-  [Function `set_description`](#0x4_collection_set_description)
-  [Function `set_uri`](#0x4_collection_set_uri)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-framework/doc/event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-framework/doc/object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="royalty.md#0x4_royalty">0x4::royalty</a>;
</code></pre>



<a name="0x4_collection_Collection"></a>

## Resource `Collection`

Represents the common fields for a collection.


<pre><code><b>struct</b> <a href="collection.md#0x4_collection_Collection">Collection</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creator: <b>address</b></code>
</dt>
<dd>
 The creator of this collection.
</dd>
<dt>
<code>description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 A brief description of the collection.
</dd>
<dt>
<code>name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 An optional categorization of similar token.
</dd>
<dt>
<code>uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 The Uniform Resource Identifier (uri) pointing to the JSON file stored in off-chain
 storage; the URL length will likely need a maximum any suggestions?
</dd>
<dt>
<code>mutation_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="collection.md#0x4_collection_MutationEvent">collection::MutationEvent</a>&gt;</code>
</dt>
<dd>
 Emitted upon any mutation of the collection.
</dd>
</dl>


</details>

<a name="0x4_collection_MutatorRef"></a>

## Struct `MutatorRef`

This enables mutating description and URI by higher level services.


<pre><code><b>struct</b> <a href="collection.md#0x4_collection_MutatorRef">MutatorRef</a> <b>has</b> drop, store
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

<a name="0x4_collection_MutationEvent"></a>

## Struct `MutationEvent`

Contains the mutated fields name. This makes the life of indexers easier, so that they can
directly understand the behavior in a writeset.


<pre><code><b>struct</b> <a href="collection.md#0x4_collection_MutationEvent">MutationEvent</a> <b>has</b> drop, store
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

<a name="0x4_collection_FixedSupply"></a>

## Resource `FixedSupply`

Fixed supply tracker, this is useful for ensuring that a limited number of tokens are minted.


<pre><code><b>struct</b> <a href="collection.md#0x4_collection_FixedSupply">FixedSupply</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>current_supply: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>max_supply: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>total_minted: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x4_collection_ECOLLECTION_DOES_NOT_EXIST"></a>

The collection does not exist


<pre><code><b>const</b> <a href="collection.md#0x4_collection_ECOLLECTION_DOES_NOT_EXIST">ECOLLECTION_DOES_NOT_EXIST</a>: u64 = 2;
</code></pre>



<a name="0x4_collection_EEXCEEDS_MAX_SUPPLY"></a>

The collections supply is at its maximum amount


<pre><code><b>const</b> <a href="collection.md#0x4_collection_EEXCEEDS_MAX_SUPPLY">EEXCEEDS_MAX_SUPPLY</a>: u64 = 1;
</code></pre>



<a name="0x4_collection_create_fixed_collection"></a>

## Function `create_fixed_collection`

Creates a fixed-sized collection, or a collection that supports a fixed amount of tokens.
This is useful to create a guaranteed, limited supply on-chain digital asset. For example,
a collection 1111 vicious vipers. Note, creating restrictions such as upward limits results
in data structures that prevent Aptos from parallelizing mints of this collection type.


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_create_fixed_collection">create_fixed_collection</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, max_supply: u64, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="royalty.md#0x4_royalty">royalty</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="royalty.md#0x4_royalty_Royalty">royalty::Royalty</a>&gt;, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/doc/object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_create_fixed_collection">create_fixed_collection</a>(
    creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    description: String,
    max_supply: u64,
    name: String,
    <a href="royalty.md#0x4_royalty">royalty</a>: Option&lt;Royalty&gt;,
    uri: String,
): ConstructorRef {
    <b>let</b> supply = <a href="collection.md#0x4_collection_FixedSupply">FixedSupply</a> {
        current_supply: 0,
        max_supply,
        total_minted: 0,
    };

    create_collection_internal(
        creator,
        description,
        name,
        <a href="royalty.md#0x4_royalty">royalty</a>,
        uri,
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(supply),
    )
}
</code></pre>



</details>

<a name="0x4_collection_create_untracked_collection"></a>

## Function `create_untracked_collection`

Creates an untracked collection, or a collection that supports an arbitrary amount of
tokens. This is useful for mass airdrops that fully leverage Aptos parallelization.


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_create_untracked_collection">create_untracked_collection</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="royalty.md#0x4_royalty">royalty</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="royalty.md#0x4_royalty_Royalty">royalty::Royalty</a>&gt;, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/doc/object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_create_untracked_collection">create_untracked_collection</a>(
    creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    description: String,
    name: String,
    <a href="royalty.md#0x4_royalty">royalty</a>: Option&lt;Royalty&gt;,
    uri: String,
): ConstructorRef {
    create_collection_internal&lt;<a href="collection.md#0x4_collection_FixedSupply">FixedSupply</a>&gt;(
        creator,
        description,
        name,
        <a href="royalty.md#0x4_royalty">royalty</a>,
        uri,
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
    )
}
</code></pre>



</details>

<a name="0x4_collection_create_collection_address"></a>

## Function `create_collection_address`

Generates the collections address based upon the creators address and the collection's name


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_create_collection_address">create_collection_address</a>(creator: &<b>address</b>, name: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_create_collection_address">create_collection_address</a>(creator: &<b>address</b>, name: &String): <b>address</b> {
    <a href="../../aptos-framework/doc/object.md#0x1_object_create_object_address">object::create_object_address</a>(creator, <a href="collection.md#0x4_collection_create_collection_seed">create_collection_seed</a>(name))
}
</code></pre>



</details>

<a name="0x4_collection_create_collection_seed"></a>

## Function `create_collection_seed`

Named objects are derived from a seed, the collection's seed is its name.


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_create_collection_seed">create_collection_seed</a>(name: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_create_collection_seed">create_collection_seed</a>(name: &String): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    *<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_bytes">string::bytes</a>(name)
}
</code></pre>



</details>

<a name="0x4_collection_increment_supply"></a>

## Function `increment_supply`

Called by token on mint to increment supply if there's an appropriate Supply struct.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="collection.md#0x4_collection_increment_supply">increment_supply</a>(<a href="collection.md#0x4_collection">collection</a>: &<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="collection.md#0x4_collection_Collection">collection::Collection</a>&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="collection.md#0x4_collection_increment_supply">increment_supply</a>(
    <a href="collection.md#0x4_collection">collection</a>: &Object&lt;<a href="collection.md#0x4_collection_Collection">Collection</a>&gt;,
): Option&lt;u64&gt; <b>acquires</b> <a href="collection.md#0x4_collection_FixedSupply">FixedSupply</a> {
    <b>let</b> collection_addr = <a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(<a href="collection.md#0x4_collection">collection</a>);
    <b>if</b> (<b>exists</b>&lt;<a href="collection.md#0x4_collection_FixedSupply">FixedSupply</a>&gt;(collection_addr)) {
        <b>let</b> supply = <b>borrow_global_mut</b>&lt;<a href="collection.md#0x4_collection_FixedSupply">FixedSupply</a>&gt;(collection_addr);
        supply.current_supply = supply.current_supply + 1;
        supply.total_minted = supply.total_minted + 1;
        <b>assert</b>!(
            supply.current_supply &lt;= supply.max_supply,
            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="collection.md#0x4_collection_EEXCEEDS_MAX_SUPPLY">EEXCEEDS_MAX_SUPPLY</a>),
        );
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(supply.total_minted)
    } <b>else</b> {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a name="0x4_collection_decrement_supply"></a>

## Function `decrement_supply`

Called by token on burn to decrement supply if there's an appropriate Supply struct.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="collection.md#0x4_collection_decrement_supply">decrement_supply</a>(<a href="collection.md#0x4_collection">collection</a>: &<a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="collection.md#0x4_collection_Collection">collection::Collection</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="collection.md#0x4_collection_decrement_supply">decrement_supply</a>(<a href="collection.md#0x4_collection">collection</a>: &Object&lt;<a href="collection.md#0x4_collection_Collection">Collection</a>&gt;) <b>acquires</b> <a href="collection.md#0x4_collection_FixedSupply">FixedSupply</a> {
    <b>let</b> collection_addr = <a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(<a href="collection.md#0x4_collection">collection</a>);
    <b>if</b> (<b>exists</b>&lt;<a href="collection.md#0x4_collection_FixedSupply">FixedSupply</a>&gt;(collection_addr)) {
        <b>let</b> supply = <b>borrow_global_mut</b>&lt;<a href="collection.md#0x4_collection_FixedSupply">FixedSupply</a>&gt;(collection_addr);
        supply.current_supply = supply.current_supply - 1;
    }
}
</code></pre>



</details>

<a name="0x4_collection_generate_mutator_ref"></a>

## Function `generate_mutator_ref`

Creates a MutatorRef, which gates the ability to mutate any fields that support mutation.


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_generate_mutator_ref">generate_mutator_ref</a>(ref: &<a href="../../aptos-framework/doc/object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>): <a href="collection.md#0x4_collection_MutatorRef">collection::MutatorRef</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_generate_mutator_ref">generate_mutator_ref</a>(ref: &ConstructorRef): <a href="collection.md#0x4_collection_MutatorRef">MutatorRef</a> {
    <b>let</b> <a href="../../aptos-framework/doc/object.md#0x1_object">object</a> = <a href="../../aptos-framework/doc/object.md#0x1_object_object_from_constructor_ref">object::object_from_constructor_ref</a>&lt;<a href="collection.md#0x4_collection_Collection">Collection</a>&gt;(ref);
    <a href="collection.md#0x4_collection_MutatorRef">MutatorRef</a> { self: <a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(&<a href="../../aptos-framework/doc/object.md#0x1_object">object</a>) }
}
</code></pre>



</details>

<a name="0x4_collection_count"></a>

## Function `count`



<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_count">count</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_count">count</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: Object&lt;T&gt;): Option&lt;u64&gt; <b>acquires</b> <a href="collection.md#0x4_collection_FixedSupply">FixedSupply</a> {
    <b>let</b> collection_address = <a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(&<a href="collection.md#0x4_collection">collection</a>);
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="collection.md#0x4_collection_Collection">Collection</a>&gt;(collection_address),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="collection.md#0x4_collection_ECOLLECTION_DOES_NOT_EXIST">ECOLLECTION_DOES_NOT_EXIST</a>),
    );

    <b>if</b> (<b>exists</b>&lt;<a href="collection.md#0x4_collection_FixedSupply">FixedSupply</a>&gt;(collection_address)) {
        <b>let</b> supply = <b>borrow_global_mut</b>&lt;<a href="collection.md#0x4_collection_FixedSupply">FixedSupply</a>&gt;(collection_address);
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(supply.current_supply)
    } <b>else</b> {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a name="0x4_collection_creator"></a>

## Function `creator`



<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_creator">creator</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_creator">creator</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: Object&lt;T&gt;): <b>address</b> <b>acquires</b> <a href="collection.md#0x4_collection_Collection">Collection</a> {
    borrow(&<a href="collection.md#0x4_collection">collection</a>).creator
}
</code></pre>



</details>

<a name="0x4_collection_description"></a>

## Function `description`



<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_description">description</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_description">description</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: Object&lt;T&gt;): String <b>acquires</b> <a href="collection.md#0x4_collection_Collection">Collection</a> {
    borrow(&<a href="collection.md#0x4_collection">collection</a>).description
}
</code></pre>



</details>

<a name="0x4_collection_name"></a>

## Function `name`



<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_name">name</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_name">name</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: Object&lt;T&gt;): String <b>acquires</b> <a href="collection.md#0x4_collection_Collection">Collection</a> {
    borrow(&<a href="collection.md#0x4_collection">collection</a>).name
}
</code></pre>



</details>

<a name="0x4_collection_uri"></a>

## Function `uri`



<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_uri">uri</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_uri">uri</a>&lt;T: key&gt;(<a href="collection.md#0x4_collection">collection</a>: Object&lt;T&gt;): String <b>acquires</b> <a href="collection.md#0x4_collection_Collection">Collection</a> {
    borrow(&<a href="collection.md#0x4_collection">collection</a>).uri
}
</code></pre>



</details>

<a name="0x4_collection_set_description"></a>

## Function `set_description`



<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_set_description">set_description</a>(mutator_ref: &<a href="collection.md#0x4_collection_MutatorRef">collection::MutatorRef</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_set_description">set_description</a>(mutator_ref: &<a href="collection.md#0x4_collection_MutatorRef">MutatorRef</a>, description: String) <b>acquires</b> <a href="collection.md#0x4_collection_Collection">Collection</a> {
    <b>let</b> <a href="collection.md#0x4_collection">collection</a> = borrow_mut(mutator_ref);
    <a href="collection.md#0x4_collection">collection</a>.description = description;
    <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>(
        &<b>mut</b> <a href="collection.md#0x4_collection">collection</a>.mutation_events,
        <a href="collection.md#0x4_collection_MutationEvent">MutationEvent</a> { mutated_field_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"description") },
    );
}
</code></pre>



</details>

<a name="0x4_collection_set_uri"></a>

## Function `set_uri`



<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_set_uri">set_uri</a>(mutator_ref: &<a href="collection.md#0x4_collection_MutatorRef">collection::MutatorRef</a>, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="collection.md#0x4_collection_set_uri">set_uri</a>(mutator_ref: &<a href="collection.md#0x4_collection_MutatorRef">MutatorRef</a>, uri: String) <b>acquires</b> <a href="collection.md#0x4_collection_Collection">Collection</a> {
    <b>let</b> <a href="collection.md#0x4_collection">collection</a> = borrow_mut(mutator_ref);
    <a href="collection.md#0x4_collection">collection</a>.uri = uri;
    <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>(
        &<b>mut</b> <a href="collection.md#0x4_collection">collection</a>.mutation_events,
        <a href="collection.md#0x4_collection_MutationEvent">MutationEvent</a> { mutated_field_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"uri") },
    );
}
</code></pre>



</details>


[move-book]: https://aptos.dev/guides/move-guides/book/SUMMARY
