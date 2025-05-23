use facet_derive_parse::*;
use std::borrow::Cow;

mod generics;
pub use generics::*;

mod process_enum;
mod process_struct;

/// Removes the `r#` prefix from a raw identifier string, if present.
fn normalize_ident_str(ident_str: &str) -> &str {
    ident_str.strip_prefix("r#").unwrap_or(ident_str)
}

pub fn facet_derive(input: TokenStream) -> TokenStream {
    let mut i = input.to_token_iter();

    // Parse as TypeDecl
    match i.parse::<Cons<AdtDecl, EndOfStream>>() {
        Ok(it) => match it.first {
            AdtDecl::Struct(parsed) => process_struct::process_struct(parsed),
            AdtDecl::Enum(parsed) => process_enum::process_enum(parsed),
        },
        Err(err) => {
            panic!(
                "Could not parse type declaration: {}\nError: {}",
                input, err
            );
        }
    }
}

pub(crate) struct ContainerAttributes {
    pub code: String,
    pub rename_rule: RenameRule,
}

/// Represents different case conversion strategies for renaming
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RenameRule {
    /// Rename to lowercase: `FooBar` -> `foobar`
    Lowercase,
    /// Rename to UPPERCASE: `FooBar` -> `FOOBAR`
    Uppercase,
    /// Rename to PascalCase: `foo_bar` -> `FooBar`
    PascalCase,
    /// Rename to camelCase: `foo_bar` -> `fooBar`
    CamelCase,
    /// Rename to snake_case: `FooBar` -> `foo_bar`
    SnakeCase,
    /// Rename to SCREAMING_SNAKE_CASE: `FooBar` -> `FOO_BAR`
    ScreamingSnakeCase,
    /// Rename to kebab-case: `FooBar` -> `foo-bar`
    KebabCase,
    /// Rename to SCREAMING-KEBAB-CASE: `FooBar` -> `FOO-BAR`
    ScreamingKebabCase,
    /// No renaming applied.
    Passthrough,
}

impl RenameRule {
    /// Parse a string into a `RenameRule`
    pub(crate) fn from_str(rule: &str) -> Option<Self> {
        match rule {
            "lowercase" => Some(RenameRule::Lowercase),
            "UPPERCASE" => Some(RenameRule::Uppercase),
            "PascalCase" => Some(RenameRule::PascalCase),
            "camelCase" => Some(RenameRule::CamelCase),
            "snake_case" => Some(RenameRule::SnakeCase),
            "SCREAMING_SNAKE_CASE" => Some(RenameRule::ScreamingSnakeCase),
            "kebab-case" => Some(RenameRule::KebabCase),
            "SCREAMING-KEBAB-CASE" => Some(RenameRule::ScreamingKebabCase),
            _ => None,
        }
    }

    /// Apply this renaming rule to a string
    pub(crate) fn apply(self, input: &str) -> String {
        match self {
            RenameRule::Lowercase => to_lowercase(input),
            RenameRule::Uppercase => to_uppercase(input),
            RenameRule::PascalCase => to_pascal_case(input),
            RenameRule::CamelCase => to_camel_case(input),
            RenameRule::SnakeCase => to_snake_case(input),
            RenameRule::ScreamingSnakeCase => to_screaming_snake_case(input),
            RenameRule::KebabCase => to_kebab_case(input),
            RenameRule::ScreamingKebabCase => to_screaming_kebab_case(input),
            RenameRule::Passthrough => input.to_string(),
        }
    }
}

/// Converts a string to lowercase
pub(crate) fn to_lowercase(input: &str) -> String {
    input.to_lowercase()
}

/// Converts a string to UPPERCASE
pub(crate) fn to_uppercase(input: &str) -> String {
    input.to_uppercase()
}

