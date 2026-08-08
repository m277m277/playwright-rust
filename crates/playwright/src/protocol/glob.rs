// URL glob matching, shared by client-side waits (`Frame::wait_for_url`) and
// client-side route matching (`Page::route`, `BrowserContext::route`).
//
// This is a port of the driver's `globToRegexPattern`
// (`packages/isomorphic/urlMatch.ts`), not an independent implementation.
// Both sides have to agree on whether a URL matches: the server decides
// which requests to report, and the client decides which handler to run, so
// a pattern the two read differently means a route event nobody answers and
// a request that hangs until it times out.

/// Characters the driver escapes when translating a glob to a regex.
///
/// Kept as the driver's exact set rather than deferring to `regex::escape`,
/// because `{`, `}` and `,` are glob syntax here (alternation) and must not
/// be escaped, and because a wider set would silently diverge.
const ESCAPED_CHARS: [char; 14] = [
    '$', '^', '+', '.', '*', '(', ')', '|', '\\', '?', '{', '}', '[', ']',
];

fn push_escaped(out: &mut String, c: char) {
    if ESCAPED_CHARS.contains(&c) {
        out.push('\\');
    }
    out.push(c);
}

/// Translates a Playwright URL glob into an anchored regex pattern.
///
/// Ported from the driver so the two stay in step:
///
/// - `\x` is an escape; the next character is always literal.
/// - `*` matches within one path segment.
/// - `**` crosses segments. Between slashes (`/**/`) it also matches *zero*
///   directories, so `a/**/b` matches `a/b`.
/// - `{a,b}` is alternation. Nested or unbalanced braces are an error.
/// - Everything else is literal.
///
/// Returns `None` for a malformed pattern (nested `{`, unmatched `}` or `{`),
/// which the driver rejects by throwing.
pub(crate) fn glob_to_regex_pattern(glob: &str) -> Option<String> {
    let mut out = String::with_capacity(glob.len() + 2);
    out.push('^');

    let mut in_group = false;
    // The driver indexes the glob directly and reads `glob[i - 1]` for the
    // character before a `*` run. Walking with an iterator instead means
    // carrying that character forward, which costs one variable and makes an
    // off-by-one or a non-advancing loop impossible to write.
    let mut previous: Option<char> = None;
    let mut chars = glob.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            // A trailing lone backslash has nothing to escape, so it is
            // literal; the driver reaches the same place via its `i + 1 <
            // length` guard falling through to the default branch.
            let literal = chars.next().unwrap_or('\\');
            push_escaped(&mut out, literal);
            previous = Some(literal);
            continue;
        }

        if c == '*' {
            let char_before = previous;
            // Consume the rest of the run with a bounded construct rather
            // than a `while peek() == '*'`: an exhausted iterator yields
            // `None` forever, so a loop keyed on that comparison can spin
            // rather than end. `next_if_eq` stops the moment it does not
            // match, which cannot.
            let crosses_segments = std::iter::from_fn(|| chars.next_if_eq(&'*')).count() > 0;

            if crosses_segments && chars.peek() == Some(&'/') {
                chars.next();
                if char_before == Some('/') {
                    // `/**/` also matches zero directories.
                    out.push_str("((.+/)|)");
                } else {
                    out.push_str("(.*/)");
                }
                previous = Some('/');
            } else {
                if crosses_segments {
                    out.push_str("(.*)");
                } else {
                    out.push_str("([^/]*)");
                }
                previous = Some('*');
            }
            continue;
        }

        match c {
            '{' => {
                if in_group {
                    return None;
                }
                in_group = true;
                out.push('(');
            }
            '}' => {
                if !in_group {
                    return None;
                }
                in_group = false;
                out.push(')');
            }
            ',' if in_group => out.push('|'),
            other => push_escaped(&mut out, other),
        }
        previous = Some(c);
    }

    if in_group {
        return None;
    }

    out.push('$');
    Some(out)
}

