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

    fn lower_place(&mut self, fn_body: &hir::FnBody, place: hir::Place) -> mir::Place {
        match fn_body.tables[place] {
            hir::PlaceData::Variable(variable) => {
                let mir_variable = self.lower_variable(fn_body, variable);
                self.add(fn_body.span(place), mir::PlaceData::Variable(mir_variable))
            }
            _ => unimplemented!("Do not yet support non-variable places"),
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
                let place = self.lower_place(fn_body, place);

                let statement = self.add(
                    fn_body.span(expression),
                    mir::StatementData {
                        kind: mir::StatementKind::Expression(mir::Rvalue::Use(mir::Operand::Copy(
                            place,
                        ))),
                    },
                );
                statements.push(statement);
            }
            _ => unimplemented!("Expression kind not yet support in MIR"),
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
        let typed_expressions = self.db.base_type_check(self.item_entity);
        let fn_body = self.db.fn_body(self.item_entity);

        self.lower_basic_block(&fn_body.value, fn_body.value.root_expression);
        /*
        match fn_body.value.tables[fn_body.value.root_expression] {
            hir::ExpressionData::Place { place, perm } => match fn_body.value.tables[place] {
                hir::PlaceData::Variable(variable) => {
                    let variable_data = fn_body.value.tables[variable];
                    let variable_type = typed_expressions.value.ty(variable).base.untern(self.db);
        
                    let boolean_entity = EntityData::LangItem(LangItem::Boolean).intern(self.db);
                    let uint_entity = EntityData::LangItem(LangItem::Uint).intern(self.db);
        
                    match variable_type.kind {
                        lark_ty::BaseKind::Named(entity) => {
                            if entity == boolean_entity {
                                println!("boolean");
                            } else if entity == uint_entity {
                                println!("uint");
                            }
                        }
                        _ => println!("Unknown basetype kind"),
                    }
                }
                _ => unimplemented!("Not a variable"),
            },
            _ => unimplemented!("Not a variable"),
        }
        */

        mir::FnBytecode {
            basic_blocks: mir::List::default(),
            tables: self.fn_bytecode_tables,
        }
    }
}