/// Splits a string into words based on case and separators
fn split_into_words(input: &str) -> Vec<String> {
    if input.is_empty() {
        return vec![];
    }

    let mut words = Vec::new();
    let mut current_word = String::new();
    let mut prev_is_lowercase = false;
    let mut prev_is_uppercase = false;
    // Removed prev_is_separator as it was unused

    for c in input.chars() {
        if c == '_' || c == '-' || c.is_whitespace() {
            if !current_word.is_empty() {
                words.push(std::mem::take(&mut current_word));
            }
            // Reset state for the next word
            prev_is_lowercase = false;
            prev_is_uppercase = false;
        } else if c.is_uppercase() {
            // Start a new word if:
            // 1. The previous character was lowercase (e.g., 'aB')
            // 2. The previous character was uppercase AND the character *after* the current one is lowercase
            //    (to handle acronyms like 'HTTPRequest' -> 'HTTP', 'Request')
            // And also ensure we don't push an empty word if the input starts with uppercase letters.
            let next_char_is_lowercase = input
                .chars()
                .skip_while(|&x| x != c)
                .nth(1)
                .is_some_and(|next| next.is_lowercase());

            if !current_word.is_empty()
                && (prev_is_lowercase || (prev_is_uppercase && next_char_is_lowercase))
            {
                words.push(std::mem::take(&mut current_word));
            }

            current_word.push(c);
            prev_is_uppercase = true;
            prev_is_lowercase = false;
        } else {
            // The current character is lowercase or digit
            // If the previous char was uppercase, we might need to start a new word
            // Example: 'CamelCase' -> 'Camel', 'Case'
            // But not for the first character of the string if it's lowercase
            if prev_is_uppercase && !current_word.chars().all(|ch| ch.is_uppercase()) {
                // This condition handles cases like 'HTTPRequest' where the last uppercase
                // belongs to the previous word 'HTTP'. If the `current_word` contains
                // lowercase it means we already split, e.g., 'CamelCase' -> 'Camel', 'c'.
                // Instead, we handle the split *before* adding the lowercase char.
                // Let's adjust the logic slightly. We split when transitioning from U->L
                // *except* when the word is currently just a sequence of uppercase (like 'HTTP' in 'HTTPRequest').
                // This seems overly complex. Let's stick to the original logic but refine the condition.
                // The split should happen *before* pushing the lowercase character `c`.

                // Correct logic: If transitioning from Uppercase to Lowercase,
                // split off the last uppercase character into the new word,
                // unless it's a sequence like 'HTTPReq'.
                // Let's rethink the original condition: `prev_is_uppercase` meant the *last* char was uppercase.
                // If `c` is lowercase, and `prev_is_uppercase` is true,
                // we need to check if `current_word` has more than one char.
                // If 'APIResponse', when we see 'R', current='API', prev_is_upper=true. We push 'R'. current='APIR'.
                // When we see 'e', current='APIR', prev_is_upper=true. We push 'e'. current='APIRe'. This is wrong.

                // Let's revert to a simpler split logic inspired by `heck` crate:
                // Split happens before an uppercase letter if the previous was lowercase.
                // Split happens before an uppercase letter if the *next* letter is lowercase (e.g. ApinRequest -> Api, Request)

                // The existing logic for uppercase `c` handles the `aB` and `ABCd` cases.
                // Now handle the lowercase `c`. If the previous was uppercase, it's usually
                // part of the same word unless we detected an acronym boundary already.
                // The original code didn't explicitly split on U -> L transition, relying on the L -> U split.
                // Let's stick to that for now.
            }
            // If previous was uppercase, and current is lowercase, the word boundary was already handled
            // when the uppercase char was processed (e.g. in 'PascalCase', the split happens *before* 'C').
            current_word.push(c);
            prev_is_lowercase = true;
            prev_is_uppercase = false;
        }
    }

    if !current_word.is_empty() {
        words.push(current_word);
    }

    // Filter out empty strings that might result from multiple separators
    words.into_iter().filter(|s| !s.is_empty()).collect()
}

/// Converts a string to PascalCase: `foo_bar` -> `FooBar`
pub(crate) fn to_pascal_case(input: &str) -> String {
    split_into_words(input)
        .iter()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => {
                    c.to_uppercase().collect::<String>() + &chars.collect::<String>().to_lowercase()
                }
            }
        })
        .collect()
}

/// Converts a string to camelCase: `foo_bar` -> `fooBar`
pub(crate) fn to_camel_case(input: &str) -> String {
    let pascal = to_pascal_case(input);
    if pascal.is_empty() {
        return String::new();
    }

    let mut result = String::new();
    let mut chars = pascal.chars();
    if let Some(first_char) = chars.next() {
        result.push(first_char.to_lowercase().next().unwrap());
    }
    result.extend(chars);
    result
}

