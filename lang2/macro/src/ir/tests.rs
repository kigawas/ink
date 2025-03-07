// Copyright 2018-2019 Parity Technologies (UK) Ltd.
// This file is part of ink!.
//
// ink! is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// ink! is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with ink!.  If not, see <http://www.gnu.org/licenses/>.

//! Tests for the ink! IR module.

use crate::ir::{
    Marker,
    Params,
};
use core::convert::TryFrom;

#[test]
fn parse_meta_storage() {
    let input: syn::Attribute = syn::parse_quote! { #[ink(storage)] };
    let result = Marker::try_from(input);
    assert!(result.is_ok());
    assert!(result.unwrap().is_simple("storage"));
}

#[test]
fn parse_meta_event() {
    let input: syn::Attribute = syn::parse_quote! { #[ink(event)] };
    let result = Marker::try_from(input);
    assert!(result.is_ok());
    assert!(result.unwrap().is_simple("event"));
}

#[test]
fn parse_params() {
    let _input: Params = syn::parse_quote! {
        env = DefaultSrmlTypes, version = "0.1.0"
    };
}
