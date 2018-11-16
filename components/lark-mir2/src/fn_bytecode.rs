use crate as mir;
use crate::MirDatabase;
use intern::{Intern, Untern};
use lark_entity::{Entity, EntityData, LangItem};
use lark_error::Diagnostic;
use lark_error::WithError;
use lark_hir as hir;
use lark_string::global::GlobalIdentifier;
use map::FxIndexMap;
use parser::pos::{HasSpan, Span, Spanned};
use std::sync::Arc;

crate fn fn_bytecode(
    db: &impl MirDatabase,
    item_entity: Entity,
) -> WithError<Arc<crate::FnBytecode>> {
    let mut errors = vec![];
    let fn_bytecode = MirLower::new(db, item_entity, &mut errors).lower_typecheck_of_item();
    WithError {
        value: Arc::new(fn_bytecode),
        errors,
    }
}

struct MirLower<'me, DB: MirDatabase> {
    db: &'me DB,
    item_entity: Entity,
    fn_bytecode_tables: mir::FnBytecodeTables,
    //variables: FxIndexMap<GlobalIdentifier, hir::Variable>,
    errors: &'me mut Vec<Diagnostic>,
}

impl<'me, DB> MirLower<'me, DB>
where
    DB: MirDatabase,
{
    fn new(db: &'me DB, item_entity: Entity, errors: &'me mut Vec<Diagnostic>) -> Self {
        MirLower {
            db,
            errors,
            item_entity,
            fn_bytecode_tables: Default::default(),
            //variables: Default::default(),
        }
    }

    fn add<D: mir::MirIndexData>(&mut self, span: Span, node: D) -> D::Index {
        D::index_vec_mut(&mut self.fn_bytecode_tables).push(Spanned(node, span))
    }

    fn span(&self, index: impl mir::SpanIndex) -> Span {
        index.span_from(&self.fn_bytecode_tables)
    }

    fn lower_identifier(
        &mut self,
        fn_body: &hir::FnBody,
        identifier: hir::Identifier,
    ) -> mir::Identifier {
        match fn_body.tables[identifier] {
            hir::IdentifierData { text } => {
                self.add(fn_body.span(identifier), mir::IdentifierData { text })
            }
        }
    }

    fn lower_variable(&mut self, fn_body: &hir::FnBody, variable: hir::Variable) -> mir::Variable {
        match fn_body.tables[variable] {
            hir::VariableData { name } => {
                let mir_identifier = self.lower_identifier(fn_body, name);
                self.add(
                    fn_body.span(variable),
                    mir::VariableData {
                        name: mir_identifier,
                    },
                )
            }
        }
    }

    fn lower_operand(
        &mut self,
        fn_body: &hir::FnBody,
        expression: hir::Expression,
        statements: &mut Vec<mir::Statement>,
    ) -> mir::Operand {
        match fn_body.tables[expression] {
            hir::ExpressionData::Place { place, .. } => {
                let place = self.lower_place(fn_body, place);
                self.add(fn_body.span(expression), mir::OperandData::Copy(place))
            }
            _ => unimplemented!("Unsupported expression for operands"),
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

    fn lower_statement(
        &mut self,
        fn_body: &hir::FnBody,
        expression: hir::Expression,
        statements: &mut Vec<mir::Statement>,
    ) {
        match fn_body.tables[expression] {
            hir::ExpressionData::Place { place, .. } => {
                let operand = self.lower_operand(fn_body, expression, statements);
                let rvalue = self.add(fn_body.span(expression), mir::RvalueData::Use(operand));
                let statement = self.add(
                    fn_body.span(expression),
                    mir::StatementData {
                        kind: mir::StatementKind::Expression(rvalue),
                    },
                );

                statements.push(statement);
            }
            hir::ExpressionData::Call {
                function,
                arguments,
            } => {
                let mut args = vec![];

                for argument in arguments.iter(fn_body) {
                    args.push(self.lower_operand(fn_body, argument, statements));
                }

                let call_arguments = mir::List::from_iterator(&mut self.fn_bytecode_tables, args);

                let entity = match fn_body.tables[function] {
                    hir::PlaceData::Entity(entity) => entity,
                    _ => unimplemented!("Call to non-entity"),
                };

                let rvalue = self.add(
                    fn_body.span(expression),
                    mir::RvalueData::Call(entity, call_arguments),
                );

                let statement = self.add(
                    fn_body.span(expression),
                    mir::StatementData {
                        kind: mir::StatementKind::Expression(rvalue),
                    },
                );
                statements.push(statement);
            }
            hir::ExpressionData::Unit {} => {}
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

    fn lower_typecheck_of_item(mut self) -> mir::FnBytecode {
        let typed_expressions = self
            .db
            .base_type_check(self.item_entity)
            .accumulate_errors_into(&mut self.errors);
        let fn_body = self
            .db
            .fn_body(self.item_entity)
            .accumulate_errors_into(&mut self.errors);

        let basic_block = self.lower_basic_block(&fn_body, fn_body.root_expression);

        let basic_blocks = vec![basic_block];

        mir::FnBytecode {
            basic_blocks: mir::List::from_iterator(&mut self.fn_bytecode_tables, basic_blocks),
            tables: self.fn_bytecode_tables,
        }
    }
}
