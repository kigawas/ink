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

//! Code generation for normal Wasm smart contract builds.
//!
//! Generated code that conflicts with specialized `test` or `doc`
//! code needs to be guarded by `#[cfg(..)]`.

use std::iter;

use proc_macro2::{
    Ident,
    TokenStream as TokenStream2,
};
use quote::{
    quote,
    ToTokens,
};
use syn::{
    punctuated::Punctuated,
    Token,
};

use crate::{
    ast,
    gen::selector_to_expr,
    hir,
};

pub fn generate_code(tokens: &mut TokenStream2, contract: &hir::Contract) {
    let env_items = {
        let mut result = quote! {};
        let tokens = &mut result;
        codegen_for_contract_env(tokens, contract);
        result
    };
    let mod_body = {
        let mut result = quote! {};
        let tokens = &mut result;
        // codegen_for_contract_env(tokens, contract);
        codegen_for_state(tokens, contract);
        codegen_for_messages(tokens, contract);
        codegen_for_message_impls(tokens, contract);
        codegen_for_method_impls(tokens, contract);
        codegen_for_instantiate(tokens, contract);
        codegen_for_entry_points(tokens, contract);
        codegen_for_event_mod(tokens, contract);
        result
    };
    tokens.extend(quote! {
        #env_items

        #[cfg(not(feature = "ink-as-dependency"))]
        mod normal {
            use super::*;
            #mod_body
        }
        #[cfg(not(feature = "ink-as-dependency"))]
        use normal::*;

        #[cfg(not(feature = "ink-as-dependency"))]
        use ink_core::env::FromAccountId as _;
    });
}

fn codegen_for_contract_env(tokens: &mut TokenStream2, contract: &hir::Contract) {
    let env_types = &contract.env_types_type;
    tokens.extend(quote! {
        mod types {
            use super::*;
            use ink_core::env::{ContractEnv, EnvTypes};

            pub type AccountId = <#env_types as EnvTypes>::AccountId;
            pub type Balance = <#env_types as EnvTypes>::Balance;
            pub type Hash = <#env_types as EnvTypes>::Hash;
            pub type Moment = <#env_types as EnvTypes>::Moment;
            pub type BlockNumber = <#env_types as EnvTypes>::BlockNumber;
        }

        type Env = ink_core::env::ContractEnv<#env_types>;
        use types::{
            AccountId,
            Balance,
            Hash,
            Moment,
            BlockNumber,
        };
    })
}

fn codegen_for_event_mod(tokens: &mut TokenStream2, contract: &hir::Contract) {
    if contract.events.is_empty() {
        // Do nothing if the user specified no events
        return
    }
    let use_event_body = {
        let mut content = quote! {};
        for event in contract.events.iter() {
            let ident = &event.ident;
            content.extend(quote! {
                #ident,
            })
        }
        content
    };
    let mod_event_body = {
        let mut content = quote! {};
        codegen_for_event_private_mod(&mut content, contract);
        codegen_for_events(&mut content, contract);
        codegen_for_event_emit_trait(&mut content, contract);
        content
    };
    tokens.extend(quote! {
        mod events {
            use super::*;

            #mod_event_body
        }

        use events::{
            EmitEventExt as _,
            #use_event_body
        };
    })
}

fn codegen_for_event_private_mod(tokens: &mut TokenStream2, contract: &hir::Contract) {
    let event_enum_mod_body = {
        let mut content = quote! {};
        for event in contract.events.iter() {
            let name = &event.ident;
            content.extend(quote! {
                #name(#name),
            })
        }
        content
    };
    tokens.extend(quote! {
        mod private {
            use super::*;

            #[doc(hidden)]
            #[derive(scale::Encode, scale::Decode)]
            pub enum Event {
                #event_enum_mod_body
            }

            /// Used to seal the emit trait.
            pub trait Sealed {}
        }
    })
}

impl quote::ToTokens for hir::Event {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        for attr in &self.attrs {
            attr.to_tokens(tokens)
        }
        <Token![pub]>::default().to_tokens(tokens);
        <Token![struct]>::default().to_tokens(tokens);
        self.ident.to_tokens(tokens);
        syn::token::Brace::default().surround(tokens, |inner| {
            for arg in self.args.iter() {
                for attr in &arg.attrs {
                    attr.to_tokens(inner)
                }
                <Token![pub]>::default().to_tokens(inner);
                arg.to_tokens(inner);
                <Token![,]>::default().to_tokens(inner);
            }
        });
    }
}

impl quote::ToTokens for ast::EventArg {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        self.ident.to_tokens(tokens);
        self.colon_tok.to_tokens(tokens);
        self.ty.to_tokens(tokens);
    }
}

fn codegen_for_events(tokens: &mut TokenStream2, contract: &hir::Contract) {
    for event in contract.events.iter() {
        let ident = &event.ident;

        tokens.extend(quote! {
            #[derive(scale::Encode, scale::Decode)]
            #event

            impl From<#ident> for private::Event {
                fn from(event: #ident) -> Self {
                    private::Event::#ident(event)
                }
            }
        })
    }
}

