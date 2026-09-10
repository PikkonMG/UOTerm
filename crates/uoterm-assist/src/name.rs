/// The form a game name is looked up by: lower case, with only letters and
/// digits kept. "Nature's Fury", "natures fury" and "NaturesFury" are then the
/// same name, the way a player types them.
pub fn name_key(name: &str) -> String {
    name.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn case_spaces_and_marks_do_not_matter() {
        assert_eq!(name_key("Nature's Fury"), "naturesfury");
        assert_eq!(name_key("natures fury"), name_key("NaturesFury"));
    }
}
