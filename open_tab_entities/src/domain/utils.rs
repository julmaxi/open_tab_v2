pub fn pad<E>(vec: Vec<E>, mask: &[bool]) -> Vec<Option<E>> {
    let mut out = vec![];
    let mut vals = vec.into_iter();
    for m in mask {
        if *m {
            if let Some(v) = vals.next() {
                out.push(Some(v));
            }
            else {
                out.push(None);
            }
        } else {
            out.push(None);
        }
    }
    out
}
