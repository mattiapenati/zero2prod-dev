use unicode_segmentation::UnicodeSegmentation;

pub struct NewSubscriber {
    pub email: String,
    pub name: SubscriberName,
}

pub struct SubscriberName(String);

impl SubscriberName {
    pub fn parse(name: String) -> Self {
        const FORBIDDEN_CHARACTERS: [char; 9] = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];

        let is_empty_or_whitespace = name.trim().is_empty();
        let is_too_long = name.graphemes(true).count() > 256;
        let contains_forbidden_characters = name.chars().any(|c| FORBIDDEN_CHARACTERS.contains(&c));

        if is_empty_or_whitespace || is_too_long || contains_forbidden_characters {
            panic!("'{}' is not a valid subscriber name", name)
        } else {
            Self(name)
        }
    }
}
