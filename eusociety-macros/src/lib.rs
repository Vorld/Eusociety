use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// Procedural macro to derive the Component trait for structs.
/// 
/// This automatically implements the Component trait for any struct,
/// providing type ID and name functionality needed by the ECS system.
#[proc_macro_derive(Component)]
pub fn derive_component(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);
    // Get the name of the type we're deriving for
    let name = input.ident;

    // Generate the Component trait implementation
    let expanded = quote! {
        impl Component for #name {
            fn type_id() -> std::any::TypeId {
                std::any::TypeId::of::<Self>()
            }
            
            fn type_name() -> &'static str {
                std::any::type_name::<Self>()
            }
        }
    };

    // Convert back to token stream and return
    TokenStream::from(expanded)
}
