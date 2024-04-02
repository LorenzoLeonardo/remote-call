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
            res.print(0);
        }
        Err(err) => {
            println!("Exception Error: {}", err);
        }
    }
}
