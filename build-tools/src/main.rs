use std::io::Write;
use std::{env, fs};

use object::{Object, ObjectSymbol, SymbolKind};
use rustc_demangle::demangle;

struct Symbol<'a> {
    address: u64,
    size: u64,
    name: &'a str,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    assert!(args.len() >= 2, "no input file");
    assert!(args.len() >= 3, "no output file");
    let bin = fs::read(&args[1]).expect("could not read binary");
    println!("{}", &args[1]);
    let file = object::File::parse(&*bin).expect("could not parse binary");

    let mut output = fs::File::create(&args[2]).expect("could not create output file");
    writeln!(output, ".section .rodata").expect("could not write output file");
    writeln!(output, ".global _sksyms").expect("could not write output file");
    writeln!(output, ".global _eksyms").expect("could not write output file");
    writeln!(output, "").expect("could not write output file");

    let mut symbols: Vec<Symbol> = vec![];

    for symbol in file.symbols() {
        if symbol.kind() == SymbolKind::Text
            && let Ok(name) = symbol.name()
        {
            symbols.push(Symbol {
                address: symbol.address(),
                size: symbol.size(),
                name,
            });
        }
    }

    symbols.sort_by(|a, b| a.address.cmp(&b.address));

    for (i, symbol) in (&symbols).iter().enumerate() {
        writeln!(output, ".Lksym{}: .asciz \"{}\"", i, demangle(symbol.name))
            .expect("could not write output file");
    }

    writeln!(output, "").expect("could not write output file");
    writeln!(output, ".align 0x8").expect("could not write output file");
    writeln!(output, "_sksyms:").expect("could not write output file");

    for (i, symbol) in (&symbols).iter().enumerate() {
        writeln!(output, ".quad {:#x}", symbol.address).expect("could not write output file");
        writeln!(output, ".quad {:#x}", symbol.size).expect("could not write output file");
        writeln!(output, ".quad .Lksym{}", i).expect("could not write output file");
    }

    writeln!(output, "_eksyms:").expect("could not write output file");
}
