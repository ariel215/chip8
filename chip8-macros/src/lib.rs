extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, LitStr};

struct MacroAction<'a>{
    variant: &'a Ident,
    name: LitStr,
    args: Vec<Arg>,
    // mask: String
}

enum Arg{
    Register,
    Immediate(u8)
}

// fn parse_args(desc: &str) -> Option<Vec<Arg>>{
//     let desc = desc.strip_prefix("0x")?;
//     let mut args = vec![];

//     for char in desc.chars(){
//         match char {
//             'X' | 'Y' => {
//                 args.push(Arg::Register);
//             },
//             'N' => {
//                 match args.last() {
//                     None | Some(Arg::Register) => {
//                         args.push(Arg::Immediate(1))
//                     },
//                     Some(Arg::Immediate(v))=>{
//                         args[args.len()-1] = Arg::Immediate(*v + 1)
//                     }
//                 }
//             }
//         }
//     }
//     Some(args)
// }


/// We want to support the following attributes: 
/// #[ mnemonic("XXX")], from which we can generate reading and writing text
/// #[ opcode("0xABCD")], from which we can generate reading and writing binary
/// #[]
#[proc_macro_derive(Chip8Instr, attributes(mnemonic,opcode))]
pub fn derive_chip8_instr(input: TokenStream) -> TokenStream{
    // Parse the input tokens into a syntax tree
    // let input_copy = input.clone();
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;

    let struct_: syn::DataEnum = match ast.data {
        syn::Data::Enum(data) => data,
        _ => panic!("Usage of #[Chip8Instr] on a non-struct type"),
    };
    for variant in struct_.variants.iter() {
        for attr in &variant.attrs {
            if attr.path().is_ident("opcode"){
                dbg!(&variant.ident);
            }
        }
    }
    quote! {{}}.into()

    // let actions = struct_.variants.iter()
    // .filter_map(|p| {
    //     if p.attrs.len() == 0 {
    //         return None
    //     } else {
    //         let mut name: Option<LitStr> = None;
    //         let mut args: Vec<Arg> = vec![];
    //         for attr in p.attrs {
    //             let attr_arg: syn::LitStr = attr.parse_args().ok()?;
    //             if attr.path().is_ident("mnemonic"){
    //                 dbg!(attr_arg)
    //             }
    //         }


    // };

}