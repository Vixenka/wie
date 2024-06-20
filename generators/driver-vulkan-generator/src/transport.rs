use vk_parse::CommandParam;

use crate::{push_param_name, to_rust_type};

pub(crate) fn check_if_count_ptr(param: &CommandParam) -> bool {
    param.definition.name.starts_with('p') && param.definition.name.ends_with("Count")
}

pub(crate) fn write_packet_param(
    builder: &mut String,
    param: &CommandParam,
    last_is_count: bool,
    is_count: bool,
) {
    if last_is_count {
        builder.push_str("_is_null_ptr(");
        push_param_name(builder, param);
        builder.push_str(");");
        return;
    }

    let t = to_rust_type(&param.definition);
    match t.as_str() {
        "*const std::os::raw::c_char" => {
            builder.push_str("_null_str(");
            push_param_name(builder, param);
        }
        _ => {
            if is_count && t.starts_with("*mut") {
                builder.push_str("_nullable_raw_ptr_mut(");
                push_param_name(builder, param);
            } else {
                builder.push('(');
                push_param_name(builder, param);
            }
        }
    }
    builder.push_str(");\n");
}

pub(crate) fn read_packet_param(
    builder: &mut String,
    param: &CommandParam,
    last_is_count: bool,
    is_count: bool,
) {
    if last_is_count {
        builder.push_str("_is_null_ptr");
    } else {
        let t = to_rust_type(&param.definition);
        builder.push_str(match t.as_str() {
            "*const std::os::raw::c_char" => "_null_str",
            _ => {
                if is_count && t.starts_with("*mut") {
                    "_nullable_raw_ptr_mut"
                } else {
                    ""
                }
            }
        });
    }
    builder.push_str("();\n");
}
