use std::collections::HashMap;
use veryl_parser::veryl_grammar_trait::*;
use veryl_parser::veryl_token::VerylToken;
use veryl_parser::veryl_walker::VerylWalker;
use veryl_parser::ParolLocation;

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub struct Location {
    pub line: usize,
    pub column: usize,
    pub length: usize,
}

impl From<&ParolLocation> for Location {
    fn from(x: &ParolLocation) -> Self {
        Self {
            line: x.line,
            column: x.column,
            length: x.length,
        }
    }
}

impl From<ParolLocation> for Location {
    fn from(x: ParolLocation) -> Self {
        Self {
            line: x.line,
            column: x.column,
            length: x.length,
        }
    }
}

#[derive(Default)]
pub struct Align {
    index: usize,
    max_width: usize,
    width: usize,
    line: usize,
    rest: Vec<(Location, usize)>,
    additions: HashMap<Location, usize>,
    last_token: Option<VerylToken>,
}

impl Align {
    fn finish_group(&mut self) {
        for (loc, width) in &self.rest {
            self.additions.insert(*loc, self.max_width - width);
        }
        self.rest.clear();
        self.max_width = 0;
    }

    fn finish_item(&mut self) {
        let last_token = self.last_token.take();
        if let Some(last_token) = last_token {
            let loc: Location = (&last_token.token.token.location).into();
            if loc.line - self.line > 1 {
                self.finish_group();
            }
            self.max_width = usize::max(self.max_width, self.width);
            self.line = loc.line;
            self.rest.push((loc, self.width));

            self.width = 0;
            self.index += 1;
        }
    }

    fn start_item(&mut self) {
        self.width = 0;
    }

    fn token(&mut self, x: &VerylToken) {
        self.width += x.token.token.location.length;
        self.last_token = Some(x.clone());
    }

    fn dummy_token(&mut self, x: &VerylToken) {
        self.width += 0; // 0 length token
        self.last_token = Some(x.clone());
    }

    fn space(&mut self, x: usize) {
        self.width += x;
    }
}

mod align_kind {
    pub const IDENTIFIER: usize = 0;
    pub const TYPE: usize = 1;
    pub const EXPRESSION: usize = 2;
    pub const WIDTH: usize = 3;
}

#[derive(Default)]
pub struct Aligner {
    pub additions: HashMap<Location, usize>,
    aligns: [Align; 4],
}

impl Aligner {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn align(&mut self, input: &Veryl) {
        self.veryl(input);
        self.finish_group();
        for align in &self.aligns {
            for (x, y) in &align.additions {
                self.additions
                    .entry(*x)
                    .and_modify(|val| *val += *y)
                    .or_insert(*y);
            }
        }
    }

    fn finish_group(&mut self) {
        for i in 0..self.aligns.len() {
            self.aligns[i].finish_group();
        }
    }

    fn insert(&mut self, token: &VerylToken, width: usize) {
        let loc: Location = (&token.token.token.location).into();
        self.additions
            .entry(loc)
            .and_modify(|val| *val += width)
            .or_insert(width);
    }
}

impl VerylWalker for Aligner {
    /// Semantic action for non-terminal 'Identifier'
    fn identifier(&mut self, arg: &Identifier) {
        self.aligns[align_kind::IDENTIFIER].start_item();
        self.aligns[align_kind::IDENTIFIER].token(&arg.identifier_token);
        self.aligns[align_kind::IDENTIFIER].finish_item();
    }

    /// Semantic action for non-terminal 'Number'
    fn number(&mut self, arg: &Number) {
        let token = match arg {
            Number::Number0(x) => match &*x.integral_number {
                IntegralNumber::IntegralNumber0(x) => &x.based.based_token,
                IntegralNumber::IntegralNumber1(x) => &x.base_less.base_less_token,
                IntegralNumber::IntegralNumber2(x) => &x.all_bit.all_bit_token,
            },
            Number::Number1(x) => match &*x.real_number {
                RealNumber::RealNumber0(x) => &x.fixed_point.fixed_point_token,
                RealNumber::RealNumber1(x) => &x.exponent.exponent_token,
            },
        };
        self.aligns[align_kind::EXPRESSION].token(token);
        self.aligns[align_kind::WIDTH].token(token);
    }

