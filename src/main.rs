mod codegen;
mod ir;

use codegen::{codegen, RustFile};
use ir::{builtin_type, Command, Context, Definition, Function};

fn main() {
    let mut bob = Function::new("bob".into(), builtin_type::I32)
        .param("x".into(), builtin_type::I32)
        .param("y".into(), builtin_type::I32);

    bob.body.push(Command::VarUse(0));
    bob.body.push(Command::VarUse(1));
    bob.body.push(Command::Sub);
    bob.body.push(Command::VarDeclWithInit(1));
    bob.body.push(Command::VarUse(1));
    bob.body.push(Command::ReturnLastStackValue);

    let mut c = Context::new();
    c.definitions.push(Definition::Fn(bob));

    let bob_def_id = c.definitions.len() - 1;

    let mut m = Function::new("main".into(), builtin_type::VOID);
    m.body.push(Command::ConstInt(11));
    m.body.push(Command::ConstInt(8));
    m.body.push(Command::Call(bob_def_id, 2));
    m.body.push(Command::ConstString("Hello, world {}".into()));
    m.body.push(Command::Call(101, 1));
    m.body.push(Command::DebugPrint);
    c.definitions.push(Definition::Fn(m));

    let mut rust = RustFile::new();

    codegen(&mut rust, &c);

    println!("{}", rust.render());
}
