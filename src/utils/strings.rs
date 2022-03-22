pub fn iterable_to_string<'a, I>(data: &mut I) -> String
where
    I: Iterator<Item = &'a u8>,
{
    let mut on = true;
    data.fold(String::new(), |mut acc, &cur| {
        if cur == 0 {
            on = false; // null byte means the string is done, discard the rest of the iterator
        }
        if on {
            acc.push(cur as char);
        }
        acc
    })
}

pub fn iterable_to_string_no_truncate<'a, I>(data: &mut I) -> String
where
    I: Iterator<Item = &'a u8>,
{
    data.fold(String::new(), |mut acc, &cur| {
        acc.push(cur as char);
        acc
    })
}
