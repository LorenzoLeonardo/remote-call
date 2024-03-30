use std::env;

use json_elem::JsonElem;
use remote_call::Connector;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        println!("Incomplete parameters! -> remote.exe <object_name> <method> <parameters in JSON format>");
        return;
    }
    let object = args[1].as_str();
    let method = args[2].as_str();

    let param = if args.len() == 3 {
        JsonElem::Null
    } else {
        JsonElem::try_from(args[3].as_bytes()).unwrap()
    };

    let proxy = Connector::connect().await.unwrap();

    let result = proxy.remote_call(object, method, param).await;
    match result {
        Ok(res) => {
            print!("Result: ");
            print_recursive(&res, 0);
        }
        Err(err) => {
            print!("Error: ");
            print_recursive(&err.error, 0);
        }
    }
}

fn print_recursive(value: &JsonElem, indent: usize) {
    match value {
        JsonElem::Null => println!("null"),
        JsonElem::Bool(b) => println!("{}", b),
        JsonElem::Integer(n) => println!("{}", n),
        JsonElem::String(s) => println!("\"{}\"", s),
        JsonElem::Vec(arr) => {
            println!("List");
            for element in arr {
                print!("{}", " ".repeat(indent + 2));
                print_recursive(element, indent + 2);
            }
        }
        JsonElem::Float(f) => println!("{}", f),
        JsonElem::HashMap(obj) => {
            println!("Map");
            for (key, val) in obj {
                print!("{}{}: ", " ".repeat(indent + 2), key);
                print_recursive(val, indent + 2);
            }
        }
    }
}
