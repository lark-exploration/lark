use crate::HirLocation;
use crate::TypeCheckDatabase;
use crate::TypeChecker;
use crate::TypeCheckerFamily;
use crate::TypeCheckerFamilyDependentExt;
use crate::TypeCheckerVariableExt;
use lark_collections::FxIndexSet;
use lark_debug_derive::DebugWith;
use lark_debug_with::DebugWith;
use lark_entity::{Entity, EntityData, ItemKind, LangItem, MemberKind};
use lark_error::ErrorReported;
use lark_error::ErrorSentinel;
use lark_hir as hir;
use lark_intern::Untern;
use lark_pretty_print::PrettyPrint;
use lark_ty::declaration::Declaration;
use lark_ty::Signature;
use lark_ty::Ty;
use lark_ty::{BaseData, BaseKind};
use lark_unify::InferVar;
use lark_unify::Inferable;

#[derive(Copy, Clone, Debug, DebugWith)]
enum Mode<F: TypeCheckerFamily> {
    Synthesize,
    CheckType(Ty<F>, HirLocation),
}
use self::Mode::*;

impl<DB, F, S> TypeChecker<'_, DB, F, S>
where
    DB: TypeCheckDatabase,
    F: TypeCheckerFamily,
    Self: TypeCheckerFamilyDependentExt<F>,
    F::Base: Inferable<F::InternTables, KnownData = BaseData<F>>,
{
    crate fn check_fn_body(&mut self) -> Vec<InferVar> {
        let hir_arguments_len = self.hir.arguments.map(|l| l.len()).unwrap_or(0);
        let declaration_signature = self
            .db
            .signature(self.fn_entity)
            .into_value()
            .unwrap_or_else(|ErrorReported(_)| {
                <Signature<Declaration>>::error_sentinel(self, hir_arguments_len)
            });
        let placeholders = self.placeholders_for(self.fn_entity);
        let signature = self.substitute(
            self.hir.root_expression,
            &placeholders,
            declaration_signature,
        );
        let hir = self.hir.clone();
        if let Ok(hir_arguments) = self.hir.arguments {
            assert_eq!(signature.inputs.len(), hir_arguments.len());
            for (argument, &input) in hir_arguments.iter(&hir).zip(signature.inputs.iter()) {
                self.record_variable_ty(argument, input);
            }
        }
        self.check_expression(
            CheckType(signature.output, HirLocation::Return),
            self.hir.root_expression,
        );

        // Complete all deferred type operations; run to steady state.
        loop {
            let vars: Vec<InferVar> = self.unify.drain_events().collect();
            if vars.is_empty() {
                break;
            }
            for var in vars {
                self.trigger_ops(var);
            }
        }

        let mut unresolved_variables = vec![];

        // Look for any deferred operations that never executed. Those
        // variables that they are blocked on must not be resolved; record
        // as an error.
        self.untriggered_ops(&mut unresolved_variables);

        unresolved_variables
    }

    /// Type-check the expression `expression` in the given mode
    /// (either "check", which specifies the type the expression must
    /// have, or "synthesize").
    fn check_expression(&mut self, mode: Mode<F>, expression: hir::Expression) -> Ty<F> {
        let max_ty = self.compute_expression_ty(mode, expression);
        let access_ty = self.record_max_expression_ty(expression, max_ty);

        match mode {
            Synthesize => (),
            CheckType(expected_ty, location) => {
                self.equate(expression, location, access_ty, expected_ty);
            }
        }

        access_ty
    }

    fn type_or_infer_variable(&mut self, mode: Mode<F>) -> Ty<F> {
        match mode {
            Synthesize => self.new_variable(),
            CheckType(expected_ty, _) => expected_ty,
        }
    }

    /// Common helper for checking and synthesizing the type of an expression.
    ///
    /// If `expected_ty` is `None`, this will synthesize. Otherwise, it will consider
    /// `expected_ty` as a hint. Returns the type of the expression.
    ///
    /// **Note:** The expected type is only a hint. If this expression
    /// does not produce a value of `expected_ty`, no error is
    /// reported; that must be enforced by the caller.
    fn compute_expression_ty(&mut self, mode: Mode<F>, expression: hir::Expression) -> Ty<F> {
        let expression_data = self.hir[expression].clone();
        match expression_data {
            hir::ExpressionData::Let {
                variable,
                initializer,
                body,
            } => {
                let variable_ty = self.request_variable_ty(variable);
                if let Some(initializer) = initializer {
                    self.check_expression(CheckType(variable_ty, expression.into()), initializer);
                }
                self.check_expression(mode, body)
            }

            hir::ExpressionData::Place { place } => self.check_place(place),

            hir::ExpressionData::Assignment { place, value } => {
                let place_ty = self.check_place(place);
                self.check_expression(CheckType(place_ty, expression.into()), value);
                self.unit_type()
            }

            hir::ExpressionData::MethodCall { method, arguments } => {
                let owner_expression = arguments.first(&self.hir).unwrap();
                let owner_ty = self.check_expression(Mode::Synthesize, owner_expression);
                self.compute_method_call_ty(expression, owner_ty, method, arguments)
            }

            hir::ExpressionData::Call {
                function,
                arguments,
            } => {
                let function_ty = self.check_place(function);
                self.compute_fn_call_ty(expression, function_ty, arguments)
            }

            hir::ExpressionData::Aggregate { entity, fields } => {
                self.check_aggregate(expression, entity, fields)
            }

            hir::ExpressionData::Sequence { first, second } => {
                self.check_expression(CheckType(self.unit_type(), expression.into()), first);
                self.check_expression(mode, second)
            }

            hir::ExpressionData::If {
                condition,
                if_true,
                if_false,
            } => {
                self.check_expression(CheckType(self.boolean_type(), expression.into()), condition);

                let ty = self.type_or_infer_variable(mode);
                self.check_expression(
                    CheckType(ty, HirLocation::AfterExpression(expression)),
                    if_true,
                );
                self.check_expression(
                    CheckType(ty, HirLocation::AfterExpression(expression)),
                    if_false,
                );

                ty
            }

            hir::ExpressionData::Literal { data } => match data.kind {
                hir::LiteralKind::String => self.string_type(),
                hir::LiteralKind::UnsignedInteger => self.uint_type(),
            },

            hir::ExpressionData::Unit {} => self.unit_type(),

            hir::ExpressionData::Error { error: _ } => self.error_type(),

            hir::ExpressionData::Binary {
                operator,
                left,
                right,
            } => self.check_binary(expression, operator, left, right),

            hir::ExpressionData::Unary { operator, value } => {
                self.check_unary(expression, operator, value)
            }
        }
    }

    /// Type-check `place`, recording and returning the resulting type (which may be
    /// an inference variable).
    fn check_place(&mut self, place: hir::Place) -> Ty<F> {
        let ty = self.compute_place_ty(place);
        self.record_place_ty(place, ty)
    }

    /// Helper for `check_place`.
    fn compute_place_ty(&mut self, place: hir::Place) -> Ty<F> {
        let place_data = self.hir[place];
        match place_data {
            hir::PlaceData::Variable(var) => self.request_variable_ty(var),

            hir::PlaceData::Entity(entity) => {
                if !entity.untern(self).is_value() {
                    self.record_error("cannot access as a value", place);
                    return self.error_type();
                }

                let entity_ty = self.db.ty(entity).into_value();
                let generics = self.record_entity_and_get_generics(place, entity);
                self.substitute(place, &generics, entity_ty)
            }

            hir::PlaceData::Temporary(expr) => self.check_expression(Synthesize, expr),

            hir::PlaceData::Field { owner, name } => {
                let text = self.hir[name].text;
                let owner_ty = self.check_place(owner);
                self.with_base_data(place, place, owner_ty.base, move |this, base_data| {
                    let BaseData { kind, generics } = base_data;
                    match kind {
                        BaseKind::Named(def_id) => {
                            match this.db.member_entity(def_id, MemberKind::Field, text) {
                                Some(field_entity) => {
                                    this.record_entity(name, field_entity);

                                    let field_decl_ty = this.db.ty(field_entity).into_value();
                                    let field_ty = this.substitute(place, &generics, field_decl_ty);
                                    this.apply_owner_perm(place, place, owner_ty.perm, field_ty)
                                }

                                None => {
                                    this.record_error("field not found", name);
                                    this.error_type()
                                }
                            }
                        }

                        BaseKind::Placeholder(_placeholder) => {
                            // Cannot presently access fields from generic types.
                            this.record_error("cannot access fields from generic types(yet)", name);
                            this.error_type()
                        }

                        BaseKind::Error => this.error_type(),
                    }
                })
            }
        }
    }

    /// Helper for `check_expression`: Compute the type from a method call.
    fn compute_fn_call_ty(
        &mut self,
        expression: hir::Expression,
        function_ty: Ty<F>,
        arguments: hir::List<hir::Expression>,
    ) -> Ty<F> {
        self.with_base_data(
            expression,
            expression,
            function_ty.base,
            move |this, base_data| {
                this.check_fn_call(expression, function_ty, arguments, base_data)
            },
        )
    }

    fn check_fn_call(
        &mut self,
        expression: hir::Expression,
        _function_ty: Ty<F>,
        arguments: hir::List<hir::Expression>,
        base_data: BaseData<F>,
    ) -> Ty<F> {
        let BaseData { kind, generics } = base_data;
        match kind {
            BaseKind::Named(entity) => {
                match entity.untern(self) {
                    EntityData::ItemName {
                        kind: ItemKind::Function,
                        ..
                    } => {
                        // You can call this
                    }

                    EntityData::LangItem(LangItem::Debug) => {
                        // You can call into the debug function
                        return self.check_arguments_in_case_of_error(arguments);
                    }

                    _ => {
                        self.record_error("cannot call value of this type", expression);
                        return self.check_arguments_in_case_of_error(arguments);
                    }
                }

                let signature_decl = match self.db.signature(entity).into_value() {
                    Ok(s) => s,
                    Err(ErrorReported(_)) => {
                        <Signature<Declaration>>::error_sentinel(self, arguments.len())
                    }
                };
                let signature = self.substitute(expression, &generics, signature_decl);

                self.check_arguments_against_signature(
                    expression,
                    expression,
                    &signature.inputs[..],
                    signature.output,
                    arguments,
                    0,
                )
            }

            BaseKind::Placeholder(_placeholder) => {
                // Cannot presently invoke generic types.
                self.record_error("cannot call a generic type (yet)", expression);
                return self.check_arguments_in_case_of_error(arguments);
            }

            BaseKind::Error => self.error_type(),
        }
    }

    /// Helper for `check_expression`: Compute the type from a method call.
    fn compute_method_call_ty(
        &mut self,
        expression: hir::Expression,
        owner_access_ty: Ty<F>,
        method_name: hir::Identifier,
        arguments: hir::List<hir::Expression>,
    ) -> Ty<F> {
        self.with_base_data(
            expression,
            expression,
            owner_access_ty.base,
            move |this, base_data| {
                this.check_method_call(
                    expression,
                    owner_access_ty,
                    method_name,
                    arguments,
                    base_data,
                )
            },
        )
    }

    fn check_method_call(
        &mut self,
        expression: hir::Expression,
        owner_access_ty: Ty<F>,
        method_name: hir::Identifier,
        arguments: hir::List<hir::Expression>,
        base_data: BaseData<F>,
    ) -> Ty<F> {
        let BaseData { kind, generics } = base_data;
        match kind {
            BaseKind::Named(def_id) => {
                let text = self.hir[method_name].text;
                let method_entity = match self.db.member_entity(def_id, MemberKind::Method, text) {
                    Some(def_id) => def_id,
                    None => {
                        self.record_error("method not found", expression);
                        return self.error_type();
                    }
                };

                self.record_entity(method_name, method_entity);

                let signature_decl = match self.db.signature(method_entity).into_value() {
                    Ok(s) => s,
                    Err(ErrorReported(_)) => {
                        <Signature<Declaration>>::error_sentinel(self, arguments.len())
                    }
                };
                let signature = self.substitute(expression, &generics, signature_decl);

                // Relate the owner type to the input
                self.equate(expression, expression, owner_access_ty, signature.inputs[0]);

                self.check_arguments_against_signature(
                    method_name,
                    expression,
                    &signature.inputs,
                    signature.output,
                    arguments,
                    1,
                )
            }

            BaseKind::Placeholder(_placeholder) => {
                // Cannot presently invoke methods on generic types.
                self.record_error("cannot invoke methods on generic types(yet)", method_name);
                return self.check_arguments_in_case_of_error(arguments);
            }

            BaseKind::Error => self.error_type(),
        }
    }

    fn check_arguments_against_signature(
        &mut self,
        cause: impl Into<hir::MetaIndex>,
        location: impl Into<HirLocation>,
        inputs: &[Ty<F>],
        output: Ty<F>,
        arguments: hir::List<hir::Expression>,
        skip: usize,
    ) -> Ty<F> {
        let cause: hir::MetaIndex = cause.into();
        let location: HirLocation = location.into();

        log::debug!(
            "check_arguments_against_signature(inputs={:?}, output={:?}, arguments={:?})",
            inputs.debug_with(self),
            output.debug_with(self),
            arguments.debug_with(self),
        );
        if inputs.len() != arguments.len() {
            self.record_error("mismatched argument count", cause);
            return self.check_arguments_in_case_of_error(arguments);
        }

        let hir = &self.hir.clone();
        for (&expected_ty, argument_expr) in inputs.iter().zip(arguments.iter(hir)).skip(skip) {
            self.check_expression(CheckType(expected_ty, location), argument_expr);
        }

        output
    }

    fn check_arguments_in_case_of_error(&mut self, arguments: hir::List<hir::Expression>) -> Ty<F> {
        let hir = &self.hir.clone();
        for argument_expr in arguments.iter(hir) {
            self.check_expression(
                CheckType(self.error_type(), HirLocation::Error),
                argument_expr,
            );
        }
        self.error_type()
    }

    fn check_aggregate(
        &mut self,
        expression: hir::Expression,
        entity: Entity,
        fields: hir::List<hir::IdentifiedExpression>,
    ) -> Ty<F> {
        let generics = self.record_entity_and_get_generics(expression, entity);

        match entity.untern(self) {
            EntityData::ItemName {
                kind: ItemKind::Struct,
                ..
            } => {
                // see code below
            }

            EntityData::Error(_) => {
                // If we can't resolve the type of the struct, then just
                // check the inner expressions. Resolve all the identifiers
                // to error.
                assert!(
                    generics.is_empty(),
                    "generics should be empty, no need to propagate error"
                );
                let error_type = self.error_type();
                let hir = &self.hir.clone();
                for field in fields.iter(hir) {
                    let field_data = self.hir[field];
                    self.record_entity(field_data.identifier, entity);
                    self.check_expression(
                        CheckType(error_type, HirLocation::Error),
                        field_data.expression,
                    );
                }
                return error_type;
            }

            // Something like `def foo() { .. } foo { .. }` is just not legal.
            _ => {
                self.record_error("disallowed expression type", expression);
                self.propagate_error(expression, &generics);
                return self.error_type();
            }
        };

        // Get a vector of **all** the fields.
        let mut missing_members: FxIndexSet<Entity> = match self.db.members(entity) {
            Ok(members) => members.iter().map(|m| m.entity).collect(),
            Err(err) => return Ty::error_sentinel(self, err),
        };

        // Find the entity for each of the field names that the user gave us.
        let hir = &self.hir.clone();
        for field_data in fields.iter_data(hir) {
            let field_name = hir[field_data.identifier].text;
            let field_ty = match self.db.member_entity(entity, MemberKind::Field, field_name) {
                Some(field_entity) => {
                    self.record_entity(field_data.identifier, field_entity);

                    missing_members.remove(&field_entity);

                    let field_ty = self.db.ty(field_entity).into_value();
                    self.substitute(expression, &generics, field_ty)
                }

                None => {
                    self.record_error("unknown field", field_data.identifier);
                    self.error_type()
                }
            };

            // Check the expression against the formal type of this field.
            self.check_expression(
                CheckType(field_ty, expression.into()),
                field_data.expression,
            );
        }

        // If we are missing any members, that's an error.
        for _missing_member in missing_members {
            self.record_error("missing member", expression);

            // Propagate this error to the generics, since they may be
            // underconstrained as a result.
            self.propagate_error(expression, &generics);
        }

        // The final type is the type of the entity with the given
        // generics substituted.
        let entity_ty = self.db.ty(entity).into_value();
        self.substitute(expression, &generics, entity_ty)
    }

    fn check_binary(
        &mut self,
        expression: hir::Expression,
        operator: hir::BinaryOperator,
        left: hir::Expression,
        right: hir::Expression,
    ) -> Ty<F> {
        // For (most) binary operators, we need to know the type of
        // left + right before we can say anything about the result
        // type. So use `with_base_data` to get a callback once that is
        // known.
        let left_ty = self.check_expression(Synthesize, left);
        let right_ty = self.check_expression(Synthesize, right);
        let result_ty = self.with_base_data(
            expression,
            expression,
            left_ty.base,
            move |this, left_base_data| {
                this.with_base_data(
                    expression,
                    expression,
                    right_ty.base,
                    move |this, right_base_data| {
                        this.check_binary_with_both_inputs_known(
                            expression,
                            operator,
                            left_base_data,
                            right_base_data,
                        )
                    },
                )
            },
        );

        match operator {
            hir::BinaryOperator::Equals | hir::BinaryOperator::NotEquals => {
                // For the `==` and `!=` operators, we know the result
                // will be boolean, so even if `result_ty` is an
                // inference variable, we can unify it *now* rather
                // than wait until the input types are known.
                let boolean_type = self.boolean_type();
                self.equate(expression, expression, result_ty, boolean_type);
                boolean_type
            }

            hir::BinaryOperator::Add
            | hir::BinaryOperator::Subtract
            | hir::BinaryOperator::Multiply
            | hir::BinaryOperator::Divide => result_ty,
        }
    }

    /// Invoked to check a binary operator once the base-data for the
    /// left and right types are known.
    fn check_binary_with_both_inputs_known(
        &mut self,
        expression: hir::Expression,
        operator: hir::BinaryOperator,
        left_base_data: BaseData<F>,
        right_base_data: BaseData<F>,
    ) -> Ty<F> {
        let int_type = self.int_type();
        let uint_type = self.uint_type();
        let boolean_type = self.boolean_type();

        match operator {
            hir::BinaryOperator::Add
            | hir::BinaryOperator::Subtract
            | hir::BinaryOperator::Multiply
            | hir::BinaryOperator::Divide => match (&left_base_data.kind, &right_base_data.kind) {
                (BaseKind::Named(entity), BaseKind::Named(right_entity))
                    if entity == right_entity =>
                {
                    match entity.untern(self) {
                        EntityData::LangItem(LangItem::Int) => int_type,
                        EntityData::LangItem(LangItem::Uint) => uint_type,
                        EntityData::Error(_) => self.error_type(),
                        _ => {
                            self.record_error(
                                format!(
                                    "type {:?} does not support this operation",
                                    self.error_type()
                                ),
                                expression,
                            );
                            self.error_type()
                        }
                    }
                }

                (BaseKind::Error, _) | (_, BaseKind::Error) => self.error_type(),

                (BaseKind::Named(_), _) | (BaseKind::Placeholder(_), _) => {
                    self.record_error(
                        format!(
                            "mismatched types ({} vs {})",
                            left_base_data.pretty_print(self.db),
                            right_base_data.pretty_print(self.db)
                        ),
                        expression,
                    );
                    self.error_type()
                }
            },

            hir::BinaryOperator::Equals | hir::BinaryOperator::NotEquals => {
                // Unclear what rule will eventually be... for now, require
                // that the two types are the same?
                if left_base_data != right_base_data {
                    self.record_error(
                        format!(
                            "mismatched types ({} vs {})",
                            left_base_data.pretty_print(self.db),
                            right_base_data.pretty_print(self.db)
                        ),
                        expression,
                    );
                }

                // Either way, yields a boolean
                boolean_type
            }
        }
    }

    fn check_unary(
        &mut self,
        expression: hir::Expression,
        operator: hir::UnaryOperator,
        value: hir::Expression,
    ) -> Ty<F> {
        // We may want to add overloading later. So make sure we know
        // the type of the expression before we determine the type of
        // the output.
        let value_ty = self.check_expression(Synthesize, value);
        self.with_base_data(
            expression,
            expression,
            value_ty.base,
            move |this, value_base_data| {
                this.check_unary_with_input_known(expression, operator, value_base_data)
            },
        )
    }

    fn check_unary_with_input_known(
        &mut self,
        expression: hir::Expression,
        operator: hir::UnaryOperator,
        value_base_data: BaseData<F>,
    ) -> Ty<F> {
        match operator {
            hir::UnaryOperator::Not => match &value_base_data.kind {
                BaseKind::Named(entity) => match entity.untern(self) {
                    EntityData::LangItem(LangItem::Boolean) => self.boolean_type(),

                    EntityData::Error(_) => self.error_type(),

                    _ => {
                        self.record_error("incompatible type for 'not' operator", expression);
                        self.error_type()
                    }
                },

                BaseKind::Error => self.error_type(),

                BaseKind::Placeholder(_) => {
                    self.record_error("unknown expression for operator", expression);
                    self.error_type()
                }
            },
        }
    }
}