fn codegen_for_event_emit_trait(tokens: &mut TokenStream2, contract: &hir::Contract) {
    let env_types = &contract.env_types_type;
    tokens.extend(quote! {
        pub trait EmitEventExt: private::Sealed {
            /// Emits the given event.
            fn emit<E>(&self, event: E)
            where
                E: Into<private::Event>,
            {
                use scale::Encode as _;
                <ink_core::env::ContractEnv<#env_types> as ink_core::env::Env>::deposit_raw_event(
                    &[], event.into().encode().as_slice()
                )
            }
        }

        impl EmitEventExt for ink_model::EnvHandler<ink_core::env::ContractEnv<#env_types>> {}
        impl private::Sealed for ink_model::EnvHandler<ink_core::env::ContractEnv<#env_types>> {}
    })
}

fn codegen_for_entry_points(tokens: &mut TokenStream2, contract: &hir::Contract) {
    let state_name = &contract.name;

    tokens.extend(quote! {
        #[cfg(not(test))]
        #[no_mangle]
        fn deploy() -> u32 {
            #state_name::instantiate().deploy().to_u32()
        }

        #[cfg(not(test))]
        #[no_mangle]
        fn call() -> u32 {
            #state_name::instantiate().dispatch().to_u32()
        }
    })
}

fn codegen_for_instantiate(tokens: &mut TokenStream2, contract: &hir::Contract) {
    let state_name = &contract.name;
    let env_types = &contract.env_types_type;

    let deploy_handler_toks = {
        let deploy_fn_args = {
            let mut deploy_fn_args: Punctuated<ast::FnArg, Token![,]> = Punctuated::new();
            for input in contract.on_deploy.sig.inputs.iter().skip(1) {
                deploy_fn_args.push(input.clone())
            }
            deploy_fn_args
        };
        let deploy_call_args = {
            let mut deploy_call_args: Punctuated<syn::Pat, Token![,]> = Punctuated::new();
            for pat_ty in deploy_fn_args.iter().filter_map(|fn_arg| {
                if let ast::FnArg::Typed(pat_ty) = fn_arg {
                    Some(pat_ty)
                } else {
                    None
                }
            }) {
                deploy_call_args.push((*pat_ty.pat).clone())
            }
            deploy_call_args
        };
        let deploy_fn_args_toks = match deploy_fn_args.iter().count() {
            0 => quote! {()},
            1 => deploy_fn_args.into_token_stream(),
            _ => {
                let mut toks = quote! {};
                syn::token::Paren::default().surround(&mut toks, |surrounded_toks| {
                    deploy_call_args.to_tokens(surrounded_toks)
                });
                toks
            }
        };
        quote! {
            .on_deploy(|env, #deploy_fn_args_toks| {
                let (handler, state) = env.split_mut();
                state.deploy(handler, #deploy_call_args)
            })
        }
    };

    let messages_toks = {
        let mut content = quote! {};
        for message in &contract.messages {
            let msg_ident = &message.sig.ident;

            use heck::CamelCase as _;
            let camelcase_msg_ident = Ident::new(
                &message.sig.ident.to_string().to_camel_case(),
                message.sig.ident.span(),
            );

            let msg_fn_args = {
                let mut msg_fn_args: Punctuated<ast::FnArg, Token![,]> =
                    Default::default();
                for input in message.sig.inputs.iter().skip(1) {
                    msg_fn_args.push(input.clone())
                }
                msg_fn_args
            };

            let msg_call_args = {
                let mut msg_call_args: Punctuated<syn::Pat, Token![,]> =
                    Default::default();
                for pat_ty in msg_fn_args.iter().filter_map(|fn_arg| {
                    if let ast::FnArg::Typed(pat_ty) = fn_arg {
                        Some(pat_ty)
                    } else {
                        None
                    }
                }) {
                    msg_call_args.push((*pat_ty.pat).clone())
                }
                msg_call_args
            };

            let msg_fn_args_toks = match msg_fn_args.iter().count() {
                0 => quote! {_},
                1 => msg_fn_args.into_token_stream(),
                _ => {
                    let mut toks = quote! {};
                    syn::token::Paren::default().surround(&mut toks, |surrounded_toks| {
                        msg_call_args.to_tokens(surrounded_toks)
                    });
                    toks
                }
            };

            let msg_toks = if message.is_mut() {
                quote! {
                    .on_msg_mut::< msg::#camelcase_msg_ident >(|env, #msg_fn_args_toks| {
                        let (handler, state) = env.split_mut();
                        state. #msg_ident (handler, #msg_call_args)
                    })
                }
            } else {
                quote! {
                    .on_msg::< msg::#camelcase_msg_ident >(|env, #msg_fn_args_toks| {
                        let (handler, state) = env.split();
                        state.  #msg_ident (handler, #msg_call_args)
                    })
                }
            };
            msg_toks.to_tokens(&mut content)
        }
        content
    };

    tokens.extend(quote! {
        use ink_model::Contract as _;

        #[cfg(not(test))]
        impl #state_name {
            pub(crate) fn instantiate() -> impl ink_model::Contract {
                ink_model::ContractDecl::using::<Self, ink_core::env::ContractEnv<#env_types>>()
                    #deploy_handler_toks
                    #messages_toks
                    .instantiate()
            }
        }
    })
}

