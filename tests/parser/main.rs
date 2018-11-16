use debug::DebugWith;
use intern::Intern;
use intern::Untern;
use lark_debug_derive::DebugWith;
use lark_entity::Entity;
use lark_entity::EntityData;
use lark_hir as hir;
use lark_parser::FileName;
use lark_parser::ParserDatabase;
use lark_query_system::LarkDatabase;
use lark_string::GlobalIdentifierTables;
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

fn select_entity(db: &impl ParserDatabase, file: FileName, index: usize) -> Entity {
    let file_entity = EntityData::InputFile { file: file.id }.intern(db);
    db.child_entities(file_entity)[index]
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
    assert_expected_debug(
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
    assert_expected_debug(
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
    assert_expected_debug(
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
fn one_struct_newline_variations() {
    let tree_base = {
        let (file_name, db) = lark_parser_db(unindent::unindent(
            "
            struct Foo {
                x: uint
            }
            ",
        ));
        EntityTree::from_file(&db, file_name)
    };

    let tree_other = {
        let (file_name, db) = lark_parser_db(unindent::unindent(
            "
            struct
            Foo {
                x: uint
            }
            ",
        ));
        EntityTree::from_file(&db, file_name)
    };
    assert_equal(&(), &tree_base, &tree_other);

    let tree_other = {
        let (file_name, db) = lark_parser_db(unindent::unindent(
            "
            struct
            Foo
            {

                x: uint


            }
            ",
        ));
        EntityTree::from_file(&db, file_name)
    };
    assert_equal(&(), &tree_base, &tree_other);

    let tree_other = {
        let (file_name, db) = lark_parser_db(unindent::unindent(
            "
            struct
            Foo
            {

                x
                :
                uint


            }
            ",
        ));
        EntityTree::from_file(&db, file_name)
    };
    assert_equal(&(), &tree_base, &tree_other);
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
    assert_equal(&(), &tree_base, &tree_other);

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
    assert_equal(&(), &tree_base, &tree_other);

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
    assert_equal(&(), &tree_base, &tree_other);

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
    assert_equal(&(), &tree_base, &tree_other);
}

#[test]
fn two_structs_overlapping_lines() {
    let (file_name, db) = lark_parser_db(unindent::unindent(
        "
        struct Foo {
        } struct Bar {
        }
        ",
    ));

    let tree = EntityTree::from_file(&db, file_name);
    assert_expected_debug(
        &db,
        &unindent::unindent(
            r#"EntityTree {
                name: "InputFile(path1)",
                children: [
                    EntityTree {
                        name: "ItemName(Foo)",
                        children: []
                    },
                    EntityTree {
                        name: "ItemName(Bar)",
                        children: []
                    }
                ]
            }"#,
        ),
        &tree,
    );
}

#[test]
fn two_structs_whitespace() {
    let base_tree = {
        let (file_name, db) = lark_parser_db(unindent::unindent(
            "
            struct Foo {
            } struct Bar {
            }
            ",
        ));
        EntityTree::from_file(&db, file_name)
    };

    let other_tree = {
        let (file_name, db) = lark_parser_db(unindent::unindent(
            "
            struct Foo {
            }
            struct Bar {
            }
            ",
        ));
        EntityTree::from_file(&db, file_name)
    };
    assert_equal(&(), &base_tree, &other_tree);

    let other_tree = {
        let (file_name, db) = lark_parser_db(unindent::unindent(
            "
            struct Foo {
            }

            struct Bar {
            }
            ",
        ));
        EntityTree::from_file(&db, file_name)
    };
    assert_equal(&(), &base_tree, &other_tree);
}

#[test]
fn eof_extra_sigil() {
    let (file_name, db) = lark_parser_db(unindent::unindent(
        "
            struct Foo {
                x: uint
            }

            +
            ",
    ));

    // These errors are (a) too numerous and (b) poor quality :(

    let entity = EntityData::InputFile { file: file_name.id }.intern(&db);
    assert_expected_debug(
        &db,
        &unindent::unindent(
            r#"
            [
                Diagnostic {
                    span: synthetic,
                    label: "unexpected character"
                },
                Diagnostic {
                    span: synthetic,
                    label: "unexpected character"
                },
                Diagnostic {
                    span: synthetic,
                    label: "unexpected character"
                },
                Diagnostic {
                    span: synthetic,
                    label: "unexpected character"
                }
            ]"#,
        ),
        &db.child_parsed_entities(entity).errors,
    );
}

#[test]
fn some_function() {
    let (file_name, db) = lark_parser_db(unindent::unindent(
        "
        fn foo() {
        }
        ",
    ));

    let tree = EntityTree::from_file(&db, file_name);
    assert_expected_debug(
        &db,
        &unindent::unindent(
            r#"EntityTree {
                name: "InputFile(path1)",
                children: [
                    EntityTree {
                        name: "ItemName(foo)",
                        children: []
                    }
                ]
            }"#,
        ),
        &tree,
    );
}

#[test]
fn function_variations() {
    let base_tree = {
        let (file_name, db) = lark_parser_db(unindent::unindent(
            "
            fn foo() { }
            ",
        ));
        EntityTree::from_file(&db, file_name)
    };

    let other_tree = {
        let (file_name, db) = lark_parser_db(unindent::unindent(
            "
            fn foo(x: uint) { }
            ",
        ));
        EntityTree::from_file(&db, file_name)
    };
    assert_equal(&(), &base_tree, &other_tree);

    let other_tree = {
        let (file_name, db) = lark_parser_db(unindent::unindent(
            "
            fn foo(
                x: uint,
            ) -> uint {
            }
            ",
        ));
        EntityTree::from_file(&db, file_name)
    };
    assert_equal(&(), &base_tree, &other_tree);
}

pub struct FnBodyContext<'me> {
    db: &'me LarkDatabase,
    fn_body: &'me hir::FnBody,
}

impl AsRef<GlobalIdentifierTables> for FnBodyContext<'_> {
    fn as_ref(&self) -> &GlobalIdentifierTables {
        self.db.as_ref()
    }
}

