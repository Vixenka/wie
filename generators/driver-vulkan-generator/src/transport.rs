use vk_parse::CommandParam;

use crate::{push_param_name, to_rust_type};

pub(crate) fn check_if_count_ptr(param: &CommandParam) -> bool {
    param.definition.name.starts_with('p') && param.definition.name.ends_with("Count")
}

pub(crate) fn write_packet_param(builder: &mut String, param: &CommandParam, is_response: bool) {
    let t = to_rust_type(&param.definition);
    match t.as_str() {
        "*const std::os::raw::c_char" => {
            builder.push_str("_null_str(");
            push_param_name(builder, param);
        }
        _ => {
            if t.starts_with("*mut") {
                if is_response {
                    builder.push_str("_raw_ptr(");
                } else {
                    builder.push_str("_nullable_raw_ptr_mut(");
                }
                push_param_name(builder, param);
            } else {
                builder.push('(');
                push_param_name(builder, param);
            }
        }
    }
    builder.push_str(");\n");
}

pub(crate) fn read_packet_param(builder: &mut String, param: &CommandParam, is_response: bool) {
    let t = to_rust_type(&param.definition);
    builder.push_str(match t.as_str() {
        "*const std::os::raw::c_char" => "_null_str",
        _ => {
            if t.starts_with("*mut") {
                if is_response {
                    "_to_raw_ptr"
                } else {
                    "_nullable_raw_ptr_mut"
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
}
