use proc_macro::TokenStream;
use quote::{quote, format_ident};
use syn::{
    parse_macro_input, AngleBracketedGenericArguments, DeriveInput, FnArg, GenericArgument,
    ItemFn, Pat, PatIdent, PatType, PathArguments, Type, TypePath, TypeReference, ReturnType
}; // Removed unused imports: parse_quote, Path, PathSegment

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
        // The user of the macro is responsible for bringing `Component` trait into scope
        // e.g., `use eusociety_core::Component;` or `use crate::Component;`
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


/// Extracts the inner type from common ECS parameter types like &T, &mut T, Res<T>, ResMut<T>.
fn extract_inner_type(ty: &Type) -> Option<&Type> {
    match ty {
        Type::Reference(TypeReference { elem, .. }) => Some(elem),
        Type::Path(TypePath { path, .. }) => {
            path.segments.last().and_then(|segment| {
                match &segment.arguments {
                    PathArguments::AngleBracketed(AngleBracketedGenericArguments { args, .. }) => {
                        args.first().and_then(|arg| match arg {
                            GenericArgument::Type(inner_ty) => Some(inner_ty),
                            _ => None,
                        })
                    }
                    _ => None,
                }
            })
        }
        _ => None,
    }
}

/// Checks if the type is Res<T>
fn is_res(ty: &Type) -> bool {
    matches!(ty, Type::Path(TypePath { path, .. }) if path.segments.last().map_or(false, |seg| seg.ident == "Res"))
}

/// Checks if the type is ResMut<T>
fn is_res_mut(ty: &Type) -> bool {
    matches!(ty, Type::Path(TypePath { path, .. }) if path.segments.last().map_or(false, |seg| seg.ident == "ResMut"))
}

/// Checks if the type is &T
fn is_ref(ty: &Type) -> bool {
    matches!(ty, Type::Reference(TypeReference { mutability: None, .. }))
}

/// Checks if the type is &mut T
fn is_mut_ref(ty: &Type) -> bool {
    matches!(ty, Type::Reference(TypeReference { mutability: Some(_), .. }))
}


