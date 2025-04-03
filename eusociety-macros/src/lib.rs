use proc_macro::TokenStream;
use quote::{quote, format_ident};
use syn::{
    parse_macro_input, DeriveInput, FnArg, ItemFn, Pat, PatIdent, PatType, ReturnType, Type,
    punctuated::Punctuated, token::Comma, Generics, Signature, Block
};
use syn::spanned::Spanned; // Import Spanned

/// Procedural macro to derive the Component trait for structs.
///
/// This automatically implements the Component trait for any struct,
/// providing type ID and name functionality needed by the ECS system.
#[proc_macro_derive(Component)]
pub fn derive_component(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as syn::DeriveInput);
    // Get the name of the type we're deriving for
    let name = input.ident;

    // Generate the Component trait implementation
    let expanded = quote! {
        // Use relative path for Component trait
        impl Component for #name {
            fn type_id() -> ::std::any::TypeId {
                ::std::any::TypeId::of::<Self>()
            }

            fn type_name() -> &'static str {
                ::std::any::type_name::<Self>()
            }
        }
    };

    // Convert back to token stream and return
    TokenStream::from(expanded)
}


/// Procedural attribute macro to transform a function into a system based on SystemParam.
///
/// # Example
///
/// ```rust,ignore
/// use eusociety_core::{World, Position, Velocity, DeltaTime, Res, ResMut, Query};
/// use eusociety_macros::system;
///
/// #[system]
/// fn movement_system(mut query: Query<(&mut Position, &Velocity)>, dt: Res<DeltaTime>) {
///     for (mut pos, vel) in query.iter() {
///         pos.x += vel.dx * dt.delta_seconds;
///         pos.y += vel.dy * dt.delta_seconds;
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn system(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);
    let ItemFn { attrs, vis, sig, block } = input_fn;
    let Signature { ident: fn_name, generics, inputs: params, output, .. } = sig;

    // --- Basic function validation ---
    if !matches!(output, ReturnType::Default) {
        return syn::Error::new_spanned(output, "System functions must not return a value")
            .to_compile_error()
            .into();
    }
    if !generics.params.is_empty() {
        return syn::Error::new_spanned(generics, "System functions cannot have generic parameters")
            .to_compile_error()
            .into();
    }
    
    // Parse parameters
    let mut param_names = Vec::new();
    let mut param_types = Vec::new();
    
    for param in params.iter() {
        match param {
            FnArg::Typed(PatType { pat, ty, .. }) => {
                // Ensure the pattern is a simple identifier
                if let Pat::Ident(PatIdent { ident: param_name, .. }) = &**pat {
                    param_names.push(param_name.clone());
                    param_types.push(ty.clone());
                } else {
                    return syn::Error::new_spanned(pat, "System parameters must be simple identifiers")
                        .to_compile_error()
                        .into();
                }
            }
            FnArg::Receiver(_) => {
                return syn::Error::new_spanned(param, "System functions cannot take 'self'")
                    .to_compile_error()
                    .into();
            }
        }
    }
    
    // Generate the wrapper function with proper type annotations
    let param_count = param_types.len();
    
    // Modify the param types to remove explicit lifetimes
    // This will allow the SystemParam trait to handle lifetimes properly
    let cleaned_param_types = param_types.iter().map(|ty| {
        quote! { #ty }
    }).collect::<Vec<_>>();
    
    let into_system_impl = match param_count {
        1 => {
            quote! {
                impl ::eusociety_core::ecs::system::IntoSystem<#(#cleaned_param_types),*, _> for #fn_name {
                    type System = ::eusociety_core::ecs::system::SystemFunction<Self, #(#cleaned_param_types),*>;
                    
                    fn into_system(self) -> Self::System {
                        ::eusociety_core::ecs::system::SystemFunction {
                            func: self,
                            _marker: ::std::marker::PhantomData,
                        }
                    }
                }
            }
        },
        2 => {
            quote! {
                impl ::eusociety_core::ecs::system::IntoSystem<(#(#cleaned_param_types),*), _> for #fn_name {
                    type System = ::eusociety_core::ecs::system::SystemFunction2<Self, #(#cleaned_param_types),*>;
                    
                    fn into_system(self) -> Self::System {
                        ::eusociety_core::ecs::system::SystemFunction2 {
                            func: self,
                            _marker: ::std::marker::PhantomData,
                        }
                    }
                }
            }
        },
        3 => {
            quote! {
                impl ::eusociety_core::ecs::system::IntoSystem<(#(#cleaned_param_types),*), _> for #fn_name {
                    type System = ::eusociety_core::ecs::system::SystemFunction3<Self, #(#cleaned_param_types),*>;
                    
                    fn into_system(self) -> Self::System {
                        ::eusociety_core::ecs::system::SystemFunction3 {
                            func: self,
                            _marker: ::std::marker::PhantomData,
                        }
                    }
                }
            }
        },
        4 => {
            quote! {
                impl ::eusociety_core::ecs::system::IntoSystem<(#(#cleaned_param_types),*), _> for #fn_name {
                    type System = ::eusociety_core::ecs::system::SystemFunction4<Self, #(#cleaned_param_types),*>;
                    
                    fn into_system(self) -> Self::System {
                        ::eusociety_core::ecs::system::SystemFunction4 {
                            func: self,
                            _marker: ::std::marker::PhantomData,
                        }
                    }
                }
            }
        },
        _ => {
            return syn::Error::new_spanned(&params, "System functions with more than 4 parameters are not currently supported")
                .to_compile_error()
                .into();
        }
    };
    
    // Keep the original function but make it return a system
    let expanded = quote! {
        // Keep the original function attributes
        #(#attrs)*
        #vis fn #fn_name() -> impl ::eusociety_core::ecs::system::System + Send + Sync {
            // Define the function that will be called by the system
            #[inline]
            fn inner_system #generics(#params) {
                #block
            }
            
            // Return the system by converting the function
            inner_system.into_system()
        }
        
        // Implement IntoSystem for the function
        #into_system_impl
    };
    
    TokenStream::from(expanded)
}
