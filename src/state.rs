use std::collections::HashMap;
use std::sync::Mutex;

use crate::category::Category;

// This contains all of the mutable state about our TeX environment
pub struct TeXStateInner {
    // A map individual characters to the category that that it is associated
    // with. Set and retrieved with \catcode, used in the lexer.
    category_map: HashMap<char, Category>,
}

impl TeXStateInner {
    fn new() -> TeXStateInner {
        // Set up the default categories of various characters
        let mut initial_categories = HashMap::new();
        // ASCII characters are marked as Letters
        for i in 0..255 {
            let ch = (i as u8) as char;
            if ('a' <= ch && ch <= 'z') || ('A' <= ch && ch <= 'Z') {
                initial_categories.insert(ch, Category::Letter);
            }
        }
        // Other various default categories
        initial_categories.insert('\u{0000}', Category::Ignored);
        initial_categories.insert('\u{00ff}', Category::Invalid);
        initial_categories.insert('\n', Category::EndOfLine);
        initial_categories.insert('\\', Category::Escape);
        initial_categories.insert('%', Category::Comment);
        initial_categories.insert(' ', Category::Space);

        // TODO(emily): These aren't actually set by default, they are set
        // after initialization in init.tex. Remove them once we can run that!
        initial_categories.insert('^', Category::Superscript);
        initial_categories.insert('{', Category::BeginGroup);
        initial_categories.insert('}', Category::EndGroup);
        initial_categories.insert('#', Category::Parameter);
        initial_categories.insert('$', Category::MathShift);

        TeXStateInner {
            category_map: initial_categories,
        }
    }

    fn get_category(&self, ch: char) -> Category {
        match self.category_map.get(&ch) {
            Some(&cat) => cat,
            None => Category::Other,
        }
    }

    fn set_category(&mut self, ch: char, cat: Category) {
        self.category_map.insert(ch, cat);
    }
}

// A lot of the state in TeX is treated as global state, where we need to be
// able to read and write to it from wherever we are in the parsing process. In
// order to accomplish this in a type-safe way, we keep a mutex pointing to the
// actual TeX state and then pull the state out of the mutex whenever we need
// it. We aren't using multiple threads or anything, we're just (ab)using the
// ability to get a mutable reference to the inner state even when other
// references exist.
pub struct TeXState {
    state_inner: Mutex<TeXStateInner>,
}

// Since we're mostly want to just be calling the same-named functions from
// TeXState onto TeXStateInner, we make a macro to easily do that for us.
macro_rules! generate_inner {
    (fn $func_name:ident(
        $($var_name:ident : $var_type:ty),*) $( -> $return_type:ty)?) =>
    {
        fn $func_name(&self, $($var_name: $var_type),*)$( -> $return_type)* {
            self.with_inner(|inner| {
                inner.$func_name($($var_name),*)
            })
        }
    }
}

impl TeXState {
    fn new() -> TeXState {
        TeXState {
            state_inner: Mutex::new(TeXStateInner::new()),
        }
    }

    // Helper function for making pulling the TeXStateInner out of the mutex
    // easier.
    fn with_inner<T, F>(&self, func: F) -> T
        where F: FnOnce(&mut TeXStateInner) -> T
    {
        let mut inner = self.state_inner.lock().unwrap();
        func(&mut inner)
    }

    generate_inner!(fn get_category(ch: char) -> Category);
    generate_inner!(fn set_category(ch: char, cat: Category));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correctly_sets_categories() {
        let state = TeXState::new();
        assert_eq!(state.get_category('@'), Category::Other);
        state.set_category('@', Category::Letter);
        assert_eq!(state.get_category('@'), Category::Letter);
    }

    #[test]
    fn allows_mutation_with_existing_refs() {
        let state = TeXState::new();

        let _state_ref: &TeXState = &state;
        state.set_category('@', Category::Letter);
        assert_eq!(state.get_category('@'), Category::Letter);
    }
}