impl AsRef<hir::FnBodyTables> for FnBodyContext<'_> {
    fn as_ref(&self) -> &hir::FnBodyTables {
        self.fn_body.as_ref()
    }
}

#[test]
fn parse_binary_expressions_precedence() {
    let (file_name, db) = lark_parser_db(unindent::unindent(
        "
            fn foo() {
              let bar = 22
              let baz = 44
              bar + baz * baz + bar
            }
        ",
    ));

    let foo = select_entity(&db, file_name, 0);
    let fn_body = db.fn_body2(foo).assert_no_errors();
    assert_expected_debug(
        &FnBodyContext {
            db: &db,
            fn_body: &fn_body,
        },
        &unindent::unindent(
            r#"FnBody {
    arguments: [],
    root_expression: Let {
        variable: VariableData {
            name: IdentifierData {
                text: "bar"
            }
        },
        initializer: Literal {
            data: LiteralData {
                kind: UnsignedInteger,
                value: "22"
            }
        },
        body: Let {
            variable: VariableData {
                name: IdentifierData {
                    text: "baz"
                }
            },
            initializer: Literal {
                data: LiteralData {
                    kind: UnsignedInteger,
                    value: "44"
                }
            },
            body: Binary {
                operator: Add,
                left: Binary {
                    operator: Add,
                    left: Place {
                        perm: Default,
                        place: Variable(
                            VariableData {
                                name: IdentifierData {
                                    text: "bar"
                                }
                            }
                        )
                    },
                    right: Binary {
                        operator: Multiply,
                        left: Place {
                            perm: Default,
                            place: Variable(
                                VariableData {
                                    name: IdentifierData {
                                        text: "baz"
                                    }
                                }
                            )
                        },
                        right: Place {
                            perm: Default,
                            place: Variable(
                                VariableData {
                                    name: IdentifierData {
                                        text: "baz"
                                    }
                                }
                            )
                        }
                    }
                },
                right: Place {
                    perm: Default,
                    place: Variable(
                        VariableData {
                            name: IdentifierData {
                                text: "bar"
                            }
                        }
                    )
                }
            }
        }
    }
}"#,
        ),
        &fn_body,
    );
}

#[test]
fn parse_fn_body_variations() {
    let debug1 = {
        let (file_name, db) = lark_parser_db(unindent::unindent(
            "
            fn foo() {
              let bar = 22
              let baz = 44
              bar + baz * baz + bar
            }
        ",
        ));
        let fn_body = db
            .fn_body2(select_entity(&db, file_name, 0))
            .assert_no_errors();
        fn_body
            .debug_with(&FnBodyContext {
                db: &db,
                fn_body: &fn_body,
            })
            .to_string()
    };

    let debug2 = {
        let (file_name, db) = lark_parser_db(unindent::unindent(
            "
            fn foo() {
              let bar =
              22
              let baz =
              44
              bar +
              baz *
              baz +
              bar
            }
        ",
        ));
        let fn_body = db
            .fn_body2(select_entity(&db, file_name, 0))
            .assert_no_errors();
        fn_body
            .debug_with(&FnBodyContext {
                db: &db,
                fn_body: &fn_body,
            })
            .to_string()
    };

    assert_equal(&(), &debug1, &debug2);
}

#[test]
fn parse_binary_expressions_chained_comparison() {
    let (file_name, db) = lark_parser_db(unindent::unindent(
        "
            fn foo() {
              let bar = 22
              let baz = 44
              bar == baz == bar
            }
        ",
    ));

    let foo = select_entity(&db, file_name, 0);
    let fn_body = db.fn_body2(foo);
    assert_eq!(fn_body.errors.len(), 2);
}

#[test]
fn parse_binary_expressions_comparison() {
    let (file_name, db) = lark_parser_db(unindent::unindent(
        "
            fn foo() {
              let bar = 22
              let baz = 44
              bar == bar + baz
            }
        ",
    ));

    let foo = select_entity(&db, file_name, 0);
    db.fn_body2(foo).assert_no_errors();
}

#[test]
fn parse_methods_chained_variations() {
    let debug1 = {
        let (file_name, db) = lark_parser_db(unindent::unindent(
            "
            fn foo() {
              let bar = 22
              let baz = 44
              bar.method1().method2()
            }
        ",
        ));
        let fn_body = db
            .fn_body2(select_entity(&db, file_name, 0))
            .assert_no_errors();
        fn_body
            .debug_with(&FnBodyContext {
                db: &db,
                fn_body: &fn_body,
            })
            .to_string()
    };

    let debug2 = {
        let (file_name, db) = lark_parser_db(unindent::unindent(
            "
            fn foo() {
              let bar = 22
              let baz = 44
              bar
                .method1()
                .method2()
            }
        ",
        ));
        let fn_body = db
            .fn_body2(select_entity(&db, file_name, 0))
            .assert_no_errors();
        fn_body
            .debug_with(&FnBodyContext {
                db: &db,
                fn_body: &fn_body,
            })
            .to_string()
    };

    assert_equal(&(), &debug1, &debug2);
}
