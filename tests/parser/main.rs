use intern::Intern;
use lark_debug_derive::DebugWith;
use lark_entity::Entity;
use lark_entity::EntityData;
use lark_parser::FileName;
use lark_parser::ParserDatabase;
use lark_test::*;

#[derive(Debug, DebugWith)]
struct EntityTree {
    entity: Entity,
    children: Vec<EntityTree>,
}

impl EntityTree {
    fn from_file(db: &impl ParserDatabase, file: FileName) -> Self {
        let entity = EntityData::InputFile { file: file.id }.intern(db);
        Self::from_entity(db, entity)
    }

    fn from_entity(db: &impl ParserDatabase, entity: Entity) -> Self {
        EntityTree {
            entity,
            children: db
                .child_entities(entity)
                .iter()
                .map(|&e| EntityTree::from_entity(db, e))
                .collect(),
        }
    }
}

#[test]
fn basic() {
    pretty_env_logger::init();

    let (file_name, db) = lark_parser_db(unindent::unindent(
        "
        struct Foo {
        }
        ",
    ));

    let tree = EntityTree::from_file(&db, file_name);
    compare_debug(&db, "", &tree);
}
