use intern::Intern;
use intern::Untern;
use lark_debug_derive::DebugWith;
use lark_entity::Entity;
use lark_entity::EntityData;
use lark_parser::FileName;
use lark_parser::ParserDatabase;
use lark_test::*;

#[derive(Debug, DebugWith, PartialEq, Eq)]
struct EntityTree {
    name: String,
    children: Vec<EntityTree>,
}

impl EntityTree {
    fn from_file(db: &impl ParserDatabase, file: FileName) -> Self {
        let entity = EntityData::InputFile { file: file.id }.intern(db);
        Self::from_entity(db, entity)
    }

    fn from_entity(db: &impl ParserDatabase, entity: Entity) -> Self {
        EntityTree {
            name: entity.untern(db).relative_name(db),
            children: db
                .child_entities(entity)
                .iter()
                .map(|&e| EntityTree::from_entity(db, e))
                .collect(),
        }
    }
}

#[test]
fn empty_struct() {
    let (file_name, db) = lark_parser_db(unindent::unindent(
        "
        struct Foo {
        }
        ",
    ));

    let tree = EntityTree::from_file(&db, file_name);
    compare_debug(
        &db,
        &unindent::unindent(
            r#"EntityTree {
                name: "InputFile(path1)",
                children: [
                    EntityTree {
                        name: "ItemName(Foo)",
                        children: []
                    }
                ]
            }"#,
        ),
        &tree,
    );
}

#[test]
fn one_field() {
    let (file_name, db) = lark_parser_db(unindent::unindent(
        "
        struct Foo {
            x: uint
        }
        ",
    ));

    let tree = EntityTree::from_file(&db, file_name);
    compare_debug(
        &db,
        &unindent::unindent(
            r#"EntityTree {
                name: "InputFile(path1)",
                children: [
                    EntityTree {
                        name: "ItemName(Foo)",
                        children: [
                            EntityTree {
                                name: "MemberName(x)",
                                children: []
                            }
                        ]
                    }
                ]
            }"#,
        ),
        &tree,
    );
}

#[test]
fn two_fields() {
    let (file_name, db) = lark_parser_db(unindent::unindent(
        "
        struct Foo {
            x: uint,
            y: uint
        }
        ",
    ));

    let tree = EntityTree::from_file(&db, file_name);
    compare_debug(
        &db,
        &unindent::unindent(
            r#"EntityTree {
                name: "InputFile(path1)",
                children: [
                    EntityTree {
                        name: "ItemName(Foo)",
                        children: [
                            EntityTree {
                                name: "MemberName(x)",
                                children: []
                            },
                            EntityTree {
                                name: "MemberName(y)",
                                children: []
                            }
                        ]
                    }
                ]
            }"#,
        ),
        &tree,
    );
}

#[test]
fn two_fields_variations() {
    let tree_base = {
        let (file_name, db) = lark_parser_db(unindent::unindent(
            "
            struct Foo {
                x: uint
                y: uint
            }
            ",
        ));
        EntityTree::from_file(&db, file_name)
    };

    let tree_other = {
        let (file_name, db) = lark_parser_db(unindent::unindent(
            "
            struct Foo {
                x: uint,
                y: uint
            }
            ",
        ));
        EntityTree::from_file(&db, file_name)
    };
    assert_eq!(tree_base, tree_other);

    let tree_other = {
        let (file_name, db) = lark_parser_db(unindent::unindent(
            "
            struct Foo {
                x: uint,
                y: uint,
            }
            ",
        ));
        EntityTree::from_file(&db, file_name)
    };
    assert_eq!(tree_base, tree_other);

    let tree_other = {
        let (file_name, db) = lark_parser_db(unindent::unindent(
            "
            struct Foo {
                x: uint
                y: uint,
            }
            ",
        ));
        EntityTree::from_file(&db, file_name)
    };
    assert_eq!(tree_base, tree_other);

    let tree_other = {
        let (file_name, db) = lark_parser_db(unindent::unindent(
            "
            struct Foo {



                x: uint


                y: uint,

            }


            ",
        ));
        EntityTree::from_file(&db, file_name)
    };
    assert_eq!(tree_base, tree_other);
}