    /// Semantic action for non-terminal 'Expression'
    fn expression(&mut self, arg: &Expression) {
        self.expression1(&arg.expression1);
        for x in &arg.expression_list {
            self.aligns[align_kind::EXPRESSION].space(1);
            self.aligns[align_kind::WIDTH].space(1);
            let token = match &*x.expression_list_group {
                ExpressionListGroup::ExpressionListGroup0(x) => {
                    &x.binary_operator.binary_operator_token
                }
                ExpressionListGroup::ExpressionListGroup1(x) => {
                    &x.common_operator.common_operator_token
                }
            };
            self.aligns[align_kind::EXPRESSION].token(token);
            self.aligns[align_kind::WIDTH].token(token);
            self.aligns[align_kind::EXPRESSION].space(1);
            self.aligns[align_kind::WIDTH].space(1);
            self.expression1(&x.expression1);
        }
    }

    /// Semantic action for non-terminal 'Expression1'
    fn expression1(&mut self, arg: &Expression1) {
        if let Some(ref x) = arg.expression1_opt {
            let token = match &*x.expression1_opt_group {
                Expression1OptGroup::Expression1OptGroup0(x) => {
                    &x.unary_operator.unary_operator_token
                }
                Expression1OptGroup::Expression1OptGroup1(x) => {
                    &x.common_operator.common_operator_token
                }
            };
            self.aligns[align_kind::EXPRESSION].token(token);
            self.aligns[align_kind::WIDTH].token(token);
        }
        self.factor(&arg.factor);
    }

    /// Semantic action for non-terminal 'Factor'
    fn factor(&mut self, arg: &Factor) {
        match arg {
            Factor::Factor0(x) => self.number(&x.number),
            Factor::Factor1(x) => {
                self.aligns[align_kind::EXPRESSION].token(&x.identifier.identifier_token);
                self.aligns[align_kind::WIDTH].token(&x.identifier.identifier_token);
                for x in &x.factor_list {
                    self.range(&x.range);
                }
            }
            Factor::Factor2(x) => {
                self.aligns[align_kind::EXPRESSION].token(&x.l_paren.l_paren_token);
                self.aligns[align_kind::WIDTH].token(&x.l_paren.l_paren_token);
                self.expression(&x.expression);
                self.aligns[align_kind::EXPRESSION].token(&x.r_paren.r_paren_token);
                self.aligns[align_kind::WIDTH].token(&x.r_paren.r_paren_token);
            }
        }
    }

    /// Semantic action for non-terminal 'Range'
    fn range(&mut self, arg: &Range) {
        self.aligns[align_kind::EXPRESSION].token(&arg.l_bracket.l_bracket_token);
        self.aligns[align_kind::WIDTH].token(&arg.l_bracket.l_bracket_token);
        self.expression(&arg.expression);
        if let Some(ref x) = arg.range_opt {
            self.aligns[align_kind::EXPRESSION].token(&x.colon.colon_token);
            self.aligns[align_kind::WIDTH].token(&x.colon.colon_token);
            self.expression(&x.expression);
        }
        self.aligns[align_kind::EXPRESSION].token(&arg.r_bracket.r_bracket_token);
        self.aligns[align_kind::WIDTH].token(&arg.r_bracket.r_bracket_token);
    }

    /// Semantic action for non-terminal 'Width'
    fn width(&mut self, arg: &Width) {
        self.aligns[align_kind::EXPRESSION].token(&arg.l_bracket.l_bracket_token);
        self.aligns[align_kind::WIDTH].token(&arg.l_bracket.l_bracket_token);
        self.expression(&arg.expression);
        self.aligns[align_kind::EXPRESSION].space("-1:0".len());
        self.aligns[align_kind::WIDTH].space("-1:0".len());
        self.aligns[align_kind::EXPRESSION].token(&arg.r_bracket.r_bracket_token);
        self.aligns[align_kind::WIDTH].token(&arg.r_bracket.r_bracket_token);
    }