/// Converts a string to snake_case: `FooBar` -> `foo_bar`
pub(crate) fn to_snake_case(input: &str) -> String {
    let words = split_into_words(input);
    words
        .iter()
        .map(|word| word.to_lowercase())
        .collect::<Vec<_>>()
        .join("_")
}

/// Converts a string to SCREAMING_SNAKE_CASE: `FooBar` -> `FOO_BAR`
pub(crate) fn to_screaming_snake_case(input: &str) -> String {
    let words = split_into_words(input);
    words
        .iter()
        .map(|word| word.to_uppercase())
        .collect::<Vec<_>>()
        .join("_")
}

/// Converts a string to kebab-case: `FooBar` -> `foo-bar`
pub(crate) fn to_kebab_case(input: &str) -> String {
    let words = split_into_words(input);
    words
        .iter()
        .map(|word| word.to_lowercase())
        .collect::<Vec<_>>()
        .join("-")
}

/// Converts a string to SCREAMING-KEBAB-CASE: `FooBar` -> `FOO-BAR`
pub(crate) fn to_screaming_kebab_case(input: &str) -> String {
    let words = split_into_words(input);
    words
        .iter()
        .map(|word| word.to_uppercase())
        .collect::<Vec<_>>()
        .join("-")
}

/// Converts PascalCase to UPPER_SNAKE_CASE
pub(crate) fn to_upper_snake_case(input: &str) -> String {
    input
        .chars()
        .enumerate()
        .fold(String::new(), |mut acc, (i, c)| {
            if c.is_uppercase() {
                if i > 0 {
                    acc.push('_');
                }
                acc.push(c.to_ascii_uppercase());
            } else {
                acc.push(c.to_ascii_uppercase());
            }
            acc
        })
}

/// Generate a static declaration that exports the crate
pub(crate) fn generate_static_decl(type_name: &str) -> String {
    format!(
        "#[used]\nstatic {}_SHAPE: &'static ::facet::Shape = <{} as ::facet::Facet>::SHAPE;",
        to_upper_snake_case(type_name),
        type_name
    )
}

pub(crate) fn build_maybe_doc(attrs: &[Attribute]) -> String {
    let doc_lines: Vec<_> = attrs
        .iter()
        .filter_map(|attr| match &attr.body.content {
            AttributeInner::Doc(doc_inner) => Some(doc_inner.value.value()),
            _ => None,
        })
        .collect();

    if doc_lines.is_empty() {
        String::new()
    } else {
        format!(r#".doc(&[{}])"#, doc_lines.join(","))
    }
}

struct FieldInfo<'a> {
    /// something like `r#type`
    raw_field_name: &'a str,

    /// something like `type`
    normalized_field_name: &'a str,

    /// something like `String`
    field_type: &'a str,

    /// something like `Person`
    struct_name: &'a str,

    /// the bounded generic params for the container/struct
    bgp: &'a BoundedGenericParams,

    /// the attributes for that field
    attrs: &'a [Attribute],

    /// the base field offset — for structs it's always zero, for
    /// enums it depends on the variant/discriminant
    base_field_offset: Option<&'a str>,

    /// the rename rule to use for the container
    rename_rule: RenameRule,
}