fn codegen_for_message_impls(tokens: &mut TokenStream2, contract: &hir::Contract) {
    let state_name = &contract.name;
    let env_types = &contract.env_types_type;
    let env_handler: syn::Type = syn::parse_quote! {
        ink_model::EnvHandler<ink_core::env::ContractEnv<#env_types>>
    };
    let message_impls = {
        let mut content = quote! {};
        for message in iter::once(&contract.on_deploy.clone().into_message())
            .chain(contract.messages.iter())
        {
            for attr in &message.attrs {
                attr.to_tokens(&mut content)
            }
            <Token![pub]>::default().to_tokens(&mut content);
            let sig = &message.sig;
            sig.fn_tok.to_tokens(&mut content);
            message.sig.ident.to_tokens(&mut content);
            sig.paren_tok.surround(&mut content, |inner_toks| {
                let inputs_with_env = sig.inputs_with_env(&env_handler);
                inputs_with_env.to_tokens(inner_toks);
            });
            sig.output.to_tokens(&mut content);
            message.block.to_tokens(&mut content);
        }
        content
    };
    tokens.extend(quote! {
        impl #state_name {
            #message_impls
        }
    });
}

fn codegen_for_method_impls(tokens: &mut TokenStream2, contract: &hir::Contract) {
    let state_name = &contract.name;
    let methods_impls = {
        let mut content = quote! {};
        for method in contract.methods.iter() {
            for attr in &method.attrs {
                attr.to_tokens(&mut content)
            }
            let sig = &method.sig;
            sig.fn_tok.to_tokens(&mut content);
            method.sig.ident.to_tokens(&mut content);
            let generics = &sig.generics;
            generics.lt_token.to_tokens(&mut content);
            generics.params.to_tokens(&mut content);
            generics.gt_token.to_tokens(&mut content);
            sig.paren_tok.surround(&mut content, |inner_toks| {
                sig.inputs.to_tokens(inner_toks);
            });
            sig.output.to_tokens(&mut content);
            generics.where_clause.to_tokens(&mut content);
            method.block.to_tokens(&mut content);
        }
        content
    };
    if contract.methods.iter().count() > 0 {
        tokens.extend(quote! {
            impl #state_name {
                #methods_impls
            }
        })
    }
}

fn codegen_for_state(tokens: &mut TokenStream2, contract: &hir::Contract) {
    let state_attrs_toks = {
        let mut content = quote! {};
        for attr in &contract.state.attrs {
            attr.to_tokens(&mut content)
        }
        content
    };
    let struct_fields_toks = &contract.state.fields;
    let name = &contract.name;
    tokens.extend(quote! {
        ink_model::state! {
            #state_attrs_toks
            #[cfg_attr(
                feature = "ink-generate-abi",
                derive(
                    type_metadata::Metadata,
                    ink_abi::HasLayout,
                )
            )]
            pub struct #name
                #struct_fields_toks
        }
    });
}

fn codegen_for_messages(tokens: &mut TokenStream2, contract: &hir::Contract) {
    let messages_content = {
        let mut content = quote! {};
        for message in contract.messages.iter() {
            for attr in &message.attrs {
                attr.to_tokens(&mut content)
            }
            let msg_id = selector_to_expr(message.selector());
            msg_id.to_tokens(&mut content);
            <Token![=>]>::default().to_tokens(&mut content);
            use heck::CamelCase as _;
            let camel_case_ident = Ident::new(
                &message.sig.ident.to_string().to_camel_case(),
                message.sig.ident.span(),
            );
            camel_case_ident.to_tokens(&mut content);
            let sig = &message.sig;
            sig.paren_tok.surround(&mut content, |inner_toks| {
                let args_without_self = {
                    let mut args_without_self: Punctuated<ast::FnArg, Token![,]> =
                        Punctuated::new();
                    for fn_arg in sig
							.inputs.iter()
							// Performing `skip(1)` here works because we already asserted
							// that all messages have to start with either `&self` or `&mut self`.
							.skip(1)
                    {
                        args_without_self.push(fn_arg.clone())
                    }
                    args_without_self
                };
                args_without_self.to_tokens(inner_toks)
            });
            sig.output.to_tokens(&mut content);
            <Token![;]>::default().to_tokens(&mut content);
        }
        content
    };
    tokens.extend(quote! {
        mod msg {
            use super::*;
            // Apparently this `use` is required even though it should not be.
            // -> Further investigations needed!
            use ink_model::messages;
            ink_model::messages! {
                #messages_content
            }
        }
    })
}
