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

use crate::{
    codegen::{
        cross_calling::CrossCallingConflictCfg,
        GenerateCode,
        GenerateCodeUsing,
    },
    ir,
};
use derive_more::From;
use proc_macro2::TokenStream as TokenStream2;
use quote::{
    quote,
    quote_spanned,
};
use syn::{
    punctuated::Punctuated,
    Token,
};

/// Generates code for the dispatch parts that dispatch constructors
/// and messages from the input and also handle the returning of data.
#[derive(From)]
pub struct Dispatch<'a> {
    /// The contract to generate code for.
    contract: &'a ir::Contract,
}

impl<'a> GenerateCodeUsing for Dispatch<'a> {
    fn contract(&self) -> &ir::Contract {
        self.contract
    }
}

impl GenerateCode for Dispatch<'_> {
    fn generate_code(&self) -> TokenStream2 {
        let conflic_depedency_cfg = self.generate_code_using::<CrossCallingConflictCfg>();
        let message_trait_impls = self.generate_message_trait_impls();
        let message_namespaces = self.generate_message_namespaces();
        let dispatch_using_mode = self.generate_dispatch_using_mode();
        let entry_points = self.generate_entry_points();

        quote! {
            // We do not generate contract dispatch code
            // while the contract is being tested or the
            // `test-env` has been enabled since both resulting
            // compilations do not require dispatching.
            #[cfg(not(any(test, feature = "test-env")))]
            #conflic_depedency_cfg
            const _: () = {
                #message_namespaces
                #message_trait_impls
                #dispatch_using_mode
                #entry_points
            };
        }
    }
}

