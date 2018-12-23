use crate::harness::options::HirMode;
use crate::harness::test::TestContext;
use lark_debug_with::DebugWith;
use lark_entity::EntityData;
use lark_entity::EntityTables;
use lark_entity::ItemKind;
use lark_entity::MemberKind;
use lark_hir as hir;
use lark_intern::Intern;
use lark_intern::Untern;
use lark_parser::ParserDatabase;
use lark_query_system::LarkDatabase;
use lark_string::GlobalIdentifierTables;
use lark_ty::base_inferred::BaseInferredTables;
use lark_ty::full_inferred::FullInferredTables;
use lark_ty::TypeFamily;
use lark_type_check::TypeCheckDatabase;
use lark_type_check::TypeCheckResults;
use std::fmt::Write;

impl TestContext<'_> {
    crate fn compare_hir_output(&self) {
        let mode = match self.options.hir_mode {
            Some(mode) => mode,

            None => {
                self.compare_reference_contents("hir", b"", false);
                return;
            }
        };

        let input_files = self.db.file_names();

        let mut text = String::new();

        for &input_file in &*input_files {
            let file_entity = EntityData::InputFile { file: input_file }.intern(&self.db);
            for &entity in self.db.descendant_entities(file_entity).iter() {
                let has_hir = match entity.untern(&self.db) {
                    EntityData::ItemName {
                        kind: ItemKind::Function,
                        ..
                    }
                    | EntityData::MemberName {
                        kind: MemberKind::Method,
                        ..
                    } => true,
                    _ => false,
                };

                if !has_hir {
                    continue;
                }

                let db = &self.db;
                let fn_body = &self.db.fn_body(entity).into_value();

                writeln!(text, "{:?}", entity.debug_with(&self.db)).unwrap();

                match mode {
                    HirMode::Base => {
                        let results = &self.db.base_type_check(entity).into_value();
                        let info = BaseInfo {
                            db,
                            fn_body,
                            results,
                        };
                        writeln!(text, "{:#?}", &fn_body.debug_with(&info)).unwrap();
                    }
                    HirMode::Full => {
                        let results = &self.db.full_type_check(entity).into_value();
                        let info = BaseInfo {
                            db,
                            fn_body,
                            results,
                        };
                        writeln!(text, "{:#?}", &fn_body.debug_with(&info)).unwrap();
                    }
                }
            }
        }

        self.compare_reference_contents("hir", text.as_bytes(), false);
    }
}

struct BaseInfo<'me, F>
where
    F: TypeFamily,
{
    db: &'me LarkDatabase,
    fn_body: &'me hir::FnBody,
    results: &'me TypeCheckResults<F>,
}

impl<F> AsRef<hir::FnBodyTables> for BaseInfo<'_, F>
where
    F: TypeFamily,
{
    fn as_ref(&self) -> &hir::FnBodyTables {
        self.fn_body.as_ref()
    }
}

impl<F> AsRef<EntityTables> for BaseInfo<'_, F>
where
    F: TypeFamily,
{
    fn as_ref(&self) -> &EntityTables {
        self.db.as_ref()
    }
}

impl<F> AsRef<GlobalIdentifierTables> for BaseInfo<'_, F>
where
    F: TypeFamily,
{
    fn as_ref(&self) -> &GlobalIdentifierTables {
        self.db.as_ref()
    }
}

impl<F> AsRef<BaseInferredTables> for BaseInfo<'_, F>
where
    F: TypeFamily,
{
    fn as_ref(&self) -> &BaseInferredTables {
        self.db.as_ref()
    }
}

impl<F> AsRef<FullInferredTables> for BaseInfo<'_, F>
where
    F: TypeFamily,
{
    fn as_ref(&self) -> &FullInferredTables {
        self.db.as_ref()
    }
}

impl<F> hir::FnBodyExtraDebug for BaseInfo<'_, F>
where
    F: TypeFamily,
{
    fn extended_debug_with(
        &self,
        index: hir::MetaIndex,
        debug: &mut std::fmt::DebugStruct<'_, '_>,
    ) -> std::fmt::Result {
        if let Some(v) = self.results.max_types.get(&index) {
            debug.field("max_types", &v.debug_with(self.db));
        }

        if let Some(v) = self.results.generics.get(&index) {
            debug.field("generics", &v.debug_with(self.db));
        }

        if let Some(v) = self.results.entities.get(&index) {
            debug.field("entity", &v.debug_with(self.db));
        }

        if let hir::MetaIndex::Expression(e) = index {
            if let Some(v) = self.results.access_types.get(&e) {
                debug.field("access_types", &v.debug_with(self.db));
            }

            if let Some(v) = self.results.access_permissions.get(&e) {
                debug.field("access_permissions", &v.debug_with(self.db));
            }
        }

        Ok(())
    }
}
