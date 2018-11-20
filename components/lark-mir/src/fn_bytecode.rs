use crate as mir;
use crate::MirDatabase;
use intern::Intern;
use lark_entity::Entity;
use lark_error::Diagnostic;
use lark_error::WithError;
use lark_hir as hir;
use lark_span::{FileName, Span, Spanned};
use lark_string::global::GlobalIdentifier;
use map::FxIndexMap;
use std::sync::Arc;

crate fn fn_bytecode(
    db: &impl MirDatabase,
    item_entity: Entity,
) -> WithError<Arc<crate::FnBytecode>> {
    let mut errors = vec![];
    let fn_bytecode = MirLower::new(db, item_entity, &mut errors).lower_to_bytecode();
    WithError {
        value: Arc::new(fn_bytecode),
        errors,
    }
}

struct MirLower<'me, DB: MirDatabase> {
    db: &'me DB,
    item_entity: Entity,
    fn_bytecode_tables: mir::FnBytecodeTables,
    variables: FxIndexMap<GlobalIdentifier, mir::Variable>,
    //errors: &'me mut Vec<Diagnostic>,
    next_temporary_id: usize,
}

impl<'me, DB> MirLower<'me, DB>
where
    DB: MirDatabase,
{
    fn new(db: &'me DB, item_entity: Entity, _errors: &'me mut Vec<Diagnostic>) -> Self {
        MirLower {
            db,
            //errors,
            item_entity,
            fn_bytecode_tables: Default::default(),
            variables: Default::default(),
            next_temporary_id: 0,
        }
    }

    fn add<D: mir::MirIndexData>(&mut self, span: Span<FileName>, node: D) -> D::Index {
        D::index_vec_mut(&mut self.fn_bytecode_tables).push(Spanned::new(node, span))
    }

    fn save_scope(&self) -> FxIndexMap<GlobalIdentifier, mir::Variable> {
        self.variables.clone()
    }

    fn restore_scope(&mut self, scope: FxIndexMap<GlobalIdentifier, mir::Variable>) {
        self.variables = scope;
    }

    /// Brings a variable into scope, returning anything that was shadowed.
    fn bring_into_scope(&mut self, variable: mir::Variable) {
        let name = self[variable].name;
        self.variables.insert(self[name].text, variable);
    }

    /*
    fn span(&self, index: impl mir::SpanIndex) -> Span<FileName> {
        index.span_from(&self.fn_bytecode_tables)
    }
    */

    fn create_temporary(&mut self, span: Span<FileName>) -> mir::Variable {
        let temp_variable_name = format!("_tmp{}", self.next_temporary_id).intern(&mut self.db);
        let temp_identifier = self.add(
            span,
            mir::IdentifierData {
                text: temp_variable_name,
            },
        );
        self.next_temporary_id += 1;
        self.add(
            span,
            mir::VariableData {
                name: temp_identifier,
            },
        )
    }

    fn lower_variable(&mut self, fn_body: &hir::FnBody, variable: hir::Variable) -> mir::Variable {
        match fn_body.tables[variable] {
            hir::VariableData { name } => match fn_body.tables[name] {
                hir::IdentifierData { text } => {
                    if let Some(&variable) = self.variables.get(&text) {
                        variable
                    } else {
                        let mir_identifier =
                            self.add(fn_body.span(variable), mir::IdentifierData { text });

                        self.add(
                            fn_body.span(variable),
                            mir::VariableData {
                                name: mir_identifier,
                            },
                        )
                    }
                }
            },
        }
    }

    fn lower_call(
        &mut self,
        fn_body: &hir::FnBody,
        function: hir::Place,
        arguments: hir::List<hir::Expression>,
        statements: &mut Vec<mir::Statement>,
    ) -> (Entity, mir::List<mir::Operand>, Vec<mir::Variable>) {
        let mut args = vec![];
        let mut temp_vars = vec![];

        for argument in arguments.iter(fn_body) {
            let (arg_operand, mut arg_temp_vars) =
                self.lower_operand(fn_body, argument, statements);
            args.push(arg_operand);
            arg_temp_vars.append(&mut temp_vars);
            temp_vars = arg_temp_vars;
        }

        let call_arguments = mir::List::from_iterator(&mut self.fn_bytecode_tables, args);

        let entity = match fn_body.tables[function] {
            hir::PlaceData::Entity(entity) => entity,
            _ => unimplemented!("Call to non-entity"),
        };

        (entity, call_arguments, temp_vars)
    }

    fn lower_operand(
        &mut self,
        fn_body: &hir::FnBody,
        expression: hir::Expression,
        statements: &mut Vec<mir::Statement>,
    ) -> (mir::Operand, Vec<mir::Variable>) {
        match fn_body.tables[expression] {
            hir::ExpressionData::Place { place, .. } => {
                let place = self.lower_place(fn_body, place);
                (
                    self.add(fn_body.span(expression), mir::OperandData::Copy(place)),
                    vec![],
                )
            }
            hir::ExpressionData::Call {
                function,
                arguments,
            } => {
                let (entity, call_arguments, mut temp_vars) =
                    self.lower_call(fn_body, function, arguments, statements);
                let new_temp_var = self.create_temporary(fn_body.span(expression));

                // Start the variable scope
                let statement = self.add(
                    fn_body.span(expression),
                    mir::StatementData {
                        kind: mir::StatementKind::StorageLive(new_temp_var),
                    },
                );
                statements.push(statement);

                // Assign this call to the temp variable
                let rvalue = self.add(
                    fn_body.span(expression),
                    mir::RvalueData::Call(entity, call_arguments),
                );
                let lvalue = self.add(
                    fn_body.span(expression),
                    mir::PlaceData::Variable(new_temp_var),
                );
                let statement = self.add(
                    fn_body.span(expression),
                    mir::StatementData {
                        kind: mir::StatementKind::Assign(lvalue, rvalue),
                    },
                );
                statements.push(statement);

                // Record that we've created this temporary variable for later dropping
                temp_vars.insert(0, new_temp_var);

                // Finally, create the operand we'll use to refer to this call
                let operand = self.add(fn_body.span(expression), mir::OperandData::Copy(lvalue));

                (operand, temp_vars)
            }
            _ => unimplemented!("Unsupported expression for operands"),
        }
    }

    fn lower_rvalue(
        &mut self,
        fn_body: &hir::FnBody,
        expression: hir::Expression,
        statements: &mut Vec<mir::Statement>,
    ) -> (mir::Rvalue, Vec<mir::Variable>) {
        match fn_body.tables[expression] {
            hir::ExpressionData::Place { place, .. } => {
                let place = self.lower_place(fn_body, place);
                let operand = self.add(fn_body.span(expression), mir::OperandData::Copy(place));
                (
                    self.add(fn_body.span(expression), mir::RvalueData::Use(operand)),
                    vec![],
                )
            }
            hir::ExpressionData::Call {
                function,
                arguments,
            } => {
                let (entity, call_arguments, temp_vars) =
                    self.lower_call(fn_body, function, arguments, statements);

                let rvalue = self.add(
                    fn_body.span(expression),
                    mir::RvalueData::Call(entity, call_arguments),
                );

                (rvalue, temp_vars)
            }
            _ => unimplemented!("Unsupported expression for rvalues"),
        }
    }

    fn lower_place(&mut self, fn_body: &hir::FnBody, place: hir::Place) -> mir::Place {
        match fn_body.tables[place] {
            hir::PlaceData::Variable(variable) => {
                let mir_variable = self.lower_variable(fn_body, variable);
                self.add(fn_body.span(place), mir::PlaceData::Variable(mir_variable))
            }
            hir::PlaceData::Entity(entity) => {
                self.add(fn_body.span(place), mir::PlaceData::Entity(entity))
            }
            x => unimplemented!("Do not yet support lowering place: {:#?}", x),
        }
    }

    fn drain_temp_variables(
        &mut self,
        span: Span<FileName>,
        temp_vars: Vec<mir::Variable>,
        statements: &mut Vec<mir::Statement>,
    ) {
        for temp_var in temp_vars {
            let statement = self.add(
                span,
                mir::StatementData {
                    kind: mir::StatementKind::StorageDead(temp_var),
                },
            );
            statements.push(statement);
        }
    }

    fn lower_statement(
        &mut self,
        fn_body: &hir::FnBody,
        expression: hir::Expression,
        statements: &mut Vec<mir::Statement>,
    ) {
        match fn_body.tables[expression] {
            hir::ExpressionData::Place { .. } | hir::ExpressionData::Call { .. } => {
                let (rvalue, temp_vars) = self.lower_rvalue(fn_body, expression, statements);
                let statement = self.add(
                    fn_body.span(expression),
                    mir::StatementData {
                        kind: mir::StatementKind::Expression(rvalue),
                    },
                );

                statements.push(statement);
                self.drain_temp_variables(fn_body.span(expression), temp_vars, statements);
            }
            hir::ExpressionData::Unit {} => {}
            hir::ExpressionData::Sequence { first, second } => {
                self.lower_statement(fn_body, first, statements);
                self.lower_statement(fn_body, second, statements);
            }
            hir::ExpressionData::Let {
                variable,
                initializer,
                body,
            } => {
                let saved_scope = self.save_scope();

                let mir_variable = self.lower_variable(fn_body, variable);
                // Start the variable scope
                let statement = self.add(
                    fn_body.span(expression),
                    mir::StatementData {
                        kind: mir::StatementKind::StorageLive(mir_variable),
                    },
                );
                statements.push(statement);

                // Initialize if there is an intializer
                match initializer {
                    Some(initial_value) => {
                        let (rvalue, temp_vars) =
                            self.lower_rvalue(fn_body, initial_value, statements);

                        let lvalue = self.add(
                            fn_body.span(expression),
                            mir::PlaceData::Variable(mir_variable),
                        );
                        let statement = self.add(
                            fn_body.span(expression),
                            mir::StatementData {
                                kind: mir::StatementKind::Assign(lvalue, rvalue),
                            },
                        );
                        statements.push(statement);
                        self.drain_temp_variables(fn_body.span(expression), temp_vars, statements);
                    }
                    None => {}
                }

                self.bring_into_scope(mir_variable);

                // Body of the let
                self.lower_statement(fn_body, body, statements);

                self.restore_scope(saved_scope);

                // End the variable scope
                let statement = self.add(
                    fn_body.span(expression),
                    mir::StatementData {
                        kind: mir::StatementKind::StorageDead(mir_variable),
                    },
                );
                statements.push(statement);
            }
            x => unimplemented!("Expression kind not yet support in MIR: {:#?}", x),
        }
    }

    fn lower_basic_block(
        &mut self,
        fn_body: &hir::FnBody,
        expression: hir::Expression,
    ) -> mir::BasicBlock {
        let mut statements = vec![];
        self.lower_statement(fn_body, expression, &mut statements);

        let statements = mir::List::from_iterator(&mut self.fn_bytecode_tables, statements);

        //FIXME: Fix this span once we nail down terminators
        self.add(
            fn_body.span(expression),
            mir::BasicBlockData {
                statements,
                terminator: mir::Terminator::Return,
            },
        )
    }

    fn lower_arguments(&mut self, fn_body: &hir::FnBody) -> Vec<mir::Variable> {
        let mut args = vec![];
        for argument in fn_body.arguments.iter(fn_body) {
            args.push(self.lower_variable(fn_body, argument));
        }

        args
    }

    fn lower_to_bytecode(mut self) -> mir::FnBytecode {
        let fn_body = self.db.fn_body(self.item_entity).value;
        let arguments = self.lower_arguments(&fn_body);

        for argument in &arguments {
            self.bring_into_scope(*argument);
        }

        let basic_blocks = vec![self.lower_basic_block(&fn_body, fn_body.root_expression)];

        let mir_basic_blocks = mir::List::from_iterator(&mut self.fn_bytecode_tables, basic_blocks);
        let mir_arguments = mir::List::from_iterator(&mut self.fn_bytecode_tables, arguments);

        mir::FnBytecode {
            basic_blocks: mir_basic_blocks,
            tables: self.fn_bytecode_tables,
            arguments: mir_arguments,
        }
    }
}

impl<'me, DB, I> std::ops::Index<I> for MirLower<'me, DB>
where
    DB: MirDatabase,
    I: mir::MirIndex,
{
    type Output = I::Data;

    fn index(&self, index: I) -> &I::Data {
        &self.fn_bytecode_tables[index]
    }
}
