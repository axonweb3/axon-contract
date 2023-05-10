use case::CaseExt;
use proc_macro2 as m4;

pub(super) fn usize_lit(num: usize) -> m4::Literal {
    m4::Literal::usize_unsuffixed(num)
}

pub(super) fn ident_new(ident: &str) -> m4::Ident {
    m4::Ident::new(ident, m4::Span::call_site())
}

pub(super) fn ident_name(name: &str, suffix: &str) -> m4::Ident {
    let span = m4::Span::call_site();
    m4::Ident::new(&format!("{}{}", name, suffix).to_camel(), span)
}

pub(super) fn entity_name(name: &str) -> m4::Ident {
    ident_name(name, "")
}

pub(super) fn reader_name(name: &str) -> m4::Ident {
    ident_name(name, "Reader")
}

pub(super) fn entity_union_name(name: &str) -> m4::Ident {
    ident_name(name, "Union")
}

pub(super) fn reader_union_name(name: &str) -> m4::Ident {
    ident_name(name, "UnionReader")
}

pub(super) fn union_item_name(name: &str) -> m4::Ident {
    ident_name(name, "")
}

pub(super) fn builder_name(name: &str) -> m4::Ident {
    ident_name(name, "Builder")
}

pub(super) fn field_name(name: &str) -> m4::Ident {
    let span = m4::Span::call_site();
    m4::Ident::new(&name.to_snake(), span)
}

pub(super) fn func_name(name: &str) -> m4::Ident {
    let span = m4::Span::call_site();
    m4::Ident::new(&name.to_snake(), span)
}

pub(super) fn entity_iterator_name(name: &str) -> m4::Ident {
    ident_name(name, "Iterator")
}

pub(super) fn reader_iterator_name(name: &str) -> m4::Ident {
    ident_name(name, "ReaderIterator")
}
