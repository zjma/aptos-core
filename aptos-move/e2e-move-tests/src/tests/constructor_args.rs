// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, tests::common, MoveHarness};
use aptos_types::{
    account_address::AccountAddress,
    on_chain_config::FeatureFlag,
    transaction::{ExecutionStatus, TransactionStatus},
};
use move_core_types::{language_storage::TypeTag, parser::parse_struct_tag, vm_status::StatusCode};
use serde::{Deserialize, Serialize};

/// Mimics `0xcafe::test::ModuleData`
#[derive(Serialize, Deserialize)]
struct ModuleData {
    state: Vec<u8>,
}

const OBJECT_ADDRESS: AccountAddress = AccountAddress::new([
    0x66, 0x2E, 0x50, 0x41, 0x8C, 0xE5, 0xF3, 0x5A, 0x6C, 0xA8, 0xB7, 0x9E, 0x28, 0x7C, 0x94, 0x12,
    0x90, 0x71, 0xAA, 0x3F, 0xBD, 0x2A, 0xB9, 0x51, 0x37, 0xF7, 0xCB, 0xAD, 0x13, 0x6F, 0x09, 0x2B,
]);

fn success(tests: Vec<(&str, Vec<Vec<u8>>, &str)>) {
    success_generic(vec![], tests)
}

fn success_generic(ty_args: Vec<TypeTag>, tests: Vec<(&str, Vec<Vec<u8>>, &str)>) {
    let mut h = MoveHarness::new_with_features(vec![FeatureFlag::STRUCT_CONSTRUCTORS], vec![]);

    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    assert_success!(h.publish_package(&acc, &common::test_dir_path("constructor_args.data/pack")));

    let module_data = parse_struct_tag("0xCAFE::test::ModuleData").unwrap();

    // Check in initial state, resource does not exist.
    assert!(!h.exists_resource(acc.address(), module_data.clone()));

    for (entry, args, expected_change) in tests {
        assert_success!(h.run_entry_function(
            &acc,
            str::parse(entry).unwrap(),
            ty_args.clone(),
            args,
        ));
        assert_eq!(
            String::from_utf8(
                h.read_resource::<ModuleData>(&OBJECT_ADDRESS, module_data.clone())
                    .unwrap()
                    .state
            )
            .unwrap(),
            expected_change,
        );
    }
}

type Closure = Box<dyn FnOnce(TransactionStatus) -> bool>;

fn fail(tests: Vec<(&str, Vec<Vec<u8>>, Closure)>) {
    fail_generic(vec![], tests)
}

fn fail_generic(ty_args: Vec<TypeTag>, tests: Vec<(&str, Vec<Vec<u8>>, Closure)>) {
    let mut h = MoveHarness::new_with_features(vec![FeatureFlag::STRUCT_CONSTRUCTORS], vec![]);

    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    assert_success!(h.publish_package(&acc, &common::test_dir_path("constructor_args.data/pack")));

    let module_data = parse_struct_tag("0xCAFE::test::ModuleData").unwrap();

    // Check in initial state, resource does not exist.
    assert!(!h.exists_resource(acc.address(), module_data));

    for (entry, args, err) in tests {
        // Now send hi transaction, after that resource should exist and carry value
        err(h.run_entry_function(&acc, str::parse(entry).unwrap(), ty_args.clone(), args));
    }
}

#[test]
fn constructor_args_good() {
    let tests = vec![
        // ensure object exist
        ("0xcafe::test::initialize", vec![], ""),
        (
            "0xcafe::test::object_arg",
            vec![
                bcs::to_bytes("hi").unwrap(),
                bcs::to_bytes(&OBJECT_ADDRESS).unwrap(),
            ],
            "hi",
        ),
        (
            "0xcafe::test::pass_optional_fixedpoint32",
            vec![
                bcs::to_bytes(&OBJECT_ADDRESS).unwrap(),     // Object<T>
                bcs::to_bytes(&vec![(1u64 << 32)]).unwrap(), // Option<FixedPoint32>
            ],
            "4294967296",
        ),
        (
            "0xcafe::test::pass_optional_vector_fixedpoint64",
            vec![
                bcs::to_bytes(&OBJECT_ADDRESS).unwrap(), // Object<T>
                bcs::to_bytes(&vec![vec![(1u128 << 64), (2u128 << 64)]]).unwrap(), // Option<vector<FixedPoint64>>
                bcs::to_bytes(&1u64).unwrap(),
            ],
            "36893488147419103232", // 2 in fixedpoint64
        ),
        (
            "0xcafe::test::pass_optional_vector_optional_string",
            vec![
                bcs::to_bytes(&OBJECT_ADDRESS).unwrap(), // Object<T>
                bcs::to_bytes(&vec![vec![vec!["a"], vec!["b"]]]).unwrap(), // Option<vector<Option<String>>>
                bcs::to_bytes(&1u64).unwrap(),
            ],
            "b", // second element of the vector
        ),
        (
            "0xcafe::test::pass_vector_optional_object",
            vec![
                bcs::to_bytes(&vec![vec![OBJECT_ADDRESS], vec![]]).unwrap(), // vector<Option<Object<T>>>
                bcs::to_bytes(&"pff vectors of optionals").unwrap(),
                bcs::to_bytes(&0u64).unwrap(),
            ],
            "pff vectors of optionals",
        ),
    ];

    success(tests);
}

#[test]
fn constructor_args_bad() {
    let good: &[u8] = "a".as_bytes();
    let bad: &[u8] = &[0x80u8; 1];

    let tests: Vec<(&str, Vec<Vec<u8>>, Closure)> = vec![
        // object doesnt exist
        (
            "0xcafe::test::object_arg",
            vec![
                bcs::to_bytes("hi").unwrap(),
                bcs::to_bytes(&OBJECT_ADDRESS).unwrap(),
            ],
            Box::new(|e| {
                matches!(
                    e,
                    TransactionStatus::Keep(ExecutionStatus::MoveAbort { .. })
                )
            }),
        ),
        (
            "0xcafe::test::pass_optional_vector_optional_string",
            vec![
                bcs::to_bytes(&OBJECT_ADDRESS).unwrap(), // Object<T>
                bcs::to_bytes(&vec![vec![vec![good], vec![bad]]]).unwrap(), // Option<vector<Option<String>>>
                bcs::to_bytes(&1u64).unwrap(),
            ],
            Box::new(|e| {
                matches!(
                    e,
                    TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(
                        StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT
                    )))
                )
            }),
        ),
    ];

    fail(tests);
}