    /// Semantic action for non-terminal 'Type'
    fn r#type(&mut self, arg: &Type) {
        let token = match &*arg.type_group {
            TypeGroup::TypeGroup0(x) => match &*x.builtin_type {
                BuiltinType::BuiltinType0(x) => x.logic.logic_token.clone(),
                BuiltinType::BuiltinType1(x) => x.bit.bit_token.clone(),
                BuiltinType::BuiltinType2(x) => x.u32.u32_token.replace("unsigned int"),
                BuiltinType::BuiltinType3(x) => x.u64.u64_token.replace("unsigned longint"),
                BuiltinType::BuiltinType4(x) => x.i32.i32_token.replace("signed int"),
                BuiltinType::BuiltinType5(x) => x.i64.i64_token.replace("signed longint"),
                BuiltinType::BuiltinType6(x) => x.f32.f32_token.replace("real"),
                BuiltinType::BuiltinType7(x) => x.f64.f64_token.replace("longreal"),
            },
            TypeGroup::TypeGroup1(x) => x.identifier.identifier_token.clone(),
        };
        self.aligns[align_kind::TYPE].start_item();
        self.aligns[align_kind::TYPE].token(&token);
        self.aligns[align_kind::TYPE].finish_item();

        if arg.type_list.is_empty() {
            self.aligns[align_kind::WIDTH].start_item();
            self.aligns[align_kind::WIDTH].dummy_token(&token);
            self.aligns[align_kind::WIDTH].finish_item();
        } else {
            self.aligns[align_kind::WIDTH].start_item();
            for x in &arg.type_list {
                self.width(&x.width);
            }
            self.aligns[align_kind::WIDTH].finish_item();
        }
    }

    /// Semantic action for non-terminal 'AssignmentStatement'
    fn assignment_statement(&mut self, arg: &AssignmentStatement) {
        self.identifier(&arg.identifier);
    }

    /// Semantic action for non-terminal 'IfStatement'
    fn if_statement(&mut self, _arg: &IfStatement) {}

    /// Semantic action for non-terminal 'ParameterDeclaration'
    fn parameter_declaration(&mut self, arg: &ParameterDeclaration) {
        self.insert(&arg.parameter.parameter_token, 1);
        self.identifier(&arg.identifier);
        self.r#type(&arg.r#type);
    }

    /// Semantic action for non-terminal 'LocalparamDeclaration'
    fn localparam_declaration(&mut self, arg: &LocalparamDeclaration) {
        self.identifier(&arg.identifier);
        self.r#type(&arg.r#type);
    }

    /// Semantic action for non-terminal 'AlwaysFfDeclaration'
    fn always_ff_declaration(&mut self, _arg: &AlwaysFfDeclaration) {}

    /// Semantic action for non-terminal 'WithParameterItem'
    fn with_parameter_item(&mut self, arg: &WithParameterItem) {
        match &*arg.with_parameter_item_group {
            WithParameterItemGroup::WithParameterItemGroup0(x) => {
                self.insert(&x.parameter.parameter_token, 1);
            }
            WithParameterItemGroup::WithParameterItemGroup1(_) => (),
        }
        self.identifier(&arg.identifier);
        self.r#type(&arg.r#type);
        self.aligns[align_kind::EXPRESSION].start_item();
        self.expression(&arg.expression);
        self.aligns[align_kind::EXPRESSION].finish_item();
    }

    /// Semantic action for non-terminal 'ModuleDeclaration'
    fn module_declaration(&mut self, arg: &ModuleDeclaration) {
        if let Some(ref x) = arg.module_declaration_opt {
            self.with_parameter(&x.with_parameter);
        }
        if let Some(ref x) = arg.module_declaration_opt0 {
            self.module_port(&x.module_port);
        }
        for x in &arg.module_declaration_list {
            self.module_item(&x.module_item);
        }
    }

    /// Semantic action for non-terminal 'Direction'
    fn direction(&mut self, arg: &Direction) {
        match arg {
            Direction::Direction0(x) => {
                self.insert(&x.input.input_token, 1);
            }
            Direction::Direction1(_) => (),
            Direction::Direction2(x) => {
                self.insert(&x.inout.inout_token, 1);
            }
        }
    }

    /// Semantic action for non-terminal 'InterfaceDeclaration'
    fn interface_declaration(&mut self, arg: &InterfaceDeclaration) {
        if let Some(ref x) = arg.interface_declaration_opt {
            self.with_parameter(&x.with_parameter);
        }
        for x in &arg.interface_declaration_list {
            self.interface_item(&x.interface_item);
        }
    }
}