/// Procedural attribute macro to transform a function into a system.
///
/// This macro transforms a function with component and resource parameters
/// into a struct that implements the System trait, automatically tracking
/// the dependencies that the system declares.
///
/// # Example
///
/// ```rust,ignore
/// use eusociety_core::{World, Position, Velocity, DeltaTime};
/// use eusociety_core::ecs::resources::{Res, ResMut};
/// use eusociety_macros::system; // Make sure this is imported
///
/// #[system]
/// fn movement_system(pos: &mut Position, vel: &Velocity, dt: Res<DeltaTime>) {
///     pos.x += vel.x * dt.delta_seconds;
///     pos.y += vel.y * dt.delta_seconds;
/// }
/// ```
#[proc_macro_attribute]
pub fn system(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);

    // --- Basic function validation ---
    if input_fn.sig.asyncness.is_some() {
        return syn::Error::new_spanned(input_fn.sig.asyncness, "System functions cannot be async")
            .to_compile_error()
            .into();
    }
    if input_fn.sig.constness.is_some() {
        return syn::Error::new_spanned(input_fn.sig.constness, "System functions cannot be const")
            .to_compile_error()
            .into();
    }
    if input_fn.sig.abi.is_some() {
        return syn::Error::new_spanned(input_fn.sig.abi, "System functions cannot have custom ABIs")
            .to_compile_error()
            .into();
    }
     if !matches!(input_fn.sig.output, ReturnType::Default) {
        return syn::Error::new_spanned(input_fn.sig.output, "System functions must not return a value")
            .to_compile_error()
            .into();
    }

    let fn_vis = &input_fn.vis;
    let fn_name = &input_fn.sig.ident;
    let struct_name = format_ident!("{}System", fn_name);

    // --- Parameter Analysis ---
    let mut component_access_calls = Vec::new(); // Stores the quote! calls for DataAccess
    let mut resource_access_calls = Vec::new();  // Stores the quote! calls for DataAccess
    let mut param_bindings = Vec::new(); // Code to bind parameters inside run()
    let mut param_names = Vec::new(); // Names to pass to the original function call

    let mut first_component_type: Option<&Type> = None;
    let mut first_component_mut = false;
    let mut mutable_component_count = 0; // Track mutable components
    let mut has_immutable_component = false;
    let mut has_resource = false;
    let mut has_mutable_resource = false;


    for param in &input_fn.sig.inputs {
        match param {
            FnArg::Typed(PatType { pat, ty, .. }) => {
                let param_name = match &**pat {
                    Pat::Ident(PatIdent { ident, .. }) => ident,
                    _ => {
                        return syn::Error::new_spanned(pat, "System parameters must be simple identifiers")
                            .to_compile_error()
                            .into();
                    }
                };
                param_names.push(param_name.clone());

                if let Some(inner_ty) = extract_inner_type(ty) {
                    if is_res(ty) {
                        has_resource = true;
                        resource_access_calls.push(quote! { ::eusociety_core::DataAccess::read(::std::any::TypeId::of::<#inner_ty>()) });
                        param_bindings.push(quote! {
                            let #param_name: ::eusociety_core::resources::Res<#inner_ty> = world.get_resource::<#inner_ty>()
                                .map(::eusociety_core::resources::Res::new)
                                .expect(concat!("Resource not found: ", stringify!(#inner_ty)));
                        });
                    } else if is_res_mut(ty) {
                        has_resource = true;
                        has_mutable_resource = true;
                        resource_access_calls.push(quote! { ::eusociety_core::DataAccess::write(::std::any::TypeId::of::<#inner_ty>()) });
                         param_bindings.push(quote! {
                            let mut #param_name: ::eusociety_core::resources::ResMut<#inner_ty> = world.get_resource_mut::<#inner_ty>()
                                .map(::eusociety_core::resources::ResMut::new)
                                .expect(concat!("Mutable resource not found: ", stringify!(#inner_ty)));
                        });
                    } else if is_ref(ty) {
                        has_immutable_component = true;
                        component_access_calls.push(quote! { ::eusociety_core::DataAccess::read(::std::any::TypeId::of::<#inner_ty>()) });
                        if first_component_type.is_none() {
                            first_component_type = Some(inner_ty);
                            first_component_mut = false;
                        }
                        // Binding happens inside the loop
                    } else if is_mut_ref(ty) {
                        mutable_component_count += 1;
                        component_access_calls.push(quote! { ::eusociety_core::DataAccess::write(::std::any::TypeId::of::<#inner_ty>()) });
                         if first_component_type.is_none() {
                            first_component_type = Some(inner_ty);
                            first_component_mut = true;
                        }
                        // Binding happens inside the loop
                    } else {
                         return syn::Error::new_spanned(ty, "Unsupported system parameter type. Use &T, &mut T, Res<T>, or ResMut<T>.")
                            .to_compile_error()
                            .into();
                    }
                } else {
                     return syn::Error::new_spanned(ty, "Unsupported system parameter type. Use &T, &mut T, Res<T>, or ResMut<T>.")
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

    let has_mutable_component = mutable_component_count > 0;

    // --- Check for unsupported combinations ---
    if has_mutable_component && (has_immutable_component || has_resource) {
         return syn::Error::new_spanned(
            input_fn.sig.inputs,
            "Systems with mutable component access (`&mut T`) cannot currently access other components or resources via the #[system] macro due to borrow checker limitations. Please implement the `System` trait manually.",
        )
        .to_compile_error()
        .into();
    }
     if has_mutable_resource && (has_mutable_component || has_immutable_component) {
         return syn::Error::new_spanned(
            input_fn.sig.inputs,
            "Systems with mutable resource access (`ResMut<T>`) cannot currently access components via the #[system] macro due to borrow checker limitations. Please implement the `System` trait manually.",
        )
        .to_compile_error()
        .into();
    }
     if mutable_component_count > 1 {
        return syn::Error::new_spanned(
            input_fn.sig.inputs,
            "Systems cannot have more than one mutable component parameter (`&mut T`).",
        )
        .to_compile_error()
        .into();
    }


    // --- Generate Run Method Logic (Simplified based on valid patterns) ---
    let run_logic = if let Some(iter_comp_ty) = first_component_type {
        // Case: Accessing components (&T or single &mut T) + potentially Res<T>
        let mut component_bindings_in_loop = Vec::new();
        let mut resource_bindings_outside_loop = Vec::new(); // Res<T> only

        for param in &input_fn.sig.inputs {
             if let FnArg::Typed(PatType { pat, ty, .. }) = param {
                 if let Pat::Ident(pat_ident) = &**pat {
                     let param_name = &pat_ident.ident;
                     if let Some(inner_ty) = extract_inner_type(ty) {
                         if is_ref(ty) {
                             component_bindings_in_loop.push(quote! {
                                 let #param_name = world.get_component::<#inner_ty>(entity)
                                     .expect(concat!("Component not found for entity in system query: ", stringify!(#inner_ty)));
                             });
                         } else if is_mut_ref(ty) {
                             // This case implies no other params due to checks above
                             component_bindings_in_loop.push(quote! {
                                 let #param_name = world.get_component_mut::<#inner_ty>(entity)
                                     .expect(concat!("Mutable component not found for entity in system query: ", stringify!(#inner_ty)));
                             });
                         } else if is_res(ty) {
                             // Allowed with &T components
                             resource_bindings_outside_loop.push(quote! {
                                 let #param_name: ::eusociety_core::resources::Res<#inner_ty> = world.get_resource::<#inner_ty>()
                                     .map(::eusociety_core::resources::Res::new)
                                     .expect(concat!("Resource not found: ", stringify!(#inner_ty)));
                             });
                         }
                         // ResMut case is disallowed with components by checks above
                     }
                 }
             }
        }

        let query_iter = if first_component_mut {
            quote! { world.components.query_mut::<#iter_comp_ty>() }
        } else {
            quote! { world.components.query::<#iter_comp_ty>() }
        };

        quote! {
            // Fetch Res<T> outside loop
            #(#resource_bindings_outside_loop)*

            // Iterate based on the first component parameter
            for (entity, _) in #query_iter {
                 // Fetch components for the current entity
                 #(#component_bindings_in_loop)*

                 // Call the original function
                 #fn_name(#(#param_names),*); // Pass the bound variables
            }
        }
    } else {
        // Case: Only resources (Res<T> or ResMut<T>)
        quote! {
             // Fetch resources once
             #(#param_bindings)* // param_bindings contains resource fetches
             #fn_name(#(#param_names),*); // Pass the bound variables
        }
    };


    // --- Generate Struct and Impl ---
    let expanded = quote! {
        // Imports are now expected to be present in the module using the macro
        // e.g., use eusociety_core::{System, SystemAccess, DataAccess, AccessType, World};
        // e.g., use eusociety_core::resources::{Res, ResMut};

        #fn_vis struct #struct_name;

        // Use fully qualified paths for traits/structs from eusociety_core
        impl ::eusociety_core::System for #struct_name {
            fn access(&self) -> ::eusociety_core::SystemAccess {
                let mut access = ::eusociety_core::SystemAccess::new();
                // Push the generated DataAccess::read/write calls directly
                #(access.component_access.push(#component_access_calls);)*
                #(access.resource_access.push(#resource_access_calls);)*
                access
            }

            #[allow(unused_mut)] // world might not be used if only resources are accessed or only read components
            fn run(&mut self, world: &mut ::eusociety_core::World) {
                #run_logic
            }

            fn name(&self) -> &str {
                stringify!(#fn_name)
            }
        }

        // Keep the original function, perhaps marked private or unused if needed
        // Or remove it if the struct is the canonical way to refer to the system
         #[allow(dead_code)] // Prevent unused warning if the struct is used directly
         #[allow(clippy::too_many_arguments)] // Systems might have many params
         #input_fn
    };

    TokenStream::from(expanded)
}
