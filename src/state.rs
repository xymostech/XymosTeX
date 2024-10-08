use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use crate::boxes::TeXBox;
use crate::category::Category;
use crate::dimension::{Dimen, FilDimen, FilKind, SpringDimen, Unit};
use crate::font::Font;
use crate::font_metrics::FontMetrics;
use crate::glue::Glue;
use crate::makro::Macro;
use crate::math_code::MathCode;
use crate::token::Token;

// A list of all primitive control sequences, used so that we can \let other
// control sequences equal to them.
const ALL_PRIMITIVES: &[&str] = &[
    "iftrue",
    "iffalse",
    "fi",
    "else",
    "def",
    "let",
    "global",
    "count",
    "ifnum",
    "advance",
    "multiply",
    "divide",
    "number",
    "par",
    "hskip",
    "hbox",
    "relax",
    "setbox",
    "wd",
    "ht",
    "dp",
    "box",
    "vskip",
    "end",
    "indent",
    "noindent",
    "copy",
    "vbox",
    "mathchardef",
    "mathcode",
    "displaystyle",
    "textstyle",
    "scriptstyle",
    "scriptscriptstyle",
    "font",
    "raise",
    "lower",
    "moveleft",
    "moveright",
    "prevdepth",
    "char",
    "over",
    "atop",
    "above",
    "overwithdelims",
    "atopwithdelims",
    "abovewithdelims",
    "hsize",
    "parskip",
    "spaceskip",
    "parfillskip",
    "pretolerance",
    "tolerance",
    "tracingparagraphs",
    "adjdemerits",
];

