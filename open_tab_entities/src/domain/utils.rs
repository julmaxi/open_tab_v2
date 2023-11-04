pub fn pad<E>(vec: Vec<E>, mask: &[bool]) -> Vec<Option<E>> {
    let mut out = vec![];
    let mut it = vec.into_iter();
    let mut mask = mask.iter();
    for v in it {
        if let Some(&true) = mask.next() {
            out.push(Some(v));
        } else {
            out.push(None);
        }
    }
    out
}