impl Dispatch<'_> {
    fn generate_trait_impls_for_message(&self, function: &ir::Function) -> TokenStream2 {
        if !(function.is_constructor() || function.is_message()) {
            return quote! {}
        }
        let span = function.span();
        let selector = function
            .selector()
            .expect("this is either a message or constructor at this point; qed");
        let (selector_bytes, selector_id) = (selector.as_bytes(), selector.unique_id());
        let sig = &function.sig;
        let inputs = sig.inputs().map(|ident_type| &ident_type.ty);
        let inputs_punct = inputs.collect::<Punctuated<_, Token![,]>>();
        let output = &sig.output;
        let output_type = match output {
            syn::ReturnType::Default => quote! {},
            syn::ReturnType::Type(_, ty) => quote! { #ty },
        };
        let is_mut = sig.is_mut();

        use syn::spanned::Spanned as _;

        let namespace = match function.kind() {
            ir::FunctionKind::Constructor(_) => quote! { Constr },
            ir::FunctionKind::Message(_) => quote! { Msg },
            ir::FunctionKind::Method => panic!("ICE: can't match a method at this point"),
        };

        let fn_input = quote_spanned!(sig.inputs.span() =>
            impl ink_lang2::FnInput for #namespace<[(); #selector_id]> {
                #[allow(unused_parens)]
                type Input = (#inputs_punct);
            }
        );
        let fn_output = quote_spanned!(sig.output.span() =>
            impl ink_lang2::FnOutput for #namespace<[(); #selector_id]> {
                #[allow(unused_parens)]
                type Output = (#output_type);
            }
        );
        let fn_selector = quote_spanned!(span =>
            impl ink_lang2::FnSelector for #namespace<[(); #selector_id]> {
                const SELECTOR: ink_core::env2::call::Selector = ink_core::env2::call::Selector::from_bytes([
                    #( #selector_bytes ),*
                ]);
            }
        );
        let message_impl = quote_spanned!(span =>
            impl ink_lang2::Message for #namespace<[(); #selector_id]> {
                const IS_MUT: bool = #is_mut;
            }
        );

        quote_spanned!(span =>
            #fn_input
            #fn_output
            #fn_selector
            #message_impl
        )
    }

    fn generate_message_trait_impls(&self) -> TokenStream2 {
        let fns = self
            .contract
            .functions
            .iter()
            .map(|fun| self.generate_trait_impls_for_message(fun));
        quote! {
            #( #fns )*
        }
    }

    fn generate_message_namespaces(&self) -> TokenStream2 {
        quote! {
            // Namespace for messages.
            //
            // # Note
            //
            // The `S` parameter is going to refer to array types `[(); N]`
            // where `N` is the unique identifier of the associated message
            // selector.
            pub struct Msg<S> {
                // We need to wrap inner because of Rust's orphan rules.
                marker: core::marker::PhantomData<fn() -> S>,
            }

            // Namespace for constructors.
            //
            // # Note
            //
            // The `S` parameter is going to refer to array types `[(); N]`
            // where `N` is the unique identifier of the associated constructor
            // selector.
            pub struct Constr<S> {
                // We need to wrap inner because of Rust's orphan rules.
                marker: core::marker::PhantomData<fn() -> S>,
            }
        }
    }

    fn generate_dispatch_using_mode_fragment(
        &self,
        function: &ir::Function,
    ) -> TokenStream2 {
        if !(function.is_constructor() || function.is_message()) {
            return quote! {}
        }
        let selector = function
            .selector()
            .expect("this is either a message or constructor at this point; qed");
        let selector_id = selector.unique_id();
        let sig = &function.sig;
        let input_idents = sig
            .inputs()
            .map(|ident_type| &ident_type.ident)
            .collect::<Punctuated<_, Token![,]>>();
        let (pat_idents, fn_idents) = if input_idents.is_empty() {
            (quote! { _ }, quote! {})
        } else {
            (quote! { (#input_idents) }, quote! { #input_idents })
        };

        let builder_name = if function.is_constructor() {
            quote! { on_instantiate }
        } else if sig.is_mut() {
            quote! { on_msg_mut }
        } else {
            quote! { on_msg }
        };

        let namespace = match function.kind() {
            ir::FunctionKind::Constructor(_) => quote! { Constr },
            ir::FunctionKind::Message(_) => quote! { Msg },
            ir::FunctionKind::Method => panic!("ICE: can't match a method at this point"),
        };
        let fn_name = &sig.ident;

        quote! {
            .#builder_name::<#namespace<[(); #selector_id]>>(|storage, #pat_idents| {
                storage.#fn_name(#fn_idents)
            })
        }
    }

    fn generate_dispatch_using_mode(&self) -> TokenStream2 {
        let fragments = self
            .contract
            .functions
            .iter()
            .map(|fun| self.generate_dispatch_using_mode_fragment(fun));

        quote! {
            impl ink_lang2::DispatchUsingMode for StorageAndEnv {
                #[allow(unused_parens)]
                fn dispatch_using_mode(
                    mode: ink_lang2::DispatchMode
                ) -> core::result::Result<(), ink_lang2::DispatchError> {
                    ink_lang2::Contract::with_storage::<StorageAndEnv>()
                        #(
                            #fragments
                        )*
                        .done()
                        .dispatch_using_mode(mode)
                }
            }
        }
    }

    fn generate_entry_points(&self) -> TokenStream2 {
        quote! {
            #[cfg(not(test))]
            #[no_mangle]
            fn deploy() -> u32 {
                ink_lang2::DispatchRetCode::from(
                    <StorageAndEnv as ink_lang2::DispatchUsingMode>::dispatch_using_mode(
                        ink_lang2::DispatchMode::Instantiate,
                    ),
                )
                .to_u32()
            }

            #[cfg(not(test))]
            #[no_mangle]
            fn call() -> u32 {
                ink_lang2::DispatchRetCode::from(
                    <StorageAndEnv as ink_lang2::DispatchUsingMode>::dispatch_using_mode(
                        ink_lang2::DispatchMode::Call,
                    ),
                )
                .to_u32()
            }
        }
    }
}