fn is_primitive(maybe_prim: &str) -> bool {
    for prim in ALL_PRIMITIVES {
        if *prim == maybe_prim {
            return true;
        }
    }
    false
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntegerParameter {
    Pretolerance,
    Tolerance,
    TracingParagraphs,
    AdjDemerits,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DimenParameter {
    HSize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GlueParameter {
    ParSkip,
    SpaceSkip,
    ParFillSkip,
}

#[derive(Clone)]
enum TokenDefinition {
    Macro(Rc<Macro>),
    Token(Token),
    MathCode(MathCode),
    Primitive(&'static str),
    Font(Font),
}

// This contains all of the mutable state about our TeX environment
#[derive(Clone)]
pub struct TeXStateInner {
    // A map individual characters to the category that that it is associated
    // with. Set and retrieved with \catcode, used in the lexer.
    category_map: HashMap<char, Category>,

    // A map of individual characters to the math code that it is associated
    // with. Set and retrieved with \mathcode, only used in math mode.
    math_code_map: HashMap<char, MathCode>,

    // There are several ways to redefine what a given token means, with \def,
    // \let, \chardef, etc. This map contains the definition of each redefined
    // token.
    token_definition_map: HashMap<Token, TokenDefinition>,

    // TeX's 256 count registers. The values here should be between 2147483647
    // and -2147483647 (which is very close to the possible values of i32,
    // except that i32 can also hold the value -2147483648. We should keep
    // close track of that).
    count_registers: [i32; 256],

    // TeX's explicit integer parameter registers, like \tolerance or
    // \linepenalty. Missing integers are treated as zero. Similar to the count
    // registers, the values here should be between 2147483647 and -2147483647.
    integer_parameter_registers: HashMap<IntegerParameter, i32>,

    // TeX's explicit dimen parameter registers, like \hsize or \parindent.
    // Missing dimens are treated as zero.
    dimen_parameter_registers: HashMap<DimenParameter, Dimen>,

    // TeX's explicit glue parameter registers, like \parskip or \spaceskip
    // Missing glues are treated as zero.
    glue_parameter_registers: HashMap<GlueParameter, Glue>,

    // TeX's 256 box registers. The values are designed such that:
    //  * When entering a new group, we don't make a copy of a box by making
    //    the values Rc.
    //  * When we use a box (via \box<n>), we can pull the box out of the state
    //    and it will be inaccessible from the other places that store a
    //    reference to this box (via RefCell).
    // We don't store these as an array because it's likely that all of the
    // boxes won't be used all the time, and we don't want to allocate 256 of
    // these if they're not going to be used.
    // TODO(xymostech): Check the assumption that most of these aren't used
    // most of the time.
    box_registers: HashMap<u8, Rc<RefCell<Option<TeXBox>>>>,

    // We keep track of the name of the current font. Metrics and other
    // information about the font are stored elsewhere.
    current_font: Font,
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
        // after initialization in plain.tex. Remove them once we can run that!
        initial_categories.insert('^', Category::Superscript);
        initial_categories.insert('_', Category::Subscript);
        initial_categories.insert('{', Category::BeginGroup);
        initial_categories.insert('}', Category::EndGroup);
        initial_categories.insert('#', Category::Parameter);
        initial_categories.insert('$', Category::MathShift);

        let mut initial_math_codes = HashMap::new();
        for i in 0..255 {
            let ch = (i as u8) as char;
            if ('a' <= ch && ch <= 'z') || ('A' <= ch && ch <= 'Z') {
                initial_math_codes
                    .insert(ch, MathCode::from_number(0x7100 + i));
            } else if '0' <= ch && ch <= '9' {
                initial_math_codes
                    .insert(ch, MathCode::from_number(0x7000 + i));
            }
        }

        let mut initial_integer_registers = HashMap::new();
        // TODO(emily): INITEX actually sets \tolerance to 10000, but it is
        // reset to 200 in plain.tex. Remove this once we run that.
        initial_integer_registers.insert(IntegerParameter::Tolerance, 200);
        initial_integer_registers.insert(IntegerParameter::Pretolerance, 100);
        // TODO(emily): This is set in plain.tex. Remove this once we run that.
        initial_integer_registers.insert(IntegerParameter::AdjDemerits, 10000);

        let mut initial_dimen_registers = HashMap::new();
        // TODO(emily): This is set in plain.tex. Remove this once we run that.
        initial_dimen_registers
            .insert(DimenParameter::HSize, Dimen::from_unit(6.5, Unit::Inch));

        let initial_glue_registers = HashMap::from([
            (
                GlueParameter::ParSkip,
                Glue {
                    space: Dimen::zero(),
                    stretch: SpringDimen::Dimen(Dimen::from_unit(
                        1.0,
                        Unit::Point,
                    )),
                    shrink: SpringDimen::Dimen(Dimen::zero()),
                },
            ),
            (
                GlueParameter::ParFillSkip,
                Glue {
                    space: Dimen::zero(),
                    stretch: SpringDimen::FilDimen(FilDimen::new(
                        FilKind::Fil,
                        1.0,
                    )),
                    shrink: SpringDimen::Dimen(Dimen::zero()),
                },
            ),
        ]);

        let mut token_definitions = HashMap::new();

        for primitive in ALL_PRIMITIVES {
            token_definitions.insert(
                Token::ControlSequence(primitive.to_string()),
                TokenDefinition::Primitive(primitive),
            );
        }

        TeXStateInner {
            category_map: initial_categories,
            math_code_map: initial_math_codes,
            token_definition_map: token_definitions,
            count_registers: [0; 256],
            integer_parameter_registers: initial_integer_registers,
            dimen_parameter_registers: initial_dimen_registers,
            glue_parameter_registers: initial_glue_registers,
            box_registers: HashMap::new(),
            current_font: Font {
                // TODO(xymostech): This should initially be "nullfont"
                font_name: "cmr10".to_string(),
                scale: Dimen::from_unit(10.0, Unit::Point),
            },
        }
    }

    fn get_category(&self, ch: char) -> Category {
        match self.category_map.get(&ch) {
            Some(&cat) => cat,
            None => Category::Other,
        }
    }

    #[cfg(test)]
    fn set_category(&mut self, ch: char, cat: Category) {
        self.category_map.insert(ch, cat);
    }

    fn get_integer_parameter(
        &self,
        integer_parameter: &IntegerParameter,
    ) -> i32 {
        *self
            .integer_parameter_registers
            .get(integer_parameter)
            .unwrap_or(&0)
    }

    fn set_integer_parameter(
        &mut self,
        integer_parameter: &IntegerParameter,
        value: i32,
    ) {
        self.integer_parameter_registers
            .insert(*integer_parameter, value);
    }

    fn get_dimen_parameter(&self, dimen_parameter: &DimenParameter) -> Dimen {
        self.dimen_parameter_registers
            .get(dimen_parameter)
            .map(|dimen| *dimen)
            .unwrap_or(Dimen::zero())
    }

    fn set_dimen_parameter(
        &mut self,
        dimen_parameter: &DimenParameter,
        dimen: &Dimen,
    ) {
        self.dimen_parameter_registers
            .insert(*dimen_parameter, *dimen);
    }

    fn get_glue_parameter(&self, glue_parameter: &GlueParameter) -> Glue {
        self.glue_parameter_registers
            .get(glue_parameter)
            .map(|glue| glue.clone())
            .unwrap_or(Glue::zero())
    }

    fn set_glue_parameter(
        &mut self,
        glue_parameter: &GlueParameter,
        glue: &Glue,
    ) {
        self.glue_parameter_registers
            .insert(*glue_parameter, glue.clone());
    }

    fn get_math_code(&self, ch: char) -> MathCode {
        match self.math_code_map.get(&ch) {
            Some(mathcode) => mathcode.clone(),
            None => MathCode::from_number(ch as u32),
        }
    }

    fn set_math_code(&mut self, ch: char, mathcode: &MathCode) {
        self.math_code_map.insert(ch, mathcode.clone());
    }

    fn get_math_chardef(&self, token: &Token) -> Option<MathCode> {
        if let Some(TokenDefinition::MathCode(math_code)) =
            self.token_definition_map.get(token)
        {
            Some(math_code.clone())
        } else {
            None
        }
    }

    fn set_math_chardef(&mut self, token: &Token, math_code: &MathCode) {
        self.token_definition_map.insert(
            token.clone(),
            TokenDefinition::MathCode(math_code.clone()),
        );
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
            let cloned_token = token_definition.clone();
            self.token_definition_map
                .insert(set_token.clone(), cloned_token);
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

    fn is_token_equal_to_prim(&self, token: &Token, prim: &str) -> bool {
        if cfg!(debug_assertions) && !is_primitive(prim) {
            panic!("Testing invalid primitive: {}", prim);
        }

        if let Token::ControlSequence(real_cs) = token {
            if real_cs == prim {
                return true;
            }
        }

        if let Some(TokenDefinition::Primitive(prim_cs)) =
            self.token_definition_map.get(token)
        {
            if prim_cs == &prim {
                return true;
            }
        }

        false
    }

    fn get_count(&self, register_index: u8) -> i32 {
        self.count_registers[register_index as usize]
    }

    fn set_count(&mut self, register_index: u8, value: i32) {
        if value == -2147483648 {
            panic!("Invalid value for count: {}", value);
        }

        self.count_registers[register_index as usize] = value;
    }

    fn get_current_font(&self) -> Font {
        self.current_font.clone()
    }

    fn set_current_font(&mut self, font: &Font) {
        self.current_font = font.clone();
    }

    fn set_fontdef(&mut self, token: &Token, font: &Font) {
        self.token_definition_map
            .insert(token.clone(), TokenDefinition::Font(font.clone()));
    }

    fn get_fontdef(&self, token: &Token) -> Option<Font> {
        if let Some(TokenDefinition::Font(font)) =
            self.token_definition_map.get(token)
        {
            Some(font.clone())
        } else {
            None
        }
    }

    fn get_box(&self, box_index: u8) -> Option<TeXBox> {
        self.box_registers
            .get(&box_index)
            .and_then(|box_refcell| box_refcell.replace(None))
    }

    fn get_box_copy(&self, box_index: u8) -> Option<TeXBox> {
        self.box_registers
            .get(&box_index)
            .and_then(|box_refcell| (*box_refcell.borrow()).clone())
    }

    fn set_box(&mut self, box_index: u8, tex_box: Rc<RefCell<Option<TeXBox>>>) {
        self.box_registers.insert(box_index, tex_box);
    }

    fn with_box<T, F>(&self, box_index: u8, func: F) -> Option<T>
    where
        F: FnOnce(&mut TeXBox) -> T,
    {
        if let Some(box_refcell) = self.box_registers.get(&box_index) {
            if let Some(ref mut tex_box) = *box_refcell.borrow_mut() {
                Some(func(tex_box))
            } else {
                None
            }
        } else {
            None
        }
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
    #[cfg(test)]
    generate_inner_global_func!(fn set_category(global: bool, ch: char, cat: Category));
    generate_inner_func!(fn get_integer_parameter(integer_parameter: &IntegerParameter) -> i32);
    generate_inner_global_func!(fn set_integer_parameter(global: bool, integer_parameter: &IntegerParameter, value: i32));
    generate_inner_func!(fn get_dimen_parameter(dimen_parameter: &DimenParameter) -> Dimen);
    generate_inner_global_func!(fn set_dimen_parameter(global: bool, dimen_parameter: &DimenParameter, dimen: &Dimen));
    generate_inner_func!(fn get_glue_parameter(glue_parameter: &GlueParameter) -> Glue);
    generate_inner_global_func!(fn set_glue_parameter(global: bool, glue_parameter: &GlueParameter, glue: &Glue));
    generate_inner_func!(fn get_math_code(ch: char) -> MathCode);
    generate_inner_global_func!(fn set_math_code(global: bool, ch: char, mathcode: &MathCode));
    generate_inner_func!(fn get_math_chardef(token: &Token) -> Option<MathCode>);
    generate_inner_global_func!(fn set_math_chardef(global: bool, token: &Token, mathcode: &MathCode));
    generate_inner_func!(fn get_macro(token: &Token) -> Option<Rc<Macro>>);
    generate_inner_global_func!(fn set_macro(global: bool, token: &Token, makro: &Rc<Macro>));
    generate_inner_func!(fn get_renamed_token(token: &Token) -> Option<Token>);
    generate_inner_global_func!(fn set_let(global: bool, set_token: &Token, to_token: &Token));
    generate_inner_func!(fn is_token_equal_to_prim(token: &Token, cs: &str) -> bool);
    generate_inner_func!(fn get_count(register_index: u8) -> i32);
    generate_inner_global_func!(fn set_count(global: bool, register_index: u8, value: i32));
    generate_inner_func!(fn get_current_font() -> Font);
    generate_inner_global_func!(fn set_current_font(global: bool, font: &Font));
    generate_inner_global_func!(fn set_fontdef(global: bool, token: &Token, font: &Font));
    generate_inner_func!(fn get_fontdef(token: &Token) -> Option<Font>);
    generate_inner_func!(fn get_box(box_index: u8) -> Option<TeXBox>);
    generate_inner_func!(fn get_box_copy(box_index: u8) -> Option<TeXBox>);

    // Because globally setting boxes means that we should share references
    // between the different stack levels, we can't handle generating this
    // function automatically with `generate_inner_global_func!()`.
    fn set_box(&mut self, global: bool, box_index: u8, tex_box: TeXBox) {
        let wrapped_box = Rc::new(RefCell::new(Some(tex_box)));
        if global {
            for state in &mut self.state_stack {
                state.set_box(box_index, wrapped_box.clone());
            }
        } else {
            let len = self.state_stack.len();
            self.state_stack[len - 1].set_box(box_index, wrapped_box);
        }
    }

    fn with_box<T, F>(&self, box_index: u8, func: F) -> Option<T>
    where
        F: FnOnce(&mut TeXBox) -> T,
    {
        self.state_stack[self.state_stack.len() - 1].with_box(box_index, func)
    }
}

// A lot of the state in TeX is treated as global state, where we need to be
// able to read and write to it from wherever we are in the parsing process. In
// order to accomplish this in a type-safe way, we keep a RefCell pointing to
// the actual TeX state and then pull the state out of the RefCell whenever we
// need it. Since we only ever pull the state out of the RefCell in the methods
// on TeXState, we can't ever end up with the RefCell being borrwed twice.
pub struct TeXState {
    state_stack: RefCell<TeXStateStack>,

    // Stores metrics information about a given font file. We don't store this
    // in the `TeXStateInner` because loading the font metrics is global and
    // isn't affected by grouping.
    font_metrics: RefCell<HashMap<Font, FontMetrics>>,
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
            state_stack: RefCell::new(TeXStateStack::new()),
            font_metrics: RefCell::new(HashMap::new()),
        }
    }

    // Helper function for making pulling the TeXStateStack out of the RefCell
    // easier.
    fn with_stack<T, F>(&self, func: F) -> T
    where
        F: FnOnce(&mut TeXStateStack) -> T,
    {
        let mut stack = self.state_stack.borrow_mut();
        func(&mut stack)
    }

    generate_stack_func!(fn push_state());
    generate_stack_func!(fn pop_state());

    generate_stack_func!(fn get_category(ch: char) -> Category);
    #[cfg(test)]
    generate_stack_func!(fn set_category(global: bool, ch: char, cat: Category));
    generate_stack_func!(fn get_integer_parameter(integer_parameter: &IntegerParameter) -> i32);
    generate_stack_func!(fn set_integer_parameter(global: bool, integer_parameter: &IntegerParameter, value: i32));
    generate_stack_func!(fn get_dimen_parameter(dimen_parameter: &DimenParameter) -> Dimen);
    generate_stack_func!(fn set_dimen_parameter(global: bool, dimen_parameter: &DimenParameter, dimen: &Dimen));
    generate_stack_func!(fn get_glue_parameter(glue_parameter: &GlueParameter) -> Glue);
    generate_stack_func!(fn set_glue_parameter(global: bool, glue_parameter: &GlueParameter, glue: &Glue));
    generate_stack_func!(fn get_math_code(ch: char) -> MathCode);
    generate_stack_func!(fn set_math_code(global: bool, ch: char, mathcode: &MathCode));
    generate_stack_func!(fn get_math_chardef(token: &Token) -> Option<MathCode>);
    generate_stack_func!(fn set_math_chardef(global: bool, token: &Token, mathcode: &MathCode));
    generate_stack_func!(fn get_macro(token: &Token) -> Option<Rc<Macro>>);
    generate_stack_func!(fn set_macro(global: bool, token: &Token, makro: &Rc<Macro>));
    generate_stack_func!(fn get_renamed_token(token: &Token) -> Option<Token>);
    generate_stack_func!(fn set_let(global: bool, set_token: &Token, to_token: &Token));
    generate_stack_func!(fn is_token_equal_to_prim(token: &Token, cs: &str) -> bool);
    generate_stack_func!(fn get_count(register_index: u8) -> i32);
    generate_stack_func!(fn set_count(global: bool, register_index: u8, value: i32));
    generate_stack_func!(fn get_current_font() -> Font);
    generate_stack_func!(fn set_current_font(global: bool, font: &Font));
    generate_stack_func!(fn set_fontdef(global: bool, token: &Token, font: &Font));
    generate_stack_func!(fn get_fontdef(token: &Token) -> Option<Font>);
    generate_stack_func!(fn get_box(box_index: u8) -> Option<TeXBox>);
    generate_stack_func!(fn get_box_copy(box_index: u8) -> Option<TeXBox>);
    generate_stack_func!(fn set_box(global: bool, box_index: u8, tex_box: TeXBox));

    /// Run a function on a mutable reference to a Box in a given Box register.
    /// This allows access and mutations to the boxes without removing or
    /// copying the boxes out. Returns None if there is no box at that register
    /// index.
    ///
    /// Note that this currently only runs on the top box of the state stack;
    /// there is no way to access or mutate boxes in other parts of the stack.
    pub fn with_box<T, F>(&self, box_index: u8, func: F) -> Option<T>
    where
        F: FnOnce(&mut TeXBox) -> T,
    {
        self.with_stack(|stack| stack.with_box(box_index, func))
    }

    /// Returns a reference to the font metrics for a given font.
    /// NOTE: this will load the font metrics for a font if they haven't been
    /// loaded yet, which attempts to generate a mutable font metrics ref in
    /// the process. Thus, holding onto a reference to FontMetrics can cause
    /// problems if unloaded fonts are accessed. Prefer `with_metrics_for_font`
    /// which drops the FontMetrics reference immediately after use.
    pub fn get_metrics_for_font(
        &self,
        font: &Font,
    ) -> Option<Ref<FontMetrics>> {
        let has_metrics = self.font_metrics.borrow().contains_key(font);

        if !has_metrics {
            let mut font_metrics_mut = self.font_metrics.borrow_mut();
            font_metrics_mut
                .insert(font.clone(), FontMetrics::from_font(font)?);
        }

        Some(Ref::map(self.font_metrics.borrow(), |x| {
            x.get(font).unwrap()
        }))
    }

    /// Given a font, calls a callback with the font's font metrics, and
    /// returns the result of the callback.
    pub fn with_metrics_for_font<T, F>(&self, font: &Font, func: F) -> Option<T>
    where
        F: FnOnce(Ref<FontMetrics>) -> T,
    {
        let metrics = self.get_metrics_for_font(font);
        match metrics {
            Some(metrics) => Some(func(metrics)),
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::boxes::HorizontalBox;

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

        assert!(state.is_token_equal_to_prim(
            &Token::ControlSequence("let".to_string()),
            "let"
        ));

        state.set_let(
            false,
            &Token::ControlSequence("boo".to_string()),
            &Token::ControlSequence("let".to_string()),
        );

        assert!(state.is_token_equal_to_prim(
            &Token::ControlSequence("boo".to_string()),
            "let"
        ));
    }

    #[test]
    fn it_retrieves_boxes_once() {
        let state = TeXState::new();

        let expected_box = TeXBox::HorizontalBox(HorizontalBox {
            height: Dimen::from_unit(0.0, Unit::Point),
            depth: Dimen::from_unit(0.0, Unit::Point),
            width: Dimen::from_unit(0.0, Unit::Point),
            list: Vec::new(),
            glue_set_ratio: None,
        });

        // \setbox0=\hbox{}
        // \box0
        // \box0
        state.set_box(false, 0, expected_box.clone());
        assert_eq!(Some(expected_box), state.get_box(0));
        assert_eq!(None, state.get_box(0));
    }

    #[test]
    fn it_retrieves_box_copies() {
        let state = TeXState::new();

        let test_box = TeXBox::HorizontalBox(HorizontalBox {
            height: Dimen::from_unit(0.0, Unit::Point),
            depth: Dimen::from_unit(0.0, Unit::Point),
            width: Dimen::from_unit(0.0, Unit::Point),
            list: Vec::new(),
            glue_set_ratio: None,
        });

        // \setbox0=\hbox{}
        // \copy0
        // \box0
        state.set_box(false, 0, test_box);
        assert!(state.get_box_copy(0).is_some());
        assert!(state.get_box(0).is_some());
    }

    #[test]
    fn it_reuses_references_among_stack_frames() {
        let state = TeXState::new();

        let test_box = TeXBox::HorizontalBox(HorizontalBox {
            height: Dimen::from_unit(0.0, Unit::Point),
            depth: Dimen::from_unit(0.0, Unit::Point),
            width: Dimen::from_unit(0.0, Unit::Point),
            list: Vec::new(),
            glue_set_ratio: None,
        });

        // \setbox0=\hbox{}
        // {\box0}
        // \box0
        state.set_box(false, 0, test_box.clone());
        state.push_state();
        assert_eq!(Some(test_box), state.get_box(0));
        state.pop_state();
        assert_eq!(None, state.get_box(0));
    }

    #[test]
    fn it_makes_new_references_in_new_stack_frames() {
        let state = TeXState::new();

        let outer_box = TeXBox::HorizontalBox(HorizontalBox {
            height: Dimen::from_unit(0.0, Unit::Point),
            depth: Dimen::from_unit(0.0, Unit::Point),
            width: Dimen::from_unit(0.0, Unit::Point),
            list: Vec::new(),
            glue_set_ratio: None,
        });

        let inner_box = TeXBox::HorizontalBox(HorizontalBox {
            height: Dimen::from_unit(0.0, Unit::Point),
            depth: Dimen::from_unit(0.0, Unit::Point),
            width: Dimen::from_unit(1.0, Unit::Point),
            list: Vec::new(),
            glue_set_ratio: None,
        });

        // \setbox0=\hbox{}
        // {\setbox0=\hbox{}
        // {\box0}
        // \box0}
        // \box0
        state.set_box(false, 0, outer_box.clone());
        state.push_state();
        state.set_box(false, 0, inner_box.clone());
        state.push_state();
        assert_eq!(Some(inner_box), state.get_box(0));
        state.pop_state();
        assert_eq!(None, state.get_box(0));
        state.pop_state();
        assert_eq!(Some(outer_box), state.get_box(0));
    }

    #[test]
    fn it_uses_the_same_reference_for_global_box_assignments() {
        let state = TeXState::new();

        let test_box = TeXBox::HorizontalBox(HorizontalBox {
            height: Dimen::from_unit(0.0, Unit::Point),
            depth: Dimen::from_unit(0.0, Unit::Point),
            width: Dimen::from_unit(0.0, Unit::Point),
            list: Vec::new(),
            glue_set_ratio: None,
        });

        // {{\global\setbox0=\hbox{}}
        // \box0}
        // \box0
        state.push_state();
        state.push_state();
        state.set_box(true, 0, test_box.clone());
        state.pop_state();
        assert_eq!(Some(test_box), state.get_box(0));
        state.pop_state();
        assert_eq!(None, state.get_box(0));
    }

    #[test]
    fn it_provides_mutable_access_to_boxes() {
        let state = TeXState::new();

        let test_box = TeXBox::HorizontalBox(HorizontalBox {
            height: Dimen::from_unit(1.0, Unit::Point),
            depth: Dimen::from_unit(2.0, Unit::Point),
            width: Dimen::from_unit(3.0, Unit::Point),
            list: Vec::new(),
            glue_set_ratio: None,
        });

        state.set_box(true, 0, test_box);
        state.push_state();

        // We have access to the box
        assert_eq!(
            state.with_box(0, |box_ref| *box_ref.mut_width()),
            Some(Dimen::from_unit(3.0, Unit::Point))
        );

        state.with_box(0, |box_ref| {
            *box_ref.mut_depth() = Dimen::from_unit(4.0, Unit::Point)
        });
        state.pop_state();

        // We mutated the box
        let mut final_box = state.get_box(0).unwrap();
        assert_eq!(*final_box.mut_depth(), Dimen::from_unit(4.0, Unit::Point));
    }

    #[test]
    fn it_sets_math_codes_initially() {
        let state = TeXState::new();

        assert_eq!(state.get_math_code('a'), MathCode::from_number(0x7161));
        assert_eq!(state.get_math_code('Z'), MathCode::from_number(0x715A));
        assert_eq!(state.get_math_code('0'), MathCode::from_number(0x7030));
        assert_eq!(state.get_math_code('('), MathCode::from_number(0x0028));
    }

    #[test]
    fn it_gets_and_sets_math_codes_correctly() {
        let state = TeXState::new();

        state.set_math_code(false, '(', &MathCode::from_number(0x4028));
        assert_eq!(state.get_math_code('('), MathCode::from_number(0x4028));
    }

    #[test]
    fn it_gets_and_sets_math_chardefs_correctly() {
        let state = TeXState::new();

        state.set_math_chardef(
            false,
            &Token::ControlSequence("hello".to_string()),
            &MathCode::from_number(0x7161),
        );
        assert_eq!(
            state
                .get_math_chardef(&Token::ControlSequence("hello".to_string())),
            Some(MathCode::from_number(0x7161))
        );
    }

    #[test]
    fn it_does_not_throw_when_accessing_invalid_fonts() {
        let state = TeXState::new();

        let fake_font = Font {
            font_name: "not_a_real_font".to_string(),
            scale: Dimen::from_unit(1.0, Unit::Point),
        };

        assert_eq!(state.get_metrics_for_font(&fake_font).is_none(), true);
        assert_eq!(
            state
                .with_metrics_for_font(&fake_font, |_metrics| {
                    panic!("Shouldn't reach here");
                })
                .is_none(),
            true
        );
    }

    #[test]
    fn it_gets_and_sets_fonts_correctly() {
        let state = TeXState::new();

        state.set_fontdef(
            false,
            &Token::ControlSequence("abc".to_string()),
            &Font {
                font_name: "cmr7".to_string(),
                scale: Dimen::from_unit(7.0, Unit::Point),
            },
        );

        assert_eq!(
            state.get_fontdef(&Token::ControlSequence("abc".to_string())),
            Some(Font {
                font_name: "cmr7".to_string(),
                scale: Dimen::from_unit(7.0, Unit::Point),
            })
        );
    }

    #[test]
    fn it_gets_and_sets_the_current_font_correctly() {
        let state = TeXState::new();

        state.set_current_font(
            false,
            &Font {
                font_name: "cmr10".to_string(),
                scale: Dimen::from_unit(5.0, Unit::Point),
            },
        );

        assert_eq!(
            state.get_current_font(),
            Font {
                font_name: "cmr10".to_string(),
                scale: Dimen::from_unit(5.0, Unit::Point),
            }
        );
    }

    #[test]
    fn it_allows_for_temporary_access_of_font_metrics() {
        let state = TeXState::new();

        let font = Font {
            font_name: "cmr10".to_string(),
            scale: Dimen::from_unit(10.0, Unit::Point),
        };

        assert_eq!(
            state.with_metrics_for_font(&font, |metrics| {
                metrics.get_checksum()
            }),
            Some(1274110073)
        );
    }

    #[test]
    fn it_gets_and_sets_glue_parameters_correctly() {
        let state = TeXState::new();

        let zero = Glue::zero();
        let one = Glue::from_dimen(Dimen::from_unit(1.0, Unit::Point));

        assert_eq!(
            state.get_glue_parameter(&GlueParameter::ParSkip),
            Glue {
                space: Dimen::zero(),
                stretch: SpringDimen::Dimen(Dimen::from_unit(1.0, Unit::Point)),
                shrink: SpringDimen::Dimen(Dimen::zero()),
            },
        );

        state.set_glue_parameter(false, &GlueParameter::ParSkip, &one);
        assert_eq!(state.get_glue_parameter(&GlueParameter::ParSkip), one);

        assert_eq!(state.get_glue_parameter(&GlueParameter::SpaceSkip), zero,);

        state.set_glue_parameter(false, &GlueParameter::SpaceSkip, &one);
        assert_eq!(state.get_glue_parameter(&GlueParameter::SpaceSkip), one);
    }
}