/// Returns whether `text` matches the glob `pattern`.
///
/// Returns `false` for a malformed pattern, matching the driver's behaviour
/// of treating it as matching nothing rather than panicking.
///
/// Callers that match repeatedly against one pattern should compile it once
/// with [`GlobMatcher::new`] instead.
pub(crate) fn glob_match(pattern: &str, text: &str) -> bool {
    GlobMatcher::new(pattern).is_some_and(|m| m.matches(text))
}

/// A glob compiled once, for callers that match the same pattern repeatedly.
///
/// `Frame::wait_for_url` polls every 50ms, so compiling per call meant
/// hundreds of identical regex compilations for a single wait.
pub(crate) struct GlobMatcher {
    regex: regex::Regex,
}

impl GlobMatcher {
    /// Compiles `pattern`, returning `None` if it is malformed.
    pub(crate) fn new(pattern: &str) -> Option<Self> {
        let regex_pattern = glob_to_regex_pattern(pattern)?;
        regex::Regex::new(&regex_pattern)
            .ok()
            .map(|regex| Self { regex })
    }

    pub(crate) fn matches(&self, text: &str) -> bool {
        self.regex.is_match(text)
    }
}

#[cfg(test)]
mod properties {
    use super::glob_match;
    use proptest::prelude::*;

