use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Mutex;

use crate::category::Category;
use crate::makro::Macro;
use crate::token::Token;

// A list of all primitive control sequences, used so that we can \let other
// control sequences equal to them.
const ALL_PRIMITIVES: &[&str] = &[
    "iftrue", "iffalse", "fi", "else", "def", "let", "global", "count",
];

#[derive(Clone)]
enum TokenDefinition {
    Macro(Rc<Macro>),
    Token(Token),
    Primitive(&'static str),
}

// This contains all of the mutable state about our TeX environment
#[derive(Clone)]
pub struct TeXStateInner {
    // A map individual characters to the category that that it is associated
    // with. Set and retrieved with \catcode, used in the lexer.
    category_map: HashMap<char, Category>,

    // There are several ways to redefine what a given token means, with \def,
    // \let, \chardef, etc. This map contains the definition of each redefined
    // token.
    token_definition_map: HashMap<Token, TokenDefinition>,

    // TeX's 256 count registers. The values here should be between 2147483647
    // and -2147483647 (which is very close to the possible values of i32,
    // except that i32 can also hold the value -2147483648. We should keep
    // close track of that).
    count_registers: [i32; 256],
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

        let mut token_definitions = HashMap::new();

        for primitive in ALL_PRIMITIVES {
            token_definitions.insert(
                Token::ControlSequence(primitive.to_string()),
                TokenDefinition::Primitive(primitive),
            );
        }

        TeXStateInner {
            category_map: initial_categories,
            token_definition_map: token_definitions,
            count_registers: [0; 256],
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

    fn get_macro(&self, token: &Token) -> Option<Rc<Macro>> {
        if let Some(TokenDefinition::Macro(makro)) =
            self.token_definition_map.get(token)
        {
            Some(Rc::clone(makro))
        } else {
            None
        }
    }

    fn set_macro(&mut self, token: &Token, makro: &Rc<Macro>) {
        self.token_definition_map
            .insert(token.clone(), TokenDefinition::Macro(makro.clone()));
    }

    fn get_renamed_token(&self, token: &Token) -> Option<Token> {
        if let Some(TokenDefinition::Token(renamed)) =
            self.token_definition_map.get(token)
        {
            Some(renamed.clone())
        } else {
            None
        }
    }

    fn set_let(&mut self, set_token: &Token, to_token: &Token) {
        if let Some(token_definition) = self.token_definition_map.get(to_token)
        {
            // If to_token already has a definition, we use that for the value
            // we're setting.
            self.token_definition_map
                .insert(set_token.clone(), token_definition.clone());
        } else if let Token::Char(_, cat) = to_token {
            if cat != &Category::Active {
                // Otherwise, if to_token is a char token with a non-active
                // category, we create a new definition for that character.
                // TODO(xymostech): Figure out if this is the correct behavior
                // for when to_token is a special token. This current guess of
                // behavior is based on trying
                // \catcode`@=13 \let\a=@ \def@{x} \show\a
                // and seeing that it gives \a=undefined
                self.token_definition_map.insert(
                    set_token.clone(),
                    TokenDefinition::Token(to_token.clone()),
                );
            }
        }
    }

    fn is_token_equal_to_cs(&self, token: &Token, cs: &str) -> bool {
        if let Token::ControlSequence(real_cs) = token {
            if real_cs == cs {
                return true;
            }
        }

        if let Some(TokenDefinition::Primitive(prim_cs)) =
            self.token_definition_map.get(token)
        {
            if prim_cs == &cs {
                return true;
            }
        }

        return false;
    }

    fn get_count(&self, register_index: usize) -> i32 {
        self.count_registers[register_index]
    }

    fn set_count(&mut self, register_index: usize, value: i32) {
        if value == -2147483648 {
            panic!("Invalid value for count: {}", value);
        }

        self.count_registers[register_index] = value;
    }
}

// TeX keeps a stack of different states around, and pushes a copy of the
// current stack when entering a group (with {) and pops the top of the stack
// when leaving a group (with }). The "current" value of variables is taken
// from the top of the stack, and assignments can be made to apply to every
// level of the stack using \global.
struct TeXStateStack {
    state_stack: Vec<TeXStateInner>,
}

// Since we're mostly want to just be calling the same-named functions from
// TeXStateStack onto the top level of TeXStateInner, we make a macro to easily
// do that for us.
macro_rules! generate_inner_func {
    (fn $func_name:ident(
        $($var_name:ident : $var_type:ty),*) $( -> $return_type:ty)?) =>
    {
        pub fn $func_name(&self, $($var_name: $var_type),*)$( -> $return_type)* {
            self.state_stack[self.state_stack.len() - 1].$func_name($($var_name),*)
        }
    }
}

// When we have setter functions that are optionally global (i.e. optionally
// operate on all of the levels of TeXStateInner), we can use this macro to
// automatically define them.
macro_rules! generate_inner_global_func {
    (fn $func_name:ident(
        global: bool, $($var_name:ident : $var_type:ty),*)) =>
    {
        fn $func_name(&mut self, global: bool, $($var_name: $var_type),*) {
            if global {
                for state in &mut self.state_stack {
                    state.$func_name($($var_name),*);
                }
            } else {
                let len = self.state_stack.len();
                self.state_stack[len - 1].$func_name($($var_name),*);
            }
        }
    }
}

impl TeXStateStack {
    fn new() -> TeXStateStack {
        TeXStateStack {
            state_stack: vec![TeXStateInner::new()],
        }
    }

