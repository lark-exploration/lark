use crate::full_inference::FullInferenceTables;
use crate::TypeCheckDatabase;
use lark_debug_with::DebugWith;
use lark_entity::Entity;
use lark_entity::EntityTables;
use lark_hir as hir;
use lark_string::GlobalIdentifierTables;
use std::fs;
use std::fs::File;
use std::io;
use std::io::Write;
use std::path::PathBuf;

crate struct DumpCx<'me, DB>
where
    DB: TypeCheckDatabase,
{
    db: &'me DB,
    fn_body: &'me hir::FnBody,
    tables: &'me FullInferenceTables,
    entity: Entity,
    base_dir: Option<PathBuf>,
}

impl<DB> DumpCx<'me, DB>
where
    DB: TypeCheckDatabase,
{
    crate fn new(
        db: &'me DB,
        fn_body: &'me hir::FnBody,
        tables: &'me FullInferenceTables,
        entity: Entity,
    ) -> Self {
        let base_dir = std::env::var("LARK_DUMP").ok().map(PathBuf::from);
        Self {
            db,
            fn_body,
            tables,
            entity,
            base_dir,
        }
    }

    crate fn dump_enabled(&self) -> bool {
        self.base_dir.is_some()
    }

    crate fn dump_facts<T>(&self, name: &str, facts: impl IntoIterator<Item = T>) -> io::Result<()>
    where
        T: DebugWith,
    {
        if let Some(base_dir) = &self.base_dir {
            let mut dir = PathBuf::from(base_dir);
            dir.push(self.entity.dump_dir(self));
            fs::create_dir_all(&dir)?;

            dir.push(name);

            let mut file = File::create(&dir)?;
            for fact in facts {
                write!(file, "{:?}\n", fact.debug_with(self))?;
            }
        }

        Ok(())
    }
}

impl<DB> AsRef<EntityTables> for DumpCx<'_, DB>
where
    DB: TypeCheckDatabase,
{
    fn as_ref(&self) -> &EntityTables {
        self.db.as_ref()
    }
}

impl<DB> AsRef<GlobalIdentifierTables> for DumpCx<'_, DB>
where
    DB: TypeCheckDatabase,
{
    fn as_ref(&self) -> &GlobalIdentifierTables {
        self.db.as_ref()
    }
}

impl<DB> AsRef<hir::FnBodyTables> for DumpCx<'_, DB>
where
    DB: TypeCheckDatabase,
{
    fn as_ref(&self) -> &hir::FnBodyTables {
        self.fn_body.as_ref()
    }
}

impl<DB> AsRef<FullInferenceTables> for DumpCx<'_, DB>
where
    DB: TypeCheckDatabase,
{
    fn as_ref(&self) -> &FullInferenceTables {
        self.tables
    }
}
