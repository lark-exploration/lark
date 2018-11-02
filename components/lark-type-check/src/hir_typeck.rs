use crate::TypeCheckDatabase;
use crate::TypeCheckFamily;
use crate::TypeChecker;
use crate::TypeCheckerFields;
use hir;
use intern::Untern;
use lark_entity::{Entity, EntityData, ItemKind, LangItem, MemberKind};
use lark_error::or_return_sentinel;
use lark_error::ErrorReported;
use lark_ty::declaration::Declaration;
use lark_ty::Signature;
use lark_ty::Ty;
use lark_ty::{BaseData, BaseKind};
use map::FxIndexSet;

impl<DB, F> TypeChecker<'_, DB, F>
where
    DB: TypeCheckDatabase,
    F: TypeCheckFamily,
    Self: AsRef<F::InternTables>,
{
    pub(super) fn check_fn_body(&mut self) {
        let declaration_signature = self
            .db
            .signature(self.fn_entity)
            .into_value()
            .unwrap_or_else(|ErrorReported(_)| {
                <Signature<Declaration>>::error_sentinel(self, self.hir.arguments.len())
            });
        let placeholders = self.placeholders_for(self.fn_entity);
        let signature = self.substitute(
            self.hir.root_expression,
            &placeholders,
            declaration_signature,
        );
        assert_eq!(signature.inputs.len(), self.hir.arguments.len());
        for (argument, &input) in self
            .hir
            .arguments
            .iter(&self.hir)
            .zip(signature.inputs.iter())
        {
            self.results.record_ty(argument, input);
        }
        self.check_expression_has_type(signature.output, self.hir.root_expression);
    }

    fn check_expression_has_type(&mut self, expected_ty: Ty<F>, expression: hir::Expression) {
        let actual_ty = self.check_expression(expression);
        self.require_assignable(expression, actual_ty, expected_ty);
    }

    /// Type-check `expression`, recording and returning the resulting type (which may be
    /// an inference variable).
    fn check_expression(&mut self, expression: hir::Expression) -> Ty<F> {
        let ty = self.compute_expression_ty(expression);
        self.results.record_ty(expression, ty);
        ty
    }

    /// Helper for `check_expression`: compute the type of the given expression.
    fn compute_expression_ty(&mut self, expression: hir::Expression) -> Ty<F> {
        let expression_data = self.hir[expression].clone();
        match expression_data {
            hir::ExpressionData::Let {
                variable,
                initializer: Some(initializer),
                body,
            } => {
                let initializer_ty = self.check_expression(initializer);
                self.results.record_ty(variable, initializer_ty);
                self.check_expression(body)
            }

            hir::ExpressionData::Let {
                variable,
                initializer: None,
                body,
            } => {
                let ty = self.new_infer_ty();
                self.results.record_ty(variable, ty);
                self.check_expression(body)
            }

            hir::ExpressionData::Place { perm, place } => {
                let place_ty = self.check_place(place);
                self.apply_user_perm(perm, place_ty)
            }

            hir::ExpressionData::Assignment { place, value } => {
                let place_ty = self.check_place(place);
                let value_ty = self.check_expression(value);
                self.require_assignable(expression, value_ty, place_ty);
                place_ty
            }

            hir::ExpressionData::MethodCall {
                owner,
                method,
                arguments,
            } => {
                let owner_ty = self.check_place(owner);
                self.compute_method_call_ty(expression, owner_ty, method, arguments)
            }

            hir::ExpressionData::Aggregate { entity, fields } => {
                self.check_aggregate(expression, entity, fields)
            }

            hir::ExpressionData::Sequence { first, second } => {
                let _ = self.check_expression(first);
                self.check_expression(second)
            }

            hir::ExpressionData::If {
                condition,
                if_true,
                if_false,
            } => {
                let condition_ty = self.check_expression(condition);
                self.require_assignable(expression, condition_ty, self.boolean_type());
                let true_ty = self.check_expression(if_true);
                let false_ty = self.check_expression(if_false);
                self.least_upper_bound(expression, true_ty, false_ty)
            }

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
        self.results.record_ty(place, ty);
        ty
    }

    /// Helper for `check_place`.
    fn compute_place_ty(&mut self, place: hir::Place) -> Ty<F> {
        let place_data = self.hir[place];
        match place_data {
            hir::PlaceData::Variable(var) => self.results.ty(var),

            hir::PlaceData::Temporary(expr) => self.check_expression(expr),

            hir::PlaceData::Field { owner, name } => {
                let text = self.hir[name].text;
                let owner_ty = self.check_place(owner);
                self.with_base_data(place, owner_ty.base.into(), move |this, base_data| {
                    let BaseData { kind, generics } = base_data;
                    match kind {
                        BaseKind::Named(def_id) => {
                            match this.db().member_entity(def_id, MemberKind::Field, text) {
                                Some(field_entity) => {
                                    this.results.record_entity(name, field_entity);

                                    let field_decl_ty = this.db().ty(field_entity).into_value();
                                    let field_ty = this.substitute(place, &generics, field_decl_ty);
                                    this.apply_owner_perm(place, owner_ty.perm, field_ty)
                                }

                                None => {
                                    this.record_error(name);
                                    this.error_type()
                                }
                            }
                        }

                        BaseKind::Placeholder(_placeholder) => {
                            // Cannot presently access fields from generic types.
                            this.record_error(name);
                            this.error_type()
                        }

                        BaseKind::Error => this.error_type(),
                    }
                })
            }
        }
    }

    /// Helper for `check_expression`: Compute the type from a method call.
    fn compute_method_call_ty(
        &mut self,
        expression: hir::Expression,
        owner_ty: Ty<F>,
        method_name: hir::Identifier,
        arguments: hir::List<hir::Expression>,
    ) -> Ty<F> {
        self.with_base_data(expression, owner_ty.base.into(), move |this, base_data| {
            this.check_method_call(expression, owner_ty, method_name, arguments, base_data)
        })
    }

    fn check_method_call(
        &mut self,
        expression: hir::Expression,
        _owner_ty: Ty<F>,
        method_name: hir::Identifier,
        arguments: hir::List<hir::Expression>,
        base_data: BaseData<F>,
    ) -> Ty<F> {
        let BaseData { kind, generics } = base_data;
        match kind {
            BaseKind::Named(def_id) => {
                let text = self.hir[method_name].text;
                let method_entity = match self.db().member_entity(def_id, MemberKind::Method, text)
                {
                    Some(def_id) => def_id,
                    None => {
                        self.record_error(expression);
                        return self.error_type();
                    }
                };

                // FIXME -- what role does `owner_ty` place here??

                self.results.record_entity(method_name, method_entity);

                let signature_decl = match self.db().signature(method_entity).into_value() {
                    Ok(s) => s,
                    Err(ErrorReported(_)) => {
                        <Signature<Declaration>>::error_sentinel(self, arguments.len())
                    }
                };
                let signature = self.substitute(expression, &generics, signature_decl);
                if signature.inputs.len() != arguments.len() {
                    self.record_error(method_name);
                }
                let hir = &self.hir.clone();
                for (&expected_ty, argument_expr) in
                    signature.inputs.iter().zip(arguments.iter(hir))
                {
                    self.check_expression_has_type(expected_ty, argument_expr);
                }
                signature.output
            }

            BaseKind::Placeholder(_placeholder) => {
                // Cannot presently invoke methods on generic types.
                let hir = &self.hir.clone();
                for argument_expr in arguments.iter(hir) {
                    self.check_expression(argument_expr);
                }
                self.record_error(method_name);
                self.error_type()
            }

            BaseKind::Error => self.error_type(),
        }
    }

    fn check_aggregate(
        &mut self,
        expression: hir::Expression,
        entity: Entity,
        fields: hir::List<hir::IdentifiedExpression>,
    ) -> Ty<F> {
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
                let error_type = self.error_type();
                let hir = &self.hir.clone();
                for field in fields.iter(hir) {
                    let field_data = self.hir[field];
                    self.results.record_entity(field_data.identifier, entity);
                    self.results.record_ty(field, error_type);
                    self.check_expression_has_type(error_type, field_data.expression);
                }
                return error_type;
            }

            // Something like `def foo() { .. } foo { .. }` is just not legal.
            _ => {
                self.record_error(expression);
                return self.error_type();
            }
        };

        let generics = self.inference_variables_for(entity);

        // Get a vector of **all** the fields.
        let mut missing_members: FxIndexSet<Entity> =
            or_return_sentinel!(&*self, self.db.members(entity))
                .iter()
                .map(|m| m.entity)
                .collect();

        // Find the entity for each of the field names that the user gave us.
        let hir = &self.hir.clone();
        for (field, field_data) in fields.iter_enumerated_data(hir) {
            let field_name = hir[field_data.identifier].text;
            let field_ty = match self.db.member_entity(entity, MemberKind::Field, field_name) {
                Some(field_entity) => {
                    self.results
                        .record_entity(field_data.identifier, field_entity);

                    missing_members.remove(&field_entity);

                    let field_ty = self.db.ty(field_entity).into_value();
                    self.substitute(expression, &generics, field_ty)
                }

                None => {
                    self.record_error(field_data.identifier);
                    self.error_type()
                }
            };

            // Record the formal type of the field on the `IdentifiedExpression`.
            self.results.record_ty(field, field_ty);

            // Check the expression against this formal type.
            self.check_expression_has_type(field_ty, field_data.expression);
        }

        // If we are missing any members, that's an error.
        for _missing_member in missing_members {
            self.record_error(expression);
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
        let left_ty = self.check_expression(left);
        let right_ty = self.check_expression(right);
        let result_ty = self.with_base_data(
            expression,
            left_ty.base.into(),
            move |this, left_base_data| {
                this.with_base_data(
                    expression,
                    right_ty.base.into(),
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
                // One exception are the `==` and `!=` operators. They
                // always yield boolean.
                let boolean_type = self.boolean_type();
                if result_ty != boolean_type {
                    self.require_assignable(expression, result_ty, boolean_type);
                }
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
                            self.record_error(expression);
                            self.error_type()
                        }
                    }
                }

                (BaseKind::Error, _) | (_, BaseKind::Error) => self.error_type(),

                (BaseKind::Named(_), _) | (BaseKind::Placeholder(_), _) => {
                    self.record_error(expression);
                    self.error_type()
                }
            },

            hir::BinaryOperator::Equals | hir::BinaryOperator::NotEquals => {
                // Unclear what rule will eventually be... for now, require
                // that the two types are the same?
                if left_base_data != right_base_data {
                    self.record_error(expression);
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
        let value_ty = self.check_expression(value);
        self.with_base_data(
            expression,
            value_ty.base.into(),
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
                        self.record_error(expression);
                        self.error_type()
                    }
                },

                BaseKind::Error => self.error_type(),

                BaseKind::Placeholder(_) => {
                    self.record_error(expression);
                    self.error_type()
                }
            },
        }
    }
}