    fn push_state(&mut self) {
        let top_state = self.state_stack[self.state_stack.len() - 1].clone();
        self.state_stack.push(top_state);
    }

    fn pop_state(&mut self) {
        self.state_stack.pop().unwrap();
    }

    generate_inner_func!(fn get_category(ch: char) -> Category);
    generate_inner_global_func!(fn set_category(global: bool, ch: char, cat: Category));
    generate_inner_func!(fn get_macro(token: &Token) -> Option<Rc<Macro>>);
    generate_inner_global_func!(fn set_macro(global: bool, token: &Token, makro: &Rc<Macro>));
    generate_inner_func!(fn get_renamed_token(token: &Token) -> Option<Token>);
    generate_inner_global_func!(fn set_let(global: bool, set_token: &Token, to_token: &Token));
    generate_inner_func!(fn is_token_equal_to_cs(token: &Token, cs: &str) -> bool);
    generate_inner_func!(fn get_count(register_index: usize) -> i32);
    generate_inner_global_func!(fn set_count(global: bool, register_index: usize, value: i32));
}

// A lot of the state in TeX is treated as global state, where we need to be
// able to read and write to it from wherever we are in the parsing process. In
// order to accomplish this in a type-safe way, we keep a mutex pointing to the
// actual TeX state and then pull the state out of the mutex whenever we need
// it. We aren't using multiple threads or anything, we're just (ab)using the
// ability to get a mutable reference to the inner state even when other
// references exist.
pub struct TeXState {
    state_stack: Mutex<TeXStateStack>,
}

// Since we're mostly want to just be calling the same-named functions from
// TeXState onto TeXStateStack, we make a macro to easily do that for us.
macro_rules! generate_stack_func {
    (fn $func_name:ident(
        $($var_name:ident : $var_type:ty),*) $( -> $return_type:ty)?) =>
    {
        pub fn $func_name(&self, $($var_name: $var_type),*)$( -> $return_type)* {
            self.with_stack(|stack| {
                stack.$func_name($($var_name),*)
            })
        }
    }
}

impl TeXState {
    pub fn new() -> TeXState {
        TeXState {
            state_stack: Mutex::new(TeXStateStack::new()),
        }
    }

    // Helper function for making pulling the TeXStateStack out of the mutex
    // easier.
    fn with_stack<T, F>(&self, func: F) -> T
    where
        F: FnOnce(&mut TeXStateStack) -> T,
    {
        let mut stack = self.state_stack.lock().unwrap();
        func(&mut stack)
    }

    generate_stack_func!(fn push_state());
    generate_stack_func!(fn pop_state());

    generate_stack_func!(fn get_category(ch: char) -> Category);
    generate_stack_func!(fn set_category(global: bool, ch: char, cat: Category));
    generate_stack_func!(fn get_macro(token: &Token) -> Option<Rc<Macro>>);
    generate_stack_func!(fn set_macro(global: bool, token: &Token, makro: &Rc<Macro>));
    generate_stack_func!(fn get_renamed_token(token: &Token) -> Option<Token>);
    generate_stack_func!(fn set_let(global: bool, set_token: &Token, to_token: &Token));
    generate_stack_func!(fn is_token_equal_to_cs(token: &Token, cs: &str) -> bool);
    generate_stack_func!(fn get_count(register_index: usize) -> i32);
    generate_stack_func!(fn set_count(global: bool, register_index: usize, value: i32));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_correctly_sets_categories() {
        let state = TeXState::new();
        assert_eq!(state.get_category('@'), Category::Other);
        state.set_category(false, '@', Category::Letter);
        assert_eq!(state.get_category('@'), Category::Letter);
    }

    #[test]
    fn it_allows_mutation_with_existing_refs() {
        let state = TeXState::new();

        let _state_ref: &TeXState = &state;
        state.set_category(false, '@', Category::Letter);
        assert_eq!(state.get_category('@'), Category::Letter);
    }

    #[test]
    fn it_restores_old_values_after_group_ends() {
        let state = TeXState::new();

        state.set_category(false, '@', Category::Letter);
        assert_eq!(state.get_category('@'), Category::Letter);

        state.push_state();

        assert_eq!(state.get_category('@'), Category::Letter);
        state.set_category(false, '@', Category::Other);
        assert_eq!(state.get_category('@'), Category::Other);

        state.pop_state();

        assert_eq!(state.get_category('@'), Category::Letter);
    }

    #[test]
    fn it_keeps_globally_set_values_after_group_ends() {
        let state = TeXState::new();

        state.set_category(false, '@', Category::Letter);

        state.push_state();

        state.set_category(false, '@', Category::Ignored);

        state.push_state();

        state.set_category(true, '@', Category::Other);
        assert_eq!(state.get_category('@'), Category::Other);

        state.pop_state();
        assert_eq!(state.get_category('@'), Category::Other);

        state.pop_state();
        assert_eq!(state.get_category('@'), Category::Other);
    }

    #[test]
    fn it_compares_control_sequences() {
        let state = TeXState::new();

        assert!(state.is_token_equal_to_cs(
            &Token::ControlSequence("foo".to_string()),
            "foo"
        ));

        state.set_let(
            false,
            &Token::ControlSequence("boo".to_string()),
            &Token::ControlSequence("let".to_string()),
        );

        assert!(state.is_token_equal_to_cs(
            &Token::ControlSequence("boo".to_string()),
            "let"
        ));
    }
}
