pub fn slugify(text: String) -> String {
    text.to_lowercase().split_whitespace().collect::<Vec<_>>().join("-")
}
