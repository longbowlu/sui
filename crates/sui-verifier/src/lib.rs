// Copyright (c) 2022, Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

pub mod verifier;

pub mod entry_points_verifier;
pub mod global_storage_access_verifier;
pub mod id_immutable_verifier;
pub mod id_leak_verifier;
pub mod struct_with_key_verifier;

use move_binary_format::{
    binary_views::BinaryIndexedView,
    file_format::{SignatureToken, StructHandleIndex},
};
use move_core_types::{account_address::AccountAddress, identifier::IdentStr};
use sui_types::error::SuiError;

fn verification_failure(error: String) -> SuiError {
    SuiError::ModuleVerificationFailure { error }
}

// TODO move these to move bytecode utils
pub fn resolve_struct<'a>(
    view: &'a BinaryIndexedView,
    sidx: StructHandleIndex,
) -> (&'a AccountAddress, &'a IdentStr, &'a IdentStr) {
    let shandle = view.struct_handle_at(sidx);
    let mhandle = view.module_handle_at(shandle.module);
    let address = view.address_identifier_at(mhandle.address);
    let module_name = view.identifier_at(mhandle.name);
    let struct_name = view.identifier_at(shandle.name);
    (address, module_name, struct_name)
}

pub fn format_signature_token(view: &BinaryIndexedView, t: &SignatureToken) -> String {
    match t {
        SignatureToken::Bool => "bool".to_string(),
        SignatureToken::U8 => "u8".to_string(),
        SignatureToken::U64 => "u64".to_string(),
        SignatureToken::U128 => "u128".to_string(),
        SignatureToken::Address => "address".to_string(),
        SignatureToken::Signer => "signer".to_string(),
        SignatureToken::Vector(inner) => {
            format!("vector<{}>", format_signature_token(view, inner))
        }
        SignatureToken::Reference(inner) => format!("&{}", format_signature_token(view, inner)),
        SignatureToken::MutableReference(inner) => {
            format!("&mut {}", format_signature_token(view, inner))
        }
        SignatureToken::TypeParameter(i) => format!("T{}", i),

        SignatureToken::Struct(idx) => format_signature_token_struct(view, *idx, &[]),
        SignatureToken::StructInstantiation(idx, ty_args) => {
            format_signature_token_struct(view, *idx, ty_args)
        }
    }
}

pub fn format_signature_token_struct(
    view: &BinaryIndexedView,
    sidx: StructHandleIndex,
    ty_args: &[SignatureToken],
) -> String {
    let (address, module_name, struct_name) = resolve_struct(view, sidx);
    let s;
    let ty_args_string = if ty_args.is_empty() {
        ""
    } else {
        s = format!(
            "<{}>",
            ty_args
                .iter()
                .map(|t| format_signature_token(view, t))
                .collect::<Vec<_>>()
                .join(", ")
        );
        &s
    };
    format!(
        "0x{}::{}::{}{}",
        address.short_str_lossless(),
        module_name,
        struct_name,
        ty_args_string
    )
}