/// Generates field definitions for a struct
///
/// `base_field_offset` applies a shift to the field offset, which is useful for
/// generating fields for a struct that is part of a #[repr(C)] enum.
pub(crate) fn gen_struct_field<'a>(fi: FieldInfo<'a>) -> String {
    // Determine field flags
    let mut flags = "::facet::FieldFlags::EMPTY";
    let mut attribute_list: Vec<String> = vec![];
    let mut doc_lines: Vec<&str> = vec![];
    let mut shape_of = "shape_of";
    // Start with the normalized name, potentially overridden by `rename`
    let mut name_for_metadata: Cow<'a, str> = Cow::Borrowed(fi.normalized_field_name);
    let mut has_explicit_rename = false;
    for attr in fi.attrs {
        match &attr.body.content {
            AttributeInner::Facet(facet_attr) => match &facet_attr.inner.content {
                FacetInner::Sensitive(_ksensitive) => {
                    flags = "::facet::FieldFlags::SENSITIVE";
                    attribute_list.push("::facet::FieldAttribute::Sensitive".to_string());
                }
                FacetInner::Default(_) => {
                    attribute_list.push("::facet::FieldAttribute::Default(None)".to_string());
                }
                FacetInner::DefaultEquals(inner) => {
                    attribute_list.push(format!(
                        r#"::facet::FieldAttribute::Default(Some(|ptr| {{
                            unsafe {{ ptr.put({}()) }}
                        }}))"#,
                        inner
                            .value
                            .value()
                            .trim_start_matches('"')
                            .trim_end_matches('"') // FIXME: that's pretty bad
                    ));
                }
                FacetInner::Transparent(_) => {
                    // Not applicable on fields; ignore.
                }
                FacetInner::Invariants(_invariant_inner) => {
                    panic!("fields cannot have invariants")
                }
                FacetInner::Opaque(_kopaque) => {
                    shape_of = "shape_of_opaque";
                }
                FacetInner::DenyUnknownFields(_) => {
                    // not applicable on fields
                }
                FacetInner::RenameAll(_) => {
                    // not applicable on fields
                }
                FacetInner::Other(tt) => {
                    let attr_str = tt.tokens_to_string();

                    // Split the attributes by commas to handle multiple attributes
                    let attrs = attr_str.split(',').map(|s| s.trim()).collect::<Vec<_>>();

                    for attr in attrs {
                        if let Some(equal_pos) = attr.find('=') {
                            let key = attr[..equal_pos].trim();
                            if key == "rename" {
                                has_explicit_rename = true;
                                let value = attr[equal_pos + 1..].trim().trim_matches('"');
                                // Use the renamed value for metadata name
                                name_for_metadata = Cow::Owned(value.to_string());
                                // Keep the Rename attribute for reflection
                                attribute_list.push(format!(
                                    r#"::facet::FieldAttribute::Rename({:?})"#,
                                    value
                                ));
                            } else if key == "skip_serializing_if" {
                                let value = attr[equal_pos + 1..].trim();
                                attribute_list.push(format!(
                                    r#"::facet::FieldAttribute::SkipSerializingIf(unsafe {{ ::std::mem::transmute({value} as fn(&{field_type}) -> bool) }})"#, field_type = fi.field_type
                                ));
                            } else {
                                attribute_list.push(format!(
                                    r#"::facet::FieldAttribute::Arbitrary({:?})"#,
                                    attr
                                ));
                            }
                        } else if attr == "skip_serializing" {
                            attribute_list
                                .push(r#"::facet::FieldAttribute::SkipSerializing"#.to_string());
                        } else if attr == "sensitive" {
                            flags = "::facet::FieldFlags::SENSITIVE";
                            attribute_list.push("::facet::FieldAttribute::Sensitive".to_string());
                        } else {
                            attribute_list
                                .push(format!(r#"::facet::FieldAttribute::Arbitrary({:?})"#, attr));
                        }
                    }
                }
            },
            AttributeInner::Doc(doc_inner) => doc_lines.push(doc_inner.value.value()),
            AttributeInner::Repr(_) => {
                // muffin
            }
            AttributeInner::Any(_) => {
                // muffin two
            }
        }
    }

    // Apply rename_all rule if there's no explicit rename attribute
    if !has_explicit_rename && fi.rename_rule != RenameRule::Passthrough {
        // Only apply to named fields (not tuple indices)
        if !fi.normalized_field_name.chars().all(|c| c.is_ascii_digit()) {
            let renamed = fi.rename_rule.apply(fi.normalized_field_name);
            attribute_list.push(format!(r#"::facet::FieldAttribute::Rename({:?})"#, renamed));
            name_for_metadata = Cow::Owned(renamed);
        }
    }

    let attributes = attribute_list.join(",");

    let maybe_field_doc = if doc_lines.is_empty() {
        String::new()
    } else {
        format!(r#".doc(&[{}])"#, doc_lines.join(","))
    };

    let maybe_base_field_offset = fi
        .base_field_offset
        .map(|offset| format!(" + {offset}"))
        .unwrap_or_default();

    // Generate each field definition
    format!(
        "::facet::Field::builder()
            .name(\"{name_for_metadata}\")
            .shape(|| ::facet::{shape_of}(&|s: &{struct_name}{bgp}| &s.{raw_field_name}))
            .offset(::core::mem::offset_of!({struct_name}{bgp}, {raw_field_name}){maybe_base_field_offset})
            .flags({flags})
            .attributes(&const {{ [{attributes}] }})
            {maybe_field_doc}
            .build()",
        struct_name = fi.struct_name,
        raw_field_name = fi.raw_field_name,
        bgp = fi.bgp.display_without_bounds()
    )
}

fn build_where_clauses(
    where_clauses: Option<&WhereClauses>,
    generics: Option<&GenericParams>,
) -> String {
    let mut where_clauses_s: Vec<String> = vec![];
    if let Some(wc) = where_clauses {
        for c in &wc.clauses.0 {
            where_clauses_s.push(c.value.to_string())
        }
    }

    if let Some(generics) = generics {
        for p in &generics.params.0 {
            match &p.value {
                GenericParam::Lifetime { name, .. } => {
                    where_clauses_s.push(format!("{name}: '__facet"));
                    where_clauses_s.push(format!("'__facet: {name}"));
                }
                GenericParam::Const { .. } => {
                    // ignore for now
                }
                GenericParam::Type { name, .. } => {
                    where_clauses_s.push(format!("{name}: ::facet::Facet<'__facet>"));
                }
            }
        }
    }

    if where_clauses_s.is_empty() {
        "".to_string()
    } else {
        format!("where {}", where_clauses_s.join(", "))
    }
}

fn build_type_params(generics: Option<&GenericParams>) -> String {
    let mut type_params_s: Vec<String> = vec![];
    if let Some(generics) = generics {
        for p in &generics.params.0 {
            match &p.value {
                GenericParam::Lifetime { .. } => {
                    // ignore for now
                }
                GenericParam::Const { .. } => {
                    // ignore for now
                }
                GenericParam::Type { name, .. } => {
                    type_params_s.push(format!(
                        "::facet::TypeParam {{ name: {:?}, shape: || <{name} as ::facet::Facet>::SHAPE }}",
                        // debug fmt because we want it to be quoted & escaped, but to_string because we don't want the `Ident { .. }`
                        name.to_string()
                    ));
                }
            }
        }
    }

    if type_params_s.is_empty() {
        "".to_string()
    } else {
        format!(".type_params(&[{}])", type_params_s.join(", "))
    }
}

fn build_container_attributes(attributes: &[Attribute]) -> ContainerAttributes {
    let mut items: Vec<Cow<str>> = vec![];
    let mut rename_all_rule: Option<RenameRule> = None;

    for attr in attributes {
        match &attr.body.content {
            AttributeInner::Facet(facet_attr) => match &facet_attr.inner.content {
                FacetInner::DenyUnknownFields(_) => {
                    items.push("::facet::ShapeAttribute::DenyUnknownFields".into());
                }
                FacetInner::DefaultEquals(_) | FacetInner::Default(_) => {
                    items.push("::facet::ShapeAttribute::Default".into());
                }
                FacetInner::Transparent(_) => {
                    items.push("::facet::ShapeAttribute::Transparent".into());
                }
                FacetInner::RenameAll(rename_all_inner) => {
                    let rule_str = rename_all_inner.value.value().trim_matches('"');
                    if let Some(rule) = RenameRule::from_str(rule_str) {
                        rename_all_rule = Some(rule);
                        items.push(
                            format!(r#"::facet::ShapeAttribute::RenameAll({:?})"#, rule_str).into(),
                        );
                    }
                }
                FacetInner::Sensitive(_) => {
                    // TODO
                }
                FacetInner::Invariants(_) => {
                    // dealt with elsewhere
                }
                FacetInner::Opaque(_) => {
                    // TODO
                }
                FacetInner::Other(other) => {
                    let attr_str = other.tokens_to_string();
                    if let Some(equal_pos) = attr_str.find('=') {
                        let key = attr_str[..equal_pos].trim();
                        if key == "rename_all" {
                            let value = attr_str[equal_pos + 1..].trim().trim_matches('"');
                            if let Some(rule) = RenameRule::from_str(value) {
                                rename_all_rule = Some(rule);
                                items.push(
                                    format!(r#"::facet::ShapeAttribute::RenameAll({:?})"#, value)
                                        .into(),
                                );
                            }
                        } else {
                            items.push(
                                format!(
                                    r#"::facet::ShapeAttribute::Arbitrary({:?})"#,
                                    other.tokens_to_string()
                                )
                                .into(),
                            );
                        }
                    } else {
                        items.push(
                            format!(
                                r#"::facet::ShapeAttribute::Arbitrary({:?})"#,
                                other.tokens_to_string()
                            )
                            .into(),
                        );
                    }
                }
            },
            _ => {
                // do nothing.
            }
        }
    }

    let attributes_string = if items.is_empty() {
        "".to_string()
    } else {
        format!(".attributes(&[{}])", items.join(", "))
    };

    ContainerAttributes {
        code: attributes_string,
        rename_rule: rename_all_rule.unwrap_or(RenameRule::Passthrough),
    }
}

fn get_discriminant_value(lit: &Literal) -> i64 {
    let s = lit.to_string();
    get_discriminant_value_from_str(&s)
}

fn strip_underscores(s: &str) -> Cow<str> {
    if s.contains('_') {
        Cow::Owned(s.chars().filter(|&c| c != '_').collect())
    } else {
        Cow::Borrowed(s)
    }
}

fn get_discriminant_value_from_str(s: &str) -> i64 {
    let s = s.trim();

    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        let hex = strip_underscores(hex);
        i64::from_str_radix(&hex, 16).expect("Invalid hex literal for discriminant")
    } else if let Some(bin) = s.strip_prefix("0b").or_else(|| s.strip_prefix("0B")) {
        let bin = strip_underscores(bin);
        i64::from_str_radix(&bin, 2).expect("Invalid binary literal for discriminant")
    } else if let Some(oct) = s.strip_prefix("0o").or_else(|| s.strip_prefix("0O")) {
        let oct = strip_underscores(oct);
        i64::from_str_radix(&oct, 8).expect("Invalid octal literal for discriminant")
    } else {
        // Plain decimal. Support optional _ separators (Rust literals)
        let parsed = strip_underscores(s);
        parsed
            .parse::<i64>()
            .expect("Invalid decimal literal for discriminant")
    }
}

#[cfg(test)]
mod tests {
    use super::get_discriminant_value_from_str;

    #[test]
    fn test_decimal_discriminants() {
        assert_eq!(get_discriminant_value_from_str("7"), 7);
        assert_eq!(get_discriminant_value_from_str("10"), 10);
        assert_eq!(get_discriminant_value_from_str("123_456"), 123456);
        assert_eq!(get_discriminant_value_from_str(" 42 "), 42);
    }

    #[test]
    fn test_hex_discriminants() {
        assert_eq!(get_discriminant_value_from_str("0x01"), 1);
        assert_eq!(get_discriminant_value_from_str("0x7F"), 127);
        assert_eq!(get_discriminant_value_from_str("0x80"), 128);
        assert_eq!(get_discriminant_value_from_str("0x10"), 16);
        assert_eq!(get_discriminant_value_from_str("0xfeed"), 0xfeed);
        assert_eq!(get_discriminant_value_from_str("0xBEEF"), 0xBEEF);
        assert_eq!(get_discriminant_value_from_str("0xBE_EF"), 0xBEEF);
        assert_eq!(get_discriminant_value_from_str("0X1A"), 26);
    }

    #[test]
    fn test_binary_discriminants() {
        assert_eq!(get_discriminant_value_from_str("0b0000_0000"), 0);
        assert_eq!(get_discriminant_value_from_str("0b0000_0001"), 1);
        assert_eq!(get_discriminant_value_from_str("0b0000_0010"), 2);
        assert_eq!(get_discriminant_value_from_str("0b0000_0100"), 4);
        assert_eq!(get_discriminant_value_from_str("0b0000_0111"), 7);
        assert_eq!(get_discriminant_value_from_str("0B1011"), 11);
    }

    #[test]
    fn test_octal_discriminants() {
        assert_eq!(get_discriminant_value_from_str("0o77"), 63);
        assert_eq!(get_discriminant_value_from_str("0o077"), 63);
        assert_eq!(get_discriminant_value_from_str("0o123"), 83);
        assert_eq!(get_discriminant_value_from_str("0o1_234"), 668);
        assert_eq!(get_discriminant_value_from_str("0O345"), 229);
    }

    #[test]
    fn test_mixed_notations() {
        assert_eq!(get_discriminant_value_from_str("1"), 1);
        assert_eq!(get_discriminant_value_from_str("0xA"), 10);
        assert_eq!(get_discriminant_value_from_str("0b1111"), 15);
        assert_eq!(get_discriminant_value_from_str("0o77"), 63);
    }
}
