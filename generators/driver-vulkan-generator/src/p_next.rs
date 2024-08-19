use std::{fs, path::Path};

use itertools::Itertools;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use vkxml::{
    EnumerationElement, EnumsElement, ExtensionElement, ExtensionSpecificationElement,
    RegistryElement,
};

const VK_STRUCTURE_TYPE: &str = "VK_STRUCTURE_TYPE_";

use crate::vulkan_types::TypeVulkan;

pub fn generate(project_directory: &Path, registry: &vkxml::Registry, types: &TypeVulkan) {
    let structure_types = get_structure_types(registry, types);

    let cdebugger = cdebugger(&structure_types);

    let result = quote! {
        //! THIS FILE IS GENERATED BY TOOL, DO NOT MODIFY.

        use ash::vk::StructureType;
        use crate::generated::vulkan_types::*;
        use std::{ffi::c_void, fmt::Debug};

        #cdebugger
    };

    let path = project_directory.join("crates/driver-common-vulkan/src/generated/p_next.rs");
    fs::create_dir_all(path.parent().unwrap()).expect("create directories");
    fs::write(path, result.to_string()).expect("write to a file");
}

fn cdebugger(structure_types: &[(String, String)]) -> TokenStream {
    let mut quotes = Vec::new();
    for (scream_name, type_name) in structure_types {
        let id = Ident::new(&scream_name[VK_STRUCTURE_TYPE.len()..], Span::call_site());
        let type_name = Ident::new(type_name, Span::call_site());

        quotes.push(quote! {
            StructureType::#id => &*(obj as *const #type_name),
        })
    }

    let quotes = quotes.into_iter().collect::<TokenStream>();
    quote! {
        /// # Safety
        /// Obj must be valid pointer to a structure supported by this function.
        pub unsafe fn p_next_cdebugger(obj: *const c_void) -> &'static dyn Debug {
            let ty = *(obj as *const StructureType);
            match ty {
                #quotes
                _ => panic!("Unknown structure type: {:?}", ty),
            }
        }
    }
}

fn get_structure_types(registry: &vkxml::Registry, types: &TypeVulkan) -> Vec<(String, String)> {
    registry
        .elements
        .iter()
        .flat_map(|x| match x {
            RegistryElement::Enums(x) => Some(
                x.elements
                    .iter()
                    .filter_map(|x| match x {
                        EnumsElement::Enumeration(x) => Some(x),
                        _ => None,
                    })
                    .flat_map(|x| &x.elements)
                    .filter_map(|x| match x {
                        EnumerationElement::Enum(x) => Some(x),
                        _ => None,
                    })
                    .filter(|x| x.name.starts_with("VK_STRUCTURE_TYPE"))
                    .map(|x| &x.name)
                    .collect_vec(),
            ),
            RegistryElement::Extensions(x) => Some(
                x.elements
                    .iter()
                    .flat_map(|x| &x.elements)
                    .filter_map(|x| match x {
                        ExtensionElement::Require(x) => Some(x),
                        _ => None,
                    })
                    .flat_map(|x| &x.elements)
                    .filter_map(|x| match x {
                        ExtensionSpecificationElement::Enum(x) => Some(x),
                        _ => None,
                    })
                    .filter(|x| x.name.starts_with("VK_STRUCTURE_TYPE"))
                    .map(|x| &x.name)
                    .collect_vec(),
            ),
            _ => None,
        })
        .flatten()
        .unique()
        .filter_map(|x| {
            let y = x[VK_STRUCTURE_TYPE.len()..]
                .replace('_', "")
                .to_ascii_lowercase();
            for ty in &types.types {
                if let Some(name) = &ty.name {
                    let i = get_index_of_prefix_end(name);
                    if y == name[i..].to_ascii_lowercase() {
                        return Some((x.clone(), name.clone()));
                    }
                }
            }
            None
        })
        .collect_vec()
}

fn get_index_of_prefix_end(text: &str) -> usize {
    if text.starts_with("Vk") {
        2
    } else {
        0
    }
}