    /// URL-ish text containing no glob metacharacter, over a deliberately
    /// narrow alphabet that nonetheless includes the regex metacharacters a
    /// real URL carries (`?`, `+`, `(`, `[`, ...).
    ///
    /// The alphabet is the point. Identifier-shaped input is already covered
    /// by the example tests below, so letting the generator roam there would
    /// rediscover those instead of exercising the escaping.
    fn url_atom() -> impl Strategy<Value = String> {
        proptest::collection::vec(
            proptest::sample::select(vec![
                "a", "b", "1", "-", "_", "=", "&", "?", "+", "(", ")", "[", "]", "|", "$", "^", ".",
            ]),
            1..6,
        )
        .prop_map(|parts| parts.concat())
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        /// A pattern with no wildcard matches itself. Translating a glob to a
        /// regex is exactly where "everything else is literal" stops holding.
        #[test]
        fn a_pattern_without_a_wildcard_is_literal(text in url_atom()) {
            prop_assert!(
                glob_match(&text, &text),
                "pattern `{}` should match itself literally", text
            );
        }

        /// A trailing `*` matches any one segment, whatever it contains.
        #[test]
        fn a_trailing_wildcard_matches_any_single_segment(
            prefix in url_atom(),
            segment in url_atom(),
        ) {
            let pattern = format!("https://example.com/{prefix}/*");
            let text = format!("https://example.com/{prefix}/{segment}");
            prop_assert!(
                glob_match(&pattern, &text),
                "`{}` should match `{}`", pattern, text
            );
        }

        /// Escaping a character makes it literal, whatever it is. This is the
        /// escape hatch for matching a URL that really does contain a `*`.
        #[test]
        fn a_backslash_escape_is_always_literal(text in url_atom()) {
            let pattern: String = text.chars().flat_map(|c| ['\\', c]).collect();
            prop_assert!(
                glob_match(&pattern, &text),
                "escaped `{}` should match `{}`", pattern, text
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::glob_match;

    #[test]
    fn exact_match_is_anchored() {
        assert!(glob_match("https://example.com/", "https://example.com/"));
        // anchored at both ends: a trailing extra segment must not match
        assert!(!glob_match("https://example.com/", "https://example.com/x"));
        // ...nor a prefix
        assert!(!glob_match("example.com", "an.example.com"));
    }

    #[test]
    fn single_star_stays_within_one_segment() {
        assert!(glob_match(
            "https://example.com/*",
            "https://example.com/foo"
        ));
        // `*` does not cross a `/`
        assert!(!glob_match(
            "https://example.com/*",
            "https://example.com/foo/bar"
        ));
        // `*` matches the empty string
        assert!(glob_match("https://example.com/*", "https://example.com/"));

        // A single star immediately before a `/` is still a single star: only
        // a run of two or more takes the cross-segment branch.
        assert!(glob_match(
            "https://example.com/*/b",
            "https://example.com/a/b"
        ));
        assert!(!glob_match(
            "https://example.com/*/b",
            "https://example.com/a/x/b"
        ));
    }

    #[test]
    fn double_star_crosses_segments() {
        assert!(glob_match(
            "https://example.com/**",
            "https://example.com/a/b/c"
        ));
        assert!(glob_match(
            "**/*.png",
            "https://cdn.example.com/img/logo.png"
        ));
    }

    #[test]
    fn double_star_between_slashes_matches_zero_directories() {
        // The driver emits `((.+/)|)` here, so the directories are optional.
        assert!(glob_match(
            "https://example.com/**/edit",
            "https://example.com/edit"
        ));
        assert!(glob_match(
            "https://example.com/**/edit",
            "https://example.com/a/b/edit"
        ));
    }

    #[test]
    fn braces_are_alternation() {
        assert!(glob_match("**/*.{png,jpg}", "https://example.com/a.png"));
        assert!(glob_match("**/*.{png,jpg}", "https://example.com/a.jpg"));
        assert!(!glob_match("**/*.{png,jpg}", "https://example.com/a.gif"));
        // A comma outside a group is literal. The negative is the load-bearing
        // half: were the comma alternation, `.../a,b` would still match itself
        // through the `b$` branch while also matching `.../a`.
        assert!(glob_match(
            "https://example.com/a,b",
            "https://example.com/a,b"
        ));
        assert!(!glob_match(
            "https://example.com/a,b",
            "https://example.com/a"
        ));
    }

    #[test]
    fn malformed_braces_match_nothing() {
        assert!(!glob_match("{a{b}}", "ab"));
        assert!(!glob_match("a}", "a}"));
        assert!(!glob_match("{a", "a"));
    }

    #[test]
    fn regex_metacharacters_are_literal() {
        // These used to reach the regex engine: `[unterminated` failed to
        // compile (matching nothing), and `?`/`+` silently changed what the
        // pattern meant.
        assert!(!glob_match("[unterminated", "anything"));
        assert!(glob_match("[unterminated", "[unterminated"));

        // A query string is the common case, and the one that motivated this.
        assert!(glob_match(
            "https://example.com/search?q=*",
            "https://example.com/search?q=widget"
        ));
        // ...and it must not match the URL the unescaped `?` used to allow.
        assert!(!glob_match(
            "https://example.com/search?q=*",
            "https://example.com/searcq=widget"
        ));
    }

    #[test]
    fn backslash_escapes_the_next_character() {
        // the escape hatch for a URL that really contains a `*`
        assert!(glob_match(
            r"https://example.com/\*",
            "https://example.com/*"
        ));
        assert!(!glob_match(
            r"https://example.com/\*",
            "https://example.com/x"
        ));
    }

    #[test]
    fn dot_is_literal_not_wildcard() {
        assert!(glob_match(
            "https://example.com/a.png",
            "https://example.com/a.png"
        ));
        assert!(!glob_match(
            "https://example.com/a.png",
            "https://example.com/axpng"
        ));
    }

    #[test]
    fn extension_globs() {
        assert!(glob_match("**/*.png", "https://example.com/a/b/c.png"));
        assert!(!glob_match("**/*.png", "https://example.com/a/b/c.jpg"));

        // `**/` before a segment emits `(.*/)`, which requires the slash, so
        // a slashless subject does not match. Pins the star-run boundary:
        // miscounting the stars here turns this into `(.*)`, which does.
        assert!(!glob_match("**/*.png", "a.png"));
    }

    #[test]
    fn a_trailing_backslash_is_literal() {
        // There is nothing to escape, so the backslash is its own character.
        // Pins the escape lookahead bound: reading past it panics.
        assert!(glob_match(r"a\", r"a\"));
        assert!(!glob_match(r"a\", "a"));
    }
}
