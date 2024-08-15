use vk_parse::CommandParam;

use crate::{
    append_ptr_to_rust_type, push_param_name, to_rust_type_without_ptr, vulkan_types::TypeVulkan,
};

pub(crate) fn check_if_count_ptr(param: &CommandParam) -> bool {
    param.definition.name.starts_with('p') && param.definition.name.ends_with("Count")
}

pub(crate) fn write_packet_param(
    builder: &mut String,
    param: &CommandParam,
    _is_response: bool,
    types: &TypeVulkan,
) {
    fn write_impl(builder: &mut String, param: &CommandParam, function_name: &str) {
        builder.push_str(".write_");
        builder.push_str(function_name);
        builder.push('(');
        push_param_name(builder, param);
        builder.push_str(");\n");
    }

    let type_without_ptr = to_rust_type_without_ptr(&param.definition.type_name, types);
    let full_type =
        append_ptr_to_rust_type(type_without_ptr.clone(), &param.definition.code, types);

    // Process exceptions first
    if full_type == "*const std::os::raw::c_char" {
        write_impl(builder, param, "null_str");
        return;
    }

    // Check if type can be serialized via cdump
    if types.contains_type(&type_without_ptr) {
        write_impl(
            builder,
            param,
            match count_chars(&full_type, '*') {
                1 => "deep",
                2 => "deep_double",
                _ => unimplemented!("Unsupported pointer level"),
            },
        );
    } else if type_without_ptr.len() == full_type.len() {
        write_impl(builder, param, "shallow");
    } else {
        write_impl(builder, param, "shallow_under_nullable_ptr");
    }
}

/*pub(crate) fn write_packet_param(
    builder: &mut String,
    param: &CommandParam,
    is_response: bool,
    types: &TypeVulkan,
) {
    let t = to_rust_type(&param.definition, types);
    match t.as_str() {
        "*const std::os::raw::c_char" => {
            builder.push_str("_null_str(");
            push_param_name(builder, param);
        }
        _ => {
            let type_without_ptr = to_rust_type_without_ptr(&param.definition.type_name, types);
            if t.starts_with("*mut") {
                if is_response {
                    builder.push_str("_raw_ptr_as_shallow(");
                } else {
                    builder.push_str("_nullable_raw_ptr_mut_as_shallow(");
                }
                push_param_name(builder, param);
            } else if t.starts_with("*const") {
                if types.contains_type(&type_without_ptr) {
                    if count_chars(&t, '*') == 1 {
                        builder.push_str("_nullable_raw_ptr(")
                    } else {
                        builder.push_str("_nullable_double_raw_ptr(")
                    }
                } else if is_response {
                    builder.push_str("_raw_ptr_as_shallow(");
                } else {
                    builder.push_str("_nullable_raw_ptr_as_shallow(");
                }
                push_param_name(builder, param);
            } else {
                builder.push_str("_as_shallow(");
                push_param_name(builder, param);
            }
        }
    }
    builder.push_str(");\n");
}*/

pub(crate) fn read_packet_param(
    builder: &mut String,
    param: &CommandParam,
    is_response: bool,
    types: &TypeVulkan,
) {
    fn write_impl(
        builder: &mut String,
        param: &CommandParam,
        function_name: &str,
        is_response: bool,
        is_mut: bool,
    ) {
        builder.push_str(".read_");

        if is_mut {
            builder.push_str("mut_");
        }

        builder.push_str(function_name);

        if is_response {
            builder.push_str("_at(");
            push_param_name(builder, param);
        } else {
            builder.push('(');
        }

        builder.push_str(");\n");
    }

    let type_without_ptr = to_rust_type_without_ptr(&param.definition.type_name, types);
    let full_type =
        append_ptr_to_rust_type(type_without_ptr.clone(), &param.definition.code, types);

    // Process exceptions first
    if full_type == "*const std::os::raw::c_char" {
        write_impl(builder, param, "null_str", is_response, false);
        return;
    }

    let is_mut = full_type.starts_with("*mut");

    // Check if type can be serialized via cdump
    if types.contains_type(&type_without_ptr) {
        write_impl(
            builder,
            param,
            match count_chars(&full_type, '*') {
                1 => "deep",
                2 => "deep_double",
                _ => unimplemented!("Unsupported pointer level"),
            },
            is_response,
            is_mut,
        );
    } else if type_without_ptr.len() == full_type.len() {
        write_impl(builder, param, "shallow", is_response, is_mut);
    } else {
        write_impl(
            builder,
            param,
            "shallow_under_nullable_ptr",
            is_response,
            is_mut,
        );
    }
}

/*pub(crate) fn read_packet_param(
    builder: &mut String,
    param: &CommandParam,
    is_response: bool,
    types: &TypeVulkan,
) {
    let t = to_rust_type(&param.definition, types);
    builder.push_str(match t.as_str() {
        "*const std::os::raw::c_char" => "_null_str",
        _ => {
            if t.starts_with("*mut") {
                if is_response {
                    "_to_raw_ptr"
                } else {
                    "_nullable_raw_ptr_mut"
                }
            } else if t.starts_with("*const") {
                if is_response {
                    "_to_raw_ptr"
                } else {
                    "_nullable_raw_ptr"
                }
            } else {
                ""
            }
        }
    });

    builder.push('(');
    if is_response {
        push_param_name(builder, param);
    }
    builder.push_str(");\n");
}*/

fn count_chars(s: &str, c: char) -> usize {
    s.chars().filter(|&x| x == c).count()
}
