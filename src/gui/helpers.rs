use std::borrow::Cow;

pub fn ellipsize_middle(s: &'_ str, max_len: usize) -> Cow<'_, str> {
    if s.len() <= max_len {
        Cow::Borrowed(s)
    } else {
        let part_len = (max_len - 3) / 2;
        let start_part = &s[..part_len].trim_end();
        let end_part = &s[s.len() - part_len..].trim_start();

        Cow::Owned(format!("{}...{}", start_part, end_part))
    }
}
