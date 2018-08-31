mod codegen;
mod ir;

use codegen::{codegen, RustFile};
use ir::{builtin_type, Command, Context, Function};

fn main() {
    let mut f = Function::new("bob".into(), builtin_type::I32).param("x".into(), builtin_type::I32);

    f.body.push(Command::VarUse(0));
    f.body.push(Command::VarUse(0));
    f.body.push(Command::Add);
    f.body.push(Command::ReturnLastStackValue);

    let c = Context::new();
    let mut rust = RustFile::new();

    codegen(&mut rust, &c, &f);

    println!("{}", rust.render());
}
