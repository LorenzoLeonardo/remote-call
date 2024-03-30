use std::env;

use json_elem::JsonElem;
use remote_call::logger::ENV_LOGGER;
use remote_call::Connector;

pub fn setup_logger() {
    let level = std::env::var(ENV_LOGGER)
        .map(|var| match var.to_lowercase().as_str() {
            "trace" => log::LevelFilter::Trace,
            "debug" => log::LevelFilter::Debug,
            "info" => log::LevelFilter::Info,
            "warn" => log::LevelFilter::Warn,
            "error" => log::LevelFilter::Error,
            "off" => log::LevelFilter::Off,
            _ => log::LevelFilter::Info,
        })
        .unwrap_or_else(|_| log::LevelFilter::Info);

    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}][{:<5}][{}:{:<3}]: {}",
                chrono::Local::now().format("%H:%M:%S%.9f"),
                record.level(),
                record.file().unwrap_or_default(),
                record.line().unwrap_or_default(),
                message
            ))
        })
        .level(level)
        .chain(std::io::stdout())
        .apply()
        .unwrap_or_else(|e| {
            panic!("{:?}", e);
        });
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    setup_logger();

    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        log::error!("Incomplete parameters! -> remote.exe <object_name> <method> <parameters in JSON format>");
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
        Ok(res) => log::info!("Success: {}", res),
        Err(err) => log::error!("Error: {}", err),
    }
}
