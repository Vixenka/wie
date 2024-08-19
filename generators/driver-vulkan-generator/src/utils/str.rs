pub fn from_screaming_snake_case_to_pascal_case(text: &str) -> String {
    let mut buf = String::new();
    let mut upper = true;
    for c in text.chars() {
        if c == '_' {
            upper = true
        } else if upper {
            buf.push(c);
            upper = false;
        } else {
            buf.push(c.to_lowercase().next().unwrap());
        }
    }
    buf
}
